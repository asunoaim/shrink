mod common;

use shrink_lib::engine::encoder::{detect, Encoder};
use shrink_lib::engine::plan::{plan, ExportJob, ExportRequest, Mode, Section};
use shrink_lib::engine::probe::probe;
use shrink_lib::engine::runner::{run_all, run_job, CancelFlag, ClipOutcome, RunEvent};
use shrink_lib::engine::tools::{Tool, Tools};
use shrink_lib::engine::EngineError;
use std::path::Path;

const KIB: u64 = 1024;

fn jobs_for(src: &Path, out: &Path, sections: &[(f64, f64)], mode: Mode, audio: &[usize], enc: Encoder) -> Vec<ExportJob> {
    let t = common::tools();
    let info = probe(&t, src).unwrap();
    let req = ExportRequest {
        sections: sections.iter().map(|&(start, end)| Section { start, end, number: None }).collect(),
        mode,
        audio_tracks: audio.to_vec(),
        out_dir: out.to_path_buf(),
    };
    plan(&info, &req, enc, &|p| p.exists()).unwrap()
}

/// Hashes of the first `n` decoded video frames, starting at `ss` seconds.
fn frame_hashes(t: &Tools, file: &Path, ss: f64, n: u32) -> Vec<String> {
    let out = t
        .cmd(Tool::Ffmpeg)
        .args(["-v", "error", "-ss", &ss.to_string(), "-i"])
        .arg(file)
        .args(["-map", "0:v:0", "-frames:v", &n.to_string(), "-f", "framemd5", "-"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.rsplit(',').next().unwrap().trim().to_string())
        .collect()
}

fn run_ok(job: &ExportJob) -> std::path::PathBuf {
    run_job(&common::tools(), job, &CancelFlag::new(), &mut |_| {}).unwrap()
}

#[test]
fn original_export_is_lossless_from_the_keyframe() {
    let t = common::tools();
    let dir = common::out_dir();
    let src = common::fixture_two_tracks();
    let jobs = jobs_for(&src, dir.path(), &[(3.4, 6.0)], Mode::Original, &[0, 1], Encoder::X264);
    let out = run_ok(&jobs[0]);

    let info = probe(&t, &out).unwrap();
    // copy keeps whole packets, so the end can run a few frames long
    assert!((info.duration - 3.0).abs() < 0.15, "duration {}", info.duration);
    assert_eq!(info.audio_tracks.len(), 2, "tracks stay separate");

    let expected = frame_hashes(&t, &src, 3.0, 30);
    assert_eq!(expected.len(), 30);
    assert_eq!(frame_hashes(&t, &out, 0.0, 30), expected);
}

#[test]
fn shrink_stays_under_target_and_mixes_audio() {
    let t = common::tools();
    let dir = common::out_dir();
    let target = 300 * KIB;
    let mode = Mode::Shrink { target_bytes: target, format: None };
    let jobs = jobs_for(&common::fixture_two_tracks(), dir.path(), &[(0.0, 8.0)], mode, &[0, 1], detect(&t));
    let out = run_ok(&jobs[0]);

    let size = std::fs::metadata(&out).unwrap().len();
    assert!(size <= target, "{size} > {target}");
    let info = probe(&t, &out).unwrap();
    assert_eq!(info.video_codec, "h264");
    assert_eq!(info.audio_tracks.len(), 1);
    assert!((info.duration - 8.0).abs() < 0.1, "duration {}", info.duration);
}

#[test]
fn shrink_without_audio_source_works() {
    let dir = common::out_dir();
    let mode = Mode::Shrink { target_bytes: 400 * KIB, format: None };
    let jobs = jobs_for(&common::fixture_no_audio(), dir.path(), &[(1.0, 5.0)], mode, &[], Encoder::X264);
    let info = probe(&common::tools(), &run_ok(&jobs[0])).unwrap();
    assert!(info.audio_tracks.is_empty());
}

#[test]
fn cancel_before_start_writes_nothing() {
    let dir = common::out_dir();
    let jobs = jobs_for(&common::fixture_two_tracks(), dir.path(), &[(0.0, 9.0)], Mode::Original, &[0], Encoder::X264);
    let cancel = CancelFlag::new();
    cancel.cancel();
    let r = run_job(&common::tools(), &jobs[0], &cancel, &mut |_| {});
    assert!(matches!(r, Err(EngineError::Cancelled)), "{r:?}");
    assert!(!jobs[0].output.exists());
}

#[test]
fn cancel_during_job_removes_the_partial_file() {
    let dir = common::out_dir();
    let mode = Mode::Shrink { target_bytes: 2048 * KIB, format: None };
    let jobs = jobs_for(&common::fixture_two_tracks(), dir.path(), &[(0.0, 10.0)], mode, &[0], Encoder::X264);
    let cancel = CancelFlag::new();
    let c = cancel.clone();
    let r = run_job(&common::tools(), &jobs[0], &cancel, &mut |_| c.cancel());
    assert!(matches!(r, Err(EngineError::Cancelled)), "{r:?}");
    assert!(!jobs[0].output.exists());
}

#[test]
fn progress_rises_and_finishes_at_one() {
    let dir = common::out_dir();
    let mode = Mode::Shrink { target_bytes: 2048 * KIB, format: None };
    let jobs = jobs_for(&common::fixture_two_tracks(), dir.path(), &[(0.0, 10.0)], mode, &[0], Encoder::X264);
    let mut seen = Vec::new();
    run_job(&common::tools(), &jobs[0], &CancelFlag::new(), &mut |f| seen.push(f)).unwrap();
    assert!(!seen.is_empty());
    assert!(seen.windows(2).all(|w| w[1] >= w[0]), "{seen:?}");
    assert!(seen.iter().all(|f| (0.0..=1.0).contains(f)), "{seen:?}");
    assert_eq!(*seen.last().unwrap(), 1.0);
}

#[test]
fn a_failing_clip_does_not_stop_the_others() {
    let dir = common::out_dir();
    let mut jobs = jobs_for(&common::fixture_two_tracks(), dir.path(), &[(0.0, 2.0), (3.0, 5.0)], Mode::Original, &[0], Encoder::X264);
    jobs[0].output = dir.path().join("missing folder").join("x.mp4");

    let mut events = Vec::new();
    let outcomes = run_all(&common::tools(), &jobs, &CancelFlag::new(), &mut |e| events.push(e));
    assert!(matches!(outcomes[0], ClipOutcome::Failed { .. }), "{:?}", outcomes[0]);
    assert!(matches!(outcomes[1], ClipOutcome::Done { .. }), "{:?}", outcomes[1]);
    assert!(events.iter().any(|e| matches!(e, RunEvent::ClipStarted { clip_number: 2, clip_count: 2 })));
    assert!(!jobs[0].output.exists());
}

#[test]
fn cancel_marks_remaining_clips_cancelled() {
    let dir = common::out_dir();
    let jobs = jobs_for(&common::fixture_two_tracks(), dir.path(), &[(0.0, 2.0), (3.0, 5.0)], Mode::Original, &[0], Encoder::X264);
    let cancel = CancelFlag::new();
    let c = cancel.clone();
    let outcomes = run_all(&common::tools(), &jobs, &cancel, &mut |e| {
        if matches!(e, RunEvent::ClipFinished { clip_number: 1, .. }) {
            c.cancel();
        }
    });
    assert!(matches!(outcomes[0], ClipOutcome::Done { .. }));
    assert_eq!(outcomes[1], ClipOutcome::Cancelled);
}

#[test]
fn paths_with_spaces_and_umlauts_work() {
    let dir = common::out_dir();
    let src = common::copy_fixture(&common::fixture_two_tracks(), dir.path(), "Replay (ä) 1.mp4");
    let jobs = jobs_for(&src, dir.path(), &[(1.0, 3.0)], Mode::Original, &[0], Encoder::X264);
    let out = run_ok(&jobs[0]);
    assert!(out.ends_with("Replay (ä) 1 - clip 1.mp4"));
    assert!(out.exists());
}

#[test]
fn a_file_that_appears_after_planning_is_never_overwritten() {
    let dir = common::out_dir();
    let jobs = jobs_for(&common::fixture_two_tracks(), dir.path(), &[(0.0, 2.0)], Mode::Original, &[0], Encoder::X264);
    // another export (or program) creates the same name before this job starts
    std::fs::write(&jobs[0].output, b"someone else's file").unwrap();
    let r = run_job(&common::tools(), &jobs[0], &CancelFlag::new(), &mut |_| {});
    assert!(r.is_err(), "{r:?}");
    assert_eq!(std::fs::read(&jobs[0].output).unwrap(), b"someone else's file");
}

#[cfg(windows)]
#[test]
fn ffmpeg_is_killed_when_its_job_object_closes() {
    use shrink_lib::engine::process_job::ProcessJob;
    use std::time::{Duration, Instant};
    let job = ProcessJob::new().unwrap();
    let mut child = common::tools()
        .cmd(Tool::Ffmpeg)
        .args(["-v", "error", "-re", "-f", "lavfi", "-i", "testsrc2=size=320x240:rate=30:duration=60", "-f", "null", "-"])
        .spawn()
        .unwrap();
    job.assign(&child).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    assert!(child.try_wait().unwrap().is_none(), "ffmpeg should still be running");
    let t = Instant::now();
    drop(job); // what happens to the app's job handle when the app exits
    child.wait().unwrap(); // (killed processes report exit code 0, so only timing proves it)
    assert!(t.elapsed() < Duration::from_secs(5), "ran {:?} of a 60 s real-time job", t.elapsed());
}
