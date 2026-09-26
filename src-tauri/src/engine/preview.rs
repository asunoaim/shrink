//! Preview helpers: the timeline's thumbnail strip, and a playable copy for clips
//! the built-in player can't decode.

use super::encoder::Encoder;
use super::plan::{ExportJob, JobKind};
use super::probe::ClipInfo;
use super::quality::Format;
use super::tools::{Tool, Tools};
use super::{EngineError, Result};
use std::path::Path;

const PROXY_MAX_HEIGHT: u32 = 720;
const PROXY_BPS: u64 = 4_000_000;

/// One PNG with `count` evenly spaced thumbnails side by side, each `height` px tall.
/// Decodes keyframes only, so it stays fast on long recordings.
pub fn thumbnail_strip(tools: &Tools, info: &ClipInfo, out_png: &Path, count: u32, height: u32) -> Result<()> {
    let filter = format!(
        "fps={count}/{:.6},scale=-2:{height}:flags=bilinear,tile={count}x1",
        info.duration
    );
    let out = tools
        .cmd(Tool::Ffmpeg)
        .args(["-hide_banner", "-nostdin", "-v", "error", "-y", "-skip_frame", "nokey", "-i"])
        .arg(&info.path)
        .args(["-map", "0:v:0", "-vf", &filter, "-frames:v", "1", "-an"])
        .arg(out_png)
        .output()?;
    if !out.status.success() || !out_png.is_file() {
        return Err(EngineError::Ffmpeg {
            code: out.status.code(),
            log: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }
    Ok(())
}

/// A job that writes an easy-to-play H.264 copy: at most 720p, same fps,
/// a keyframe every half second, first audio track only.
pub fn proxy_job(info: &ClipInfo, out: &Path, encoder: Encoder) -> ExportJob {
    let scale = (info.height > PROXY_MAX_HEIGHT).then(|| {
        let w = ((info.width as f64 * PROXY_MAX_HEIGHT as f64 / info.height as f64 / 2.0).round() as u32) * 2;
        Format { width: w, height: PROXY_MAX_HEIGHT, fps: info.fps }
    });
    ExportJob {
        clip_number: 0,
        input: info.path.clone(),
        output: out.to_path_buf(),
        start: 0.0,
        duration: info.duration,
        audio_tracks: if info.audio_tracks.is_empty() { vec![] } else { vec![0] },
        kind: JobKind::Encode {
            encoder,
            video_bitrate: PROXY_BPS,
            target_bytes: None,
            scale,
            gop: Some(((info.fps / 2.0).round() as u32).max(1)),
        },
    }
}
