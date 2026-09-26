//! Which H.264 encoder to use, and how to drive it for size-targeted output.

use super::tools::{Tool, Tools};
use serde::{Deserialize, Serialize};
use std::process::Stdio;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Encoder {
    Nvenc,
    Amf,
    Qsv,
    X264,
}

impl Encoder {
    /// Preference order: NVIDIA, AMD, Intel, then the CPU.
    pub const ORDER: [Encoder; 4] = [Encoder::Nvenc, Encoder::Amf, Encoder::Qsv, Encoder::X264];

    pub fn ffmpeg_name(self) -> &'static str {
        match self {
            Encoder::Nvenc => "h264_nvenc",
            Encoder::Amf => "h264_amf",
            Encoder::Qsv => "h264_qsv",
            Encoder::X264 => "libx264",
        }
    }

    pub fn is_hardware(self) -> bool {
        self != Encoder::X264
    }

    /// Video encoder arguments for an average of `bitrate` bits per second.
    /// GPU encoders run constant bitrate with a half-second buffer: on short game
    /// clips their VBR mode overshot the size by 10-74 %, CBR stayed within 2 %.
    pub fn video_args(self, bitrate: u64, gop: Option<u32>) -> Vec<String> {
        let (b, half, double) = (bitrate.to_string(), (bitrate / 2).to_string(), (bitrate * 2).to_string());
        let mut a: Vec<String> = vec!["-c:v".into(), self.ffmpeg_name().into()];
        let rate: Vec<&str> = match self {
            // p3 measured the same VMAF as p5 (88.78 vs 88.80) at 2.5x the speed
            Encoder::Nvenc => vec![
                "-preset", "p3", "-tune", "hq", "-rc", "cbr", "-multipass", "fullres",
                "-spatial-aq", "1", "-bf", "3", "-b:v", &b, "-bufsize", &half,
            ],
            Encoder::Amf => vec!["-quality", "quality", "-rc", "cbr", "-b:v", &b, "-bufsize", &half],
            Encoder::Qsv => vec!["-preset", "medium", "-b:v", &b, "-maxrate", &b, "-bufsize", &half],
            Encoder::X264 => vec!["-preset", "medium", "-b:v", &b, "-maxrate", &double, "-bufsize", &double],
        };
        a.extend(rate.iter().map(|s| s.to_string()));
        if let Some(g) = gop {
            a.extend(["-g".into(), g.to_string()]);
        }
        a.extend(["-profile:v", "high", "-pix_fmt", "yuv420p"].map(String::from));
        a
    }
}

/// Does a tiny test encode succeed with this encoder on this PC?
pub fn works(tools: &Tools, e: Encoder) -> bool {
    tools
        .cmd(Tool::Ffmpeg)
        .args(["-hide_banner", "-v", "error", "-f", "lavfi", "-i", "color=black:s=256x256:r=30:d=0.2"])
        .args(e.video_args(1_000_000, None))
        .args(["-f", "null", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// First encoder in [`Encoder::ORDER`] that works; the CPU as a last resort.
pub fn detect(tools: &Tools) -> Encoder {
    Encoder::ORDER
        .into_iter()
        .find(|e| works(tools, *e))
        .unwrap_or(Encoder::X264)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_pair(args: &[String], key: &str, val: &str) -> bool {
        args.windows(2).any(|w| w[0] == key && w[1] == val)
    }

    #[test]
    fn every_encoder_gets_bitrate() {
        for e in Encoder::ORDER {
            let a = e.video_args(4_000_000, None);
            assert!(has_pair(&a, "-c:v", e.ffmpeg_name()), "{e:?}");
            assert!(has_pair(&a, "-b:v", "4000000"), "{e:?}");
            assert!(has_pair(&a, "-pix_fmt", "yuv420p"), "{e:?}");
        }
    }

    #[test]
    fn gpu_encoders_use_constant_bitrate_with_half_second_buffer() {
        // measured on an RTX 3070: VBR overshot short game clips by 10-74 %,
        // CBR with full-res two-pass and a 0.5 s buffer stayed within 2 %
        let a = Encoder::Nvenc.video_args(4_000_000, None);
        assert!(has_pair(&a, "-rc", "cbr"));
        assert!(has_pair(&a, "-multipass", "fullres"));
        assert!(has_pair(&a, "-bufsize", "2000000"));
        assert!(has_pair(&Encoder::Amf.video_args(4_000_000, None), "-rc", "cbr"));
        assert!(has_pair(&Encoder::Qsv.video_args(4_000_000, None), "-maxrate", "4000000"));
    }

    #[test]
    fn nvenc_uses_the_fast_preset_that_measured_equal_quality() {
        // p3 vs p5 at 6 Mbps 1080p120: VMAF 88.78 vs 88.80, 2.5x faster
        let a = Encoder::Nvenc.video_args(1_000_000, None);
        assert!(has_pair(&a, "-preset", "p3"));
        assert!(has_pair(&a, "-spatial-aq", "1"));
    }

    #[test]
    fn x264_keeps_average_bitrate_mode() {
        let a = Encoder::X264.video_args(4_000_000, None);
        assert!(has_pair(&a, "-maxrate", "8000000"));
        assert!(!a.contains(&"-rc".to_string()));
    }

    #[test]
    fn gop_is_passed_when_given() {
        let a = Encoder::Nvenc.video_args(1_000_000, Some(60));
        assert!(has_pair(&a, "-g", "60"));
        assert!(!Encoder::Nvenc.video_args(1_000_000, None).contains(&"-g".to_string()));
    }

    #[test]
    fn only_x264_is_software() {
        assert!(!Encoder::X264.is_hardware());
        assert!(Encoder::Nvenc.is_hardware() && Encoder::Amf.is_hardware() && Encoder::Qsv.is_hardware());
        assert_eq!(Encoder::ORDER.last(), Some(&Encoder::X264));
    }
}
