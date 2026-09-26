//! Size and quality math for Shrink exports. Pure functions, no I/O.

use serde::{Deserialize, Serialize};

/// Audio bitrate for shrunk files (AAC stereo).
pub const AUDIO_BPS: u64 = 160_000;
/// Share of the budget held back so the file lands under the target.
pub const SAFETY: f64 = 0.03;
/// Below this many bits per pixel per frame, fast motion looks blocky.
/// From the discord-clip.ps1 VMAF tests: 0.033 was where loss became visible.
pub const WARN_BPP: f64 = 0.035;
/// At or above this, a format counts as clean (0.048 matched the original, VMAF 96).
pub const CLEAN_BPP: f64 = 0.045;
/// Encoders produce garbage below this; such targets are rejected.
pub const MIN_VIDEO_BPS: u64 = 100_000;

const MIB: u64 = 1024 * 1024;
const HEIGHT_LADDER: [u32; 5] = [1440, 1080, 720, 480, 360];

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Format {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub format: Format,
    pub bpp: f64,
    pub clean: bool,
    pub suggested: bool,
}

/// Video bits per second that fit `target_bytes` over `duration` seconds.
pub fn video_bitrate(target_bytes: u64, duration: f64, has_audio: bool) -> u64 {
    let total = target_bytes as f64 * 8.0 / duration;
    let audio = if has_audio { AUDIO_BPS as f64 } else { 0.0 };
    ((total - audio) * (1.0 - SAFETY)).max(0.0) as u64
}

/// Bits per pixel per frame.
pub fn bpp(video_bps: u64, f: &Format) -> f64 {
    video_bps as f64 / (f.width as f64 * f.height as f64 * f.fps)
}

/// Largest target (bytes) that still falls inside the "too small" zone.
pub fn warn_zone_max_bytes(duration: f64, has_audio: bool, f: &Format) -> u64 {
    let video = WARN_BPP * f.width as f64 * f.height as f64 * f.fps;
    let audio = if has_audio { AUDIO_BPS as f64 } else { 0.0 };
    ((video / (1.0 - SAFETY) + audio) * duration / 8.0).ceil() as u64
}

/// Slider upper end: the longest section at the source's own bitrate, at least 1 MB.
pub fn slider_max_bytes(source_bytes: u64, source_duration: f64, longest_section: f64) -> u64 {
    let share = (source_bytes as f64 * longest_section / source_duration).round() as u64;
    share.max(MIB)
}

/// Everything the size slider needs for the current sections and target.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SizeAdvice {
    pub slider_min_bytes: u64,
    pub slider_max_bytes: u64,
    /// Targets at or below this are in the yellow zone.
    pub warn_max_bytes: u64,
    pub too_small: bool,
    pub suggestions: Vec<Suggestion>,
}

/// Advice for the longest section, since it's the one that needs the most room.
pub fn advice(source_bytes: u64, source_duration: f64, longest: f64, target_bytes: u64, has_audio: bool, src: &Format) -> SizeAdvice {
    let audio = if has_audio { AUDIO_BPS as f64 } else { 0.0 };
    let min = ((MIN_VIDEO_BPS as f64 / (1.0 - SAFETY) + audio) * longest / 8.0).ceil() as u64;
    let max = slider_max_bytes(source_bytes, source_duration, longest);
    SizeAdvice {
        slider_min_bytes: min.max(MIB).min(max),
        slider_max_bytes: max,
        warn_max_bytes: warn_zone_max_bytes(longest, has_audio, src),
        too_small: bpp(video_bitrate(target_bytes, longest, has_audio), src) < WARN_BPP,
        suggestions: suggestions(target_bytes, longest, has_audio, src),
    }
}

/// Lower resolution/fps options for `src`, best (most pixels per second) first.
pub fn candidates(src: &Format) -> Vec<Format> {
    let mut heights: Vec<u32> = HEIGHT_LADDER.iter().copied().filter(|h| *h <= src.height).collect();
    if !heights.contains(&src.height) {
        heights.insert(0, src.height);
    }
    let mut rates = vec![src.fps];
    for r in [60.0, 30.0] {
        // offer 30 only when the source is 60 or less
        if r < src.fps && (r == 60.0 || src.fps <= 60.0) {
            rates.push(r);
        }
    }
    let mut out: Vec<Format> = heights
        .iter()
        .flat_map(|&h| {
            let w = ((src.width as f64 * h as f64 / src.height as f64 / 2.0).round() as u32) * 2;
            rates.iter().map(move |&fps| Format { width: w, height: h, fps })
        })
        .filter(|f| f != src)
        .collect();
    out.sort_by(|a, b| {
        let ra = a.width as f64 * a.height as f64 * a.fps;
        let rb = b.width as f64 * b.height as f64 * b.fps;
        rb.total_cmp(&ra)
    });
    out
}

/// One-click fixes for a tight target: the best clean format (suggested), plus the
/// next better one as an alternative. Empty when the source format is fine.
pub fn suggestions(target_bytes: u64, duration: f64, has_audio: bool, src: &Format) -> Vec<Suggestion> {
    let vbps = video_bitrate(target_bytes, duration, has_audio);
    if bpp(vbps, src) >= WARN_BPP {
        return Vec::new();
    }
    let cands = candidates(src);
    if cands.is_empty() {
        return Vec::new();
    }
    let make = |f: Format, suggested: bool| {
        let b = bpp(vbps, &f);
        Suggestion { format: f, bpp: b, clean: b >= CLEAN_BPP, suggested }
    };
    let i = cands
        .iter()
        .position(|f| bpp(vbps, f) >= CLEAN_BPP)
        .unwrap_or(cands.len() - 1);
    let mut out = vec![make(cands[i], true)];
    if i > 0 {
        out.push(make(cands[i - 1], false));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIB: u64 = 1024 * 1024;
    const QHD120: Format = Format { width: 2560, height: 1440, fps: 120.0 };
    const FHD120: Format = Format { width: 1920, height: 1080, fps: 120.0 };

    #[test]
    fn video_bitrate_subtracts_audio_and_margin() {
        // (25 MB * 8 / 30 s - 160 kbps) * 0.97
        let expected = ((25.0 * MIB as f64 * 8.0 / 30.0) - 160_000.0) * 0.97;
        assert_eq!(video_bitrate(25 * MIB, 30.0, true), expected as u64);
    }

    #[test]
    fn video_bitrate_without_audio_uses_everything() {
        let expected = (10.0 * MIB as f64 * 8.0 / 20.0) * 0.97;
        assert_eq!(video_bitrate(10 * MIB, 20.0, false), expected as u64);
    }

    #[test]
    fn video_bitrate_never_negative() {
        assert_eq!(video_bitrate(1000, 60.0, true), 0);
    }

    #[test]
    fn bpp_matches_discord_clip_measurement() {
        // 12 Mbps at 1080p120 looked like the original (VMAF 96.1)
        let b = bpp(12_000_000, &FHD120);
        assert!((b - 0.0482).abs() < 0.0005, "{b}");
    }

    #[test]
    fn warn_zone_boundary_sits_at_warn_bpp() {
        let max = warn_zone_max_bytes(30.0, true, &QHD120);
        let at = bpp(video_bitrate(max, 30.0, true), &QHD120);
        assert!((at - WARN_BPP).abs() < 0.0005, "{at}");
        assert!(bpp(video_bitrate(max - MIB, 30.0, true), &QHD120) < WARN_BPP);
    }

    #[test]
    fn candidates_never_exceed_source_and_keep_aspect() {
        let c = candidates(&QHD120);
        assert!(!c.contains(&QHD120));
        assert!(c.iter().all(|f| f.height <= 1440 && f.fps <= 120.0));
        assert!(c.iter().all(|f| f.width % 2 == 0));
        assert!(c.contains(&Format { width: 1280, height: 720, fps: 60.0 }));
        // sorted best (most pixels per second) first
        let rates: Vec<f64> = c.iter().map(|f| f.width as f64 * f.height as f64 * f.fps).collect();
        assert!(rates.windows(2).all(|w| w[0] >= w[1]));
    }

    #[test]
    fn candidates_for_60fps_source_offer_30() {
        let src = Format { width: 1920, height: 1080, fps: 60.0 };
        let c = candidates(&src);
        assert!(c.contains(&Format { width: 1920, height: 1080, fps: 30.0 }));
        assert!(c.iter().all(|f| f.fps <= 60.0));
    }

    #[test]
    fn tight_target_suggests_highest_clean_format_plus_next_better() {
        let s = suggestions(10 * MIB, 30.0, true, &QHD120);
        assert_eq!(s.len(), 2, "{s:?}");
        assert!(s[0].suggested && s[0].clean);
        assert_eq!(s[0].format, Format { width: 1280, height: 720, fps: 60.0 });
        assert!(!s[1].suggested && !s[1].clean);
        // the alternative is the next candidate above the suggestion
        let c = candidates(&QHD120);
        let i = c.iter().position(|f| *f == s[0].format).unwrap();
        assert_eq!(s[1].format, c[i - 1]);
    }

    #[test]
    fn roomy_target_needs_no_suggestions() {
        assert!(suggestions(100 * MIB, 30.0, true, &QHD120).is_empty());
    }

    #[test]
    fn hopeless_target_suggests_smallest_format_not_clean() {
        let s = suggestions(MIB, 60.0, true, &QHD120);
        let smallest = *candidates(&QHD120).last().unwrap();
        assert!(s[0].suggested && !s[0].clean);
        assert_eq!(s[0].format, smallest);
    }

    #[test]
    fn advice_for_tight_target_flags_zone_and_suggests() {
        let a = advice(200 * MIB, 70.0, 30.0, 10 * MIB, true, &QHD120);
        assert!(a.too_small);
        assert_eq!(a.warn_max_bytes, warn_zone_max_bytes(30.0, true, &QHD120));
        assert_eq!(a.slider_max_bytes, slider_max_bytes(200 * MIB, 70.0, 30.0));
        assert!(a.suggestions[0].suggested);
        // the smallest allowed target still leaves the minimum video bitrate
        assert!(video_bitrate(a.slider_min_bytes, 30.0, true) >= MIN_VIDEO_BPS);
        assert!(video_bitrate(a.slider_min_bytes - 1024, 30.0, true) < MIN_VIDEO_BPS || a.slider_min_bytes == MIB);
    }

    #[test]
    fn advice_for_roomy_target_is_quiet() {
        let a = advice(200 * MIB, 70.0, 10.0, 60 * MIB, true, &QHD120);
        assert!(!a.too_small);
        assert!(a.suggestions.is_empty());
    }

    #[test]
    fn slider_max_is_longest_section_at_source_size() {
        // 200 MB over 70 s, longest section 14 s -> 40 MB
        assert_eq!(slider_max_bytes(200 * MIB, 70.0, 14.0), 40 * MIB);
        // never below 1 MB
        assert_eq!(slider_max_bytes(MIB, 70.0, 1.0), MIB);
    }
}
