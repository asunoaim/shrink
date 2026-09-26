//! Clip reader: what's inside a video file, via ffprobe. Never decodes the video.

use super::tools::{Tool, Tools};
use super::{EngineError, Result};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioTrack {
    /// Position among the audio streams (ffmpeg's `0:a:<index>`).
    pub index: usize,
    pub channels: u32,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipInfo {
    pub path: PathBuf,
    pub format_name: String,
    /// Lowercase file extension, reused for lossless exports.
    pub container_ext: String,
    pub duration: f64,
    pub size_bytes: u64,
    /// Overall bits per second.
    pub bitrate: u64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub video_codec: String,
    pub audio_tracks: Vec<AudioTrack>,
    /// Keyframe times in seconds from the start of the clip, ascending.
    pub keyframes: Vec<f64>,
}

pub fn probe(tools: &Tools, path: &Path) -> Result<ClipInfo> {
    let json = run_ffprobe(
        tools,
        &["-v", "error", "-print_format", "json", "-show_format", "-show_streams"],
        path,
    )?;
    let v: Value = serde_json::from_str(&json).map_err(|e| EngineError::Probe(e.to_string()))?;
    let streams = v["streams"].as_array().cloned().unwrap_or_default();
    let format = &v["format"];

    let video = streams
        .iter()
        .find(|s| s["codec_type"] == "video" && s["disposition"]["attached_pic"] != 1)
        .ok_or_else(|| EngineError::Probe("no video stream".into()))?;

    let duration = num(&format["duration"])
        .or_else(|| num(&video["duration"]))
        .filter(|d| *d > 0.0)
        .ok_or_else(|| EngineError::Probe("unknown duration".into()))?;
    let start_time = num(&format["start_time"]).unwrap_or(0.0);
    let size_bytes = std::fs::metadata(path)?.len();
    let bitrate = num(&format["bit_rate"])
        .map(|b| b as u64)
        .unwrap_or_else(|| (size_bytes as f64 * 8.0 / duration) as u64);

    let fps = rate(&video["avg_frame_rate"])
        .or_else(|| rate(&video["r_frame_rate"]))
        .ok_or_else(|| EngineError::Probe("unknown frame rate".into()))?;

    let audio_tracks = streams
        .iter()
        .filter(|s| s["codec_type"] == "audio")
        .enumerate()
        .map(|(index, s)| AudioTrack {
            index,
            channels: s["channels"].as_u64().unwrap_or(2) as u32,
            title: track_title(&s["tags"]),
        })
        .collect();

    Ok(ClipInfo {
        path: path.to_path_buf(),
        format_name: format["format_name"].as_str().unwrap_or_default().to_string(),
        container_ext: path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_else(|| "mp4".into()),
        duration,
        size_bytes,
        bitrate,
        width: video["width"].as_u64().unwrap_or(0) as u32,
        height: video["height"].as_u64().unwrap_or(0) as u32,
        fps,
        video_codec: video["codec_name"].as_str().unwrap_or_default().to_string(),
        audio_tracks,
        keyframes: keyframes(tools, path, start_time)?,
    })
}

/// Keyframe times from packet flags (fast even for hour-long recordings).
fn keyframes(tools: &Tools, path: &Path, start_time: f64) -> Result<Vec<f64>> {
    let csv = run_ffprobe(
        tools,
        &["-v", "error", "-select_streams", "v:0", "-show_entries", "packet=pts_time,flags", "-of", "csv=p=0"],
        path,
    )?;
    let mut out: Vec<f64> = csv
        .lines()
        .filter_map(|line| {
            let (pts, flags) = line.trim().split_once(',')?;
            if !flags.starts_with('K') {
                return None;
            }
            pts.parse::<f64>().ok().map(|t| (t - start_time).max(0.0))
        })
        .collect();
    out.sort_by(|a, b| a.total_cmp(b));
    out.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    Ok(out)
}

fn run_ffprobe(tools: &Tools, args: &[&str], path: &Path) -> Result<String> {
    if !path.is_file() {
        return Err(EngineError::Probe(format!("file not found: {}", path.display())));
    }
    let out = tools.cmd(Tool::Ffprobe).args(args).arg(path).output()?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(EngineError::Probe(if msg.is_empty() { "unreadable file".into() } else { msg }));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// mp4 stores titles as `name`, mkv as `title`; OBS may only set the handler name.
fn track_title(tags: &Value) -> Option<String> {
    const GENERIC: [&str; 4] = ["soundhandler", "sound media handler", "soundhandle", "audio"];
    ["title", "name", "handler_name"]
        .iter()
        .filter_map(|k| tags[*k].as_str())
        .map(str::trim)
        .find(|t| !t.is_empty() && !GENERIC.contains(&t.to_lowercase().as_str()))
        .map(str::to_string)
}

fn num(v: &Value) -> Option<f64> {
    v.as_str().and_then(|s| s.parse().ok()).or_else(|| v.as_f64())
}

fn rate(v: &Value) -> Option<f64> {
    let (n, d) = v.as_str()?.split_once('/')?;
    let (n, d): (f64, f64) = (n.parse().ok()?, d.parse().ok()?);
    (n > 0.0 && d > 0.0).then(|| n / d)
}
