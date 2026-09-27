mod common;

use shrink_lib::engine::probe::{keyframes, probe, probe_quick};
use shrink_lib::engine::EngineError;

#[test]
fn reads_video_properties() {
    let info = probe(&common::tools(), &common::fixture_two_tracks()).unwrap();
    assert!((info.duration - 10.0).abs() < 0.1, "duration {}", info.duration);
    assert_eq!((info.width, info.height), (640, 360));
    assert!((info.fps - 60.0).abs() < 0.01);
    assert_eq!(info.video_codec, "h264");
    assert_eq!(info.container_ext, "mp4");
    assert!(info.size_bytes > 0);
    assert!(info.bitrate > 0);
}

#[test]
fn reads_audio_tracks_with_titles() {
    let info = probe(&common::tools(), &common::fixture_two_tracks()).unwrap();
    let titles: Vec<_> = info.audio_tracks.iter().map(|t| t.title.clone()).collect();
    assert_eq!(titles, vec![Some("Game".to_string()), Some("Mic".to_string())]);
    assert_eq!(info.audio_tracks[1].index, 1);
    assert_eq!(info.audio_tracks[0].channels, 1);
}

#[test]
fn clip_without_audio_has_no_tracks() {
    let info = probe(&common::tools(), &common::fixture_no_audio()).unwrap();
    assert!(info.audio_tracks.is_empty());
}

#[test]
fn keyframes_come_every_second() {
    let info = probe(&common::tools(), &common::fixture_two_tracks()).unwrap();
    assert_eq!(info.keyframes.len(), 10, "{:?}", info.keyframes);
    for (i, k) in info.keyframes.iter().enumerate() {
        assert!((k - i as f64).abs() < 0.02, "keyframe {i} at {k}");
    }
}

#[test]
fn path_with_spaces_and_umlaut_works() {
    let dir = common::out_dir();
    let p = common::copy_fixture(&common::fixture_two_tracks(), dir.path(), "Replay 2026 (ä).mp4");
    let info = probe(&common::tools(), &p).unwrap();
    assert_eq!(info.width, 640);
}

#[test]
fn non_video_file_is_a_probe_error() {
    let dir = common::out_dir();
    let p = dir.path().join("notes.mp4");
    std::fs::write(&p, b"this is not a video").unwrap();
    match probe(&common::tools(), &p) {
        Err(EngineError::Probe(_)) => {}
        other => panic!("expected Probe error, got {other:?}"),
    }
}

#[test]
fn missing_file_is_a_probe_error() {
    let dir = common::out_dir();
    match probe(&common::tools(), &dir.path().join("gone.mp4")) {
        Err(EngineError::Probe(_)) => {}
        other => panic!("expected Probe error, got {other:?}"),
    }
}

#[test]
fn quick_probe_skips_the_keyframe_scan() {
    let t = common::tools();
    let quick = probe_quick(&t, &common::fixture_two_tracks()).unwrap();
    assert!(quick.keyframes.is_empty());
    assert_eq!((quick.width, quick.height), (640, 360));
    assert_eq!(quick.audio_tracks.len(), 2);
}

#[test]
fn keyframes_later_match_the_full_probe() {
    let t = common::tools();
    let full = probe(&t, &common::fixture_two_tracks()).unwrap();
    let quick = probe_quick(&t, &common::fixture_two_tracks()).unwrap();
    assert_eq!(keyframes(&t, &quick).unwrap(), full.keyframes);
}
