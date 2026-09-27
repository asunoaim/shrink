//! The bridge between the screen and the engine. Commands stay thin: all logic
//! lives in `engine`, which is tested on its own.

use crate::clipboard;
use crate::engine::encoder::{self, Encoder};
use crate::engine::plan::{self, ExportJob, ExportRequest};
use crate::engine::preview;
use crate::engine::probe::{self, ClipInfo};
use crate::engine::quality::{self, Format, SizeAdvice};
use crate::engine::runner::{self, CancelFlag, ClipOutcome};
use crate::engine::tools::Tools;
use crate::engine::{EngineError, Result};
use crate::settings::{self, Settings};
use serde::Serialize;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

const THUMB_COUNT: u32 = 24;
const THUMB_HEIGHT: u32 = 72;

#[derive(Default)]
pub struct AppState {
    tools: Mutex<Option<Tools>>,
    encoder: Mutex<Option<Encoder>>,
    clip: Mutex<Option<ClipInfo>>,
    cancel: Mutex<Option<CancelFlag>>,
    /// One export at a time, so two runs can't race for the same file names.
    exporting: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl AppState {
    fn tools(&self) -> Result<Tools> {
        let mut t = self.tools.lock().unwrap();
        if t.is_none() {
            *t = Some(Tools::locate()?);
        }
        Ok(t.clone().unwrap())
    }

    fn encoder(&self, tools: &Tools) -> Encoder {
        *self.encoder.lock().unwrap().get_or_insert_with(|| encoder::detect(tools))
    }

    fn clip(&self) -> Result<ClipInfo> {
        self.clip
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| EngineError::InvalidRequest("no clip is open".into()))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipView {
    pub info: ClipInfo,
    pub thumbs: Option<PathBuf>,
    pub thumb_count: u32,
    pub encoder: Encoder,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportDone {
    outcomes: Vec<ClipOutcome>,
    copied_to_clipboard: bool,
}

#[tauri::command]
pub async fn open_clip(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<ClipView> {
    let tools = state.tools()?;
    let info = probe::probe(&tools, Path::new(&path))?;
    allow(&app, &info.path)?;
    let encoder = state.encoder(&tools);

    let thumbs = cache_file(&app, "thumbs", &info.path, "png").ok().filter(|png| {
        png.is_file() || preview::thumbnail_strip(&tools, &info, png, THUMB_COUNT, THUMB_HEIGHT).is_ok()
    });
    if let Some(t) = &thumbs {
        allow(&app, t)?;
    }
    *state.clip.lock().unwrap() = Some(info.clone());
    Ok(ClipView { info, thumbs, thumb_count: THUMB_COUNT, encoder })
}

#[tauri::command]
pub fn size_advice(state: State<'_, AppState>, longest: f64, target_bytes: u64, has_audio: bool) -> Result<SizeAdvice> {
    let c = state.clip()?;
    let src = Format { width: c.width, height: c.height, fps: c.fps };
    Ok(quality::advice(c.size_bytes, c.duration, longest, target_bytes, has_audio, &src))
}

/// Plans the export and starts it in the background. Progress arrives as
/// `export-event`, the end as `export-done`.
#[tauri::command]
pub fn start_export(app: AppHandle, state: State<'_, AppState>, request: ExportRequest) -> Result<Vec<ExportJob>> {
    use std::sync::atomic::Ordering;
    if state.exporting.swap(true, Ordering::SeqCst) {
        return Err(EngineError::InvalidRequest("an export is already running".into()));
    }
    let planned = (|| -> Result<(Vec<ExportJob>, Tools)> {
        let tools = state.tools()?;
        let info = state.clip()?;
        Ok((plan::plan(&info, &request, state.encoder(&tools), &|p| p.exists())?, tools))
    })();
    let (jobs, tools) = match planned {
        Ok(v) => v,
        Err(e) => {
            state.exporting.store(false, Ordering::SeqCst);
            return Err(e);
        }
    };
    let exporting = state.exporting.clone();
    let cancel = CancelFlag::new();
    *state.cancel.lock().unwrap() = Some(cancel.clone());

    let thread_jobs = jobs.clone();
    std::thread::spawn(move || {
        let outcomes = runner::run_all(&tools, &thread_jobs, &cancel, &mut |e| {
            let _ = app.emit("export-event", e);
        });
        let done: Vec<PathBuf> = outcomes
            .iter()
            .filter_map(|o| match o {
                ClipOutcome::Done { output, .. } => Some(output.clone()),
                _ => None,
            })
            .collect();
        let copied_to_clipboard = !done.is_empty() && clipboard::copy_files(&done).is_ok();
        exporting.store(false, std::sync::atomic::Ordering::SeqCst);
        let _ = app.emit("export-done", ExportDone { outcomes, copied_to_clipboard });
    });
    Ok(jobs)
}

#[tauri::command]
pub fn cancel_export(state: State<'_, AppState>) {
    if let Some(c) = state.cancel.lock().unwrap().as_ref() {
        c.cancel();
    }
}

/// A playable copy for clips the built-in player can't decode. Progress arrives
/// as `proxy-progress` (0..1). Reused when it already exists.
#[tauri::command]
pub async fn make_proxy(app: AppHandle, state: State<'_, AppState>) -> Result<PathBuf> {
    let tools = state.tools()?;
    let info = state.clip()?;
    let out = cache_file(&app, "proxies", &info.path, "mp4")?;
    if !out.is_file() {
        let job = preview::proxy_job(&info, &out, state.encoder(&tools));
        runner::run_job(&tools, &job, &CancelFlag::new(), &mut |f| {
            let _ = app.emit("proxy-progress", f);
        })?;
    }
    allow(&app, &out)?;
    Ok(out)
}

/// The file passed on the command line ("Open in shrink").
#[tauri::command]
pub fn initial_file() -> Option<String> {
    std::env::args().skip(1).find(|a| !a.starts_with('-'))
}

#[tauri::command]
pub fn copy_to_clipboard(paths: Vec<PathBuf>) -> std::result::Result<(), String> {
    clipboard::copy_files(&paths)
}

fn settings_path(app: &AppHandle) -> std::result::Result<PathBuf, String> {
    Ok(app.path().app_config_dir().map_err(|e| e.to_string())?.join("settings.json"))
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    settings_path(&app).map(|p| settings::load(&p)).unwrap_or_default()
}

#[tauri::command]
pub fn set_settings(app: AppHandle, settings: Settings) -> std::result::Result<(), String> {
    settings::save(&settings_path(&app)?, &settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn folder_exists(path: String) -> bool {
    Path::new(&path).is_dir()
}

fn allow(app: &AppHandle, path: &Path) -> Result<()> {
    app.asset_protocol_scope()
        .allow_file(path)
        .map_err(|e| EngineError::InvalidRequest(e.to_string()))
}

/// `<app cache>/<kind>/<hash of path, size, mtime>.<ext>`
fn cache_file(app: &AppHandle, kind: &str, source: &Path, ext: &str) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| EngineError::InvalidRequest(e.to_string()))?
        .join(kind);
    std::fs::create_dir_all(&dir)?;
    let meta = std::fs::metadata(source)?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut h);
    meta.len().hash(&mut h);
    meta.modified().ok().hash(&mut h);
    Ok(dir.join(format!("{:016x}.{ext}", h.finish())))
}
