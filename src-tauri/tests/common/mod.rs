//! Shared test helpers: locate ffmpeg and generate small synthetic clips once.
#![allow(dead_code)]

use shrink_lib::engine::tools::{Tool, Tools};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static GEN_LOCK: Mutex<()> = Mutex::new(());

pub fn tools() -> Tools {
    Tools::locate().expect("ffmpeg and ffprobe must be installed to run the tests")
}

pub fn fixtures_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fixtures");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 640x360, 60 fps, 10 s, H.264 with a keyframe exactly every second,
/// two AAC tracks titled "Game" (440 Hz) and "Mic" (880 Hz).
pub fn fixture_two_tracks() -> PathBuf {
    generate("two_tracks.mp4", true)
}

/// Same video, no audio.
pub fn fixture_no_audio() -> PathBuf {
    generate("no_audio.mp4", false)
}

/// A fresh output directory for one test.
pub fn out_dir() -> tempfile::TempDir {
    tempfile::tempdir_in(fixtures_dir()).unwrap()
}

/// Copy a fixture to `dir/name`, for path-handling tests.
pub fn copy_fixture(src: &Path, dir: &Path, name: &str) -> PathBuf {
    let dst = dir.join(name);
    std::fs::copy(src, &dst).unwrap();
    dst
}

fn generate(name: &str, audio: bool) -> PathBuf {
    let _guard = GEN_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let out = fixtures_dir().join(name);
    if out.exists() {
        return out;
    }
    let tmp = out.with_extension("tmp.mp4");
    let mut args: Vec<&str> = vec![
        "-hide_banner", "-v", "error", "-y",
        "-f", "lavfi", "-i", "testsrc2=size=640x360:rate=60:duration=10",
    ];
    if audio {
        args.extend([
            "-f", "lavfi", "-i", "sine=frequency=440:duration=10",
            "-f", "lavfi", "-i", "sine=frequency=880:duration=10",
            "-map", "0:v", "-map", "1:a", "-map", "2:a",
            "-c:a", "aac", "-metadata:s:a:0", "title=Game", "-metadata:s:a:1", "title=Mic",
        ]);
    }
    args.extend([
        "-c:v", "libx264", "-g", "60", "-keyint_min", "60", "-sc_threshold", "0",
        "-pix_fmt", "yuv420p",
    ]);
    let status = tools()
        .cmd(Tool::Ffmpeg)
        .args(&args)
        .arg(&tmp)
        .status()
        .unwrap();
    assert!(status.success(), "fixture generation failed for {name}");
    std::fs::rename(&tmp, &out).unwrap();
    out
}
