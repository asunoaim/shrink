mod common;

use shrink_lib::engine::encoder::Encoder;
use shrink_lib::engine::preview::{proxy_job, thumbnail_strip};
use shrink_lib::engine::probe::probe;
use shrink_lib::engine::runner::{run_job, CancelFlag};
use shrink_lib::engine::tools::Tool;

#[test]
fn thumbnail_strip_is_one_row_of_count_thumbs() {
    let t = common::tools();
    let dir = common::out_dir();
    let info = probe(&t, &common::fixture_two_tracks()).unwrap();
    let png = dir.path().join("strip.png");
    thumbnail_strip(&t, &info, &png, 12, 90).unwrap();

    let out = t
        .cmd(Tool::Ffprobe)
        .args(["-v", "error", "-show_entries", "stream=width,height", "-of", "csv=p=0"])
        .arg(&png)
        .output()
        .unwrap();
    let dims = String::from_utf8_lossy(&out.stdout).trim().to_string();
    // 640x360 scaled to 90 px tall = 160 px wide, 12 of them side by side
    assert_eq!(dims, "1920,90");
}

#[test]
fn proxy_is_small_seekable_h264() {
    let t = common::tools();
    let dir = common::out_dir();
    let info = probe(&t, &common::fixture_two_tracks()).unwrap();
    let job = proxy_job(&info, &dir.path().join("proxy.mp4"), Encoder::X264);
    let out = run_job(&t, &job, &CancelFlag::new(), &mut |_| {}).unwrap();

    let p = probe(&t, &out).unwrap();
    assert_eq!(p.video_codec, "h264");
    assert!(p.height <= 720);
    assert!((p.duration - info.duration).abs() < 0.2);
    assert_eq!(p.audio_tracks.len(), 1);
    // a keyframe every half second makes scrubbing smooth
    assert!(p.keyframes.len() >= 19, "{:?}", p.keyframes);
}
