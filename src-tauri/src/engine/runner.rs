//! Runs planned jobs through ffmpeg: progress, cancel, cleanup, and the size promise.

use super::encoder::Encoder;
use super::plan::{ExportJob, JobKind};
use super::process_job::app_job;
use super::tools::{Tool, Tools};
use super::{EngineError, Result};
use serde::Serialize;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Re-encodes allowed per encoder when a file lands over its target.
/// A hardware encoder that can't get under gets the same number of tries with x264.
const MAX_RETRIES: u32 = 2;
/// Lines of ffmpeg's error output kept for the error message.
const LOG_LINES: usize = 12;

#[derive(Debug, Clone, Default)]
pub struct CancelFlag(Arc<AtomicBool>);

impl CancelFlag {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ClipOutcome {
    Done { output: PathBuf, size_bytes: u64 },
    Failed { error: String },
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RunEvent {
    ClipStarted { clip_number: usize, clip_count: usize },
    Progress { clip_number: usize, fraction: f64 },
    ClipFinished { clip_number: usize, outcome: ClipOutcome },
}

/// Bitrate for another attempt after landing at `actual` bytes instead of `target`.
pub fn retry_bitrate(current: u64, target: u64, actual: u64) -> u64 {
    (current as f64 * target as f64 / actual as f64 * 0.97) as u64
}

/// Run one job to completion. Progress goes 0..=1 and ends at exactly 1.
/// On cancel or failure the output file is removed.
pub fn run_job(tools: &Tools, job: &ExportJob, cancel: &CancelFlag, on_progress: &mut dyn FnMut(f64)) -> Result<PathBuf> {
    let mut job = job.clone();
    let planned_bitrate = match job.kind {
        JobKind::Encode { video_bitrate, .. } => video_bitrate,
        JobKind::Copy => 0,
    };
    let mut attempt = 0;
    loop {
        let mut last = 0.0_f64;
        let mut report = |f: f64| {
            // never go backwards within one attempt, even if ffmpeg's clock wobbles
            let f = f.clamp(0.0, 1.0).max(last);
            last = f;
            on_progress(f);
        };
        // claim the name first; if something else already has it, it isn't ours to touch
        claim(&job.output)?;
        let attempt_result = run_once(tools, &job, cancel, &mut report);
        if let Err(e) = attempt_result {
            let _ = std::fs::remove_file(&job.output);
            return Err(e);
        }
        let JobKind::Encode { target_bytes: Some(target), video_bitrate, encoder, .. } = &mut job.kind else {
            return Ok(job.output);
        };
        let size = std::fs::metadata(&job.output)?.len();
        if size <= *target {
            return Ok(job.output);
        }
        let _ = std::fs::remove_file(&job.output);
        if attempt < MAX_RETRIES {
            attempt += 1;
            *video_bitrate = retry_bitrate(*video_bitrate, *target, size);
        } else if encoder.is_hardware() {
            // GPU encoders have a floor on some content; x264 honours low rates reliably
            *encoder = Encoder::X264;
            *video_bitrate = planned_bitrate;
            attempt = 0;
        } else {
            return Err(EngineError::InvalidRequest(format!(
                "couldn't get clip {} under {:.1} MB",
                job.clip_number,
                *target as f64 / 1_048_576.0
            )));
        }
    }
}

fn run_once(tools: &Tools, job: &ExportJob, cancel: &CancelFlag, report: &mut dyn FnMut(f64)) -> Result<()> {
    if cancel.is_cancelled() {
        return Err(EngineError::Cancelled);
    }
    let mut child = tools
        .cmd(Tool::Ffmpeg)
        .args(job.args())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(job) = app_job() {
        // killed with the app instead of finishing unchecked in the background
        let _ = job.assign(&child);
    }

    let log = Arc::new(Mutex::new(VecDeque::<String>::new()));
    let stderr = child.stderr.take().expect("stderr is piped");
    let log_writer = Arc::clone(&log);
    let log_thread = std::thread::spawn(move || collect_log(stderr, &log_writer));

    let stdout = child.stdout.take().expect("stdout is piped");
    let mut cancelled = false;
    for line in BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        if let Some(us) = line.strip_prefix("out_time_us=") {
            if let Ok(us) = us.trim().parse::<f64>() {
                report(us / 1e6 / job.duration);
            }
        } else if line.trim() == "progress=end" {
            report(1.0);
        }
        if cancel.is_cancelled() {
            cancelled = true;
            let _ = child.kill();
            break;
        }
    }
    let status = child.wait()?;
    let _ = log_thread.join();
    if cancelled || cancel.is_cancelled() {
        return Err(EngineError::Cancelled);
    }
    if !status.success() {
        let log = log.lock().map(|l| Vec::from(l.clone()).join("\n")).unwrap_or_default();
        return Err(EngineError::Ffmpeg { code: status.code(), log });
    }
    Ok(())
}

/// Create the output file, failing if the name is already taken (ffmpeg's `-y`
/// then only overwrites the empty file this run just made).
fn claim(output: &std::path::Path) -> Result<()> {
    match std::fs::OpenOptions::new().write(true).create_new(true).open(output) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(EngineError::InvalidRequest(format!(
            "{} already exists",
            output.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        ))),
        Err(e) => Err(e.into()),
    }
}

fn collect_log(stderr: impl Read, log: &Mutex<VecDeque<String>>) {
    for line in BufReader::new(stderr).lines().map_while(|l| l.ok()) {
        if let Ok(mut l) = log.lock() {
            if l.len() == LOG_LINES {
                l.pop_front();
            }
            l.push_back(line);
        }
    }
}

/// Run every job in order. A failed clip doesn't stop the rest; after a cancel,
/// the remaining clips are reported as cancelled.
pub fn run_all(tools: &Tools, jobs: &[ExportJob], cancel: &CancelFlag, on_event: &mut dyn FnMut(RunEvent)) -> Vec<ClipOutcome> {
    let count = jobs.len();
    jobs.iter()
        .map(|job| {
            let n = job.clip_number;
            let outcome = if cancel.is_cancelled() {
                ClipOutcome::Cancelled
            } else {
                on_event(RunEvent::ClipStarted { clip_number: n, clip_count: count });
                let result = run_job(tools, job, cancel, &mut |fraction| {
                    on_event(RunEvent::Progress { clip_number: n, fraction })
                });
                match result {
                    Ok(output) => {
                        let size_bytes = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
                        ClipOutcome::Done { output, size_bytes }
                    }
                    Err(EngineError::Cancelled) => ClipOutcome::Cancelled,
                    Err(e) => ClipOutcome::Failed { error: e.to_string() },
                }
            };
            on_event(RunEvent::ClipFinished { clip_number: n, outcome: outcome.clone() });
            outcome
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_bitrate_scales_down_with_margin() {
        // landed 10% over: next try is 1/1.1 of the rate, minus 3%
        let b = retry_bitrate(1_000_000, 1_000, 1_100);
        assert_eq!(b, (1_000_000.0 * (1000.0 / 1100.0) * 0.97) as u64);
        assert!(b < 1_000_000);
    }
}
