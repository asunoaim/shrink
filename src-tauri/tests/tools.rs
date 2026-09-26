mod common;

use shrink_lib::engine::tools::Tools;

#[test]
fn locate_finds_ffmpeg_and_ffprobe() {
    let t = Tools::locate().unwrap();
    assert!(t.ffmpeg.is_file());
    assert!(t.ffprobe.is_file());
}

#[test]
fn locate_in_picks_first_dir_with_both_tools() {
    let empty = tempfile::tempdir().unwrap();
    let only_ffmpeg = tempfile::tempdir().unwrap();
    std::fs::write(only_ffmpeg.path().join("ffmpeg.exe"), b"").unwrap();
    let both = tempfile::tempdir().unwrap();
    std::fs::write(both.path().join("ffmpeg.exe"), b"").unwrap();
    std::fs::write(both.path().join("ffprobe.exe"), b"").unwrap();

    let dirs = vec![
        empty.path().to_path_buf(),
        only_ffmpeg.path().to_path_buf(),
        both.path().to_path_buf(),
    ];
    let t = Tools::locate_in(&dirs).unwrap();
    assert_eq!(t.ffmpeg, both.path().join("ffmpeg.exe"));
    assert_eq!(t.ffprobe, both.path().join("ffprobe.exe"));
}

#[test]
fn locate_in_returns_none_without_tools() {
    let empty = tempfile::tempdir().unwrap();
    assert!(Tools::locate_in(&[empty.path().to_path_buf()]).is_none());
}

#[test]
fn fixtures_are_generated() {
    for p in [common::fixture_two_tracks(), common::fixture_no_audio()] {
        assert!(std::fs::metadata(&p).unwrap().len() > 10_000, "{p:?}");
    }
}
