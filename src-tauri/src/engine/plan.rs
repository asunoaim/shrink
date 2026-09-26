//! Planner: turns marked sections + export settings into exact ffmpeg jobs. Pure, no I/O.

use super::encoder::Encoder;
use super::naming::output_path;
use super::probe::ClipInfo;
use super::quality::{self, Format};
use super::{EngineError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Sections shorter than this are treated as empty.
const MIN_SECTION: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub start: f64,
    pub end: f64,
    /// Clip number to use (keeps "Try again" from renumbering); by timeline order when absent.
    #[serde(default)]
    pub number: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Mode {
    /// Lossless stream copy, starting at the keyframe at or before each section.
    Original,
    /// Re-encode each section to at most `target_bytes`, optionally at a lower format.
    Shrink { target_bytes: u64, format: Option<Format> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub sections: Vec<Section>,
    pub mode: Mode,
    /// Audio stream positions to keep (`0:a:<n>`).
    pub audio_tracks: Vec<usize>,
    pub out_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum JobKind {
    Copy,
    Encode {
        encoder: Encoder,
        video_bitrate: u64,
        /// Size promise to verify after encoding; `None` for preview copies.
        target_bytes: Option<u64>,
        scale: Option<Format>,
        gop: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportJob {
    pub clip_number: usize,
    pub input: PathBuf,
    pub output: PathBuf,
    /// Where the output starts in the source (the keyframe, for Copy).
    pub start: f64,
    pub duration: f64,
    pub audio_tracks: Vec<usize>,
    pub kind: JobKind,
}

/// The keyframe at or before `start` (0 if there is none).
pub fn actual_start(keyframes: &[f64], start: f64) -> f64 {
    keyframes
        .iter()
        .copied()
        .take_while(|k| *k <= start + 1e-3)
        .last()
        .unwrap_or(0.0)
}

pub fn plan(info: &ClipInfo, req: &ExportRequest, encoder: Encoder, exists: &dyn Fn(&Path) -> bool) -> Result<Vec<ExportJob>> {
    if req.sections.is_empty() {
        return Err(EngineError::InvalidRequest("mark at least one section".into()));
    }
    let mut audio = req.audio_tracks.clone();
    audio.sort_unstable();
    audio.dedup();
    if let Some(bad) = audio.iter().find(|i| **i >= info.audio_tracks.len()) {
        return Err(EngineError::InvalidRequest(format!("audio track {} doesn't exist", bad + 1)));
    }

    let mut sections: Vec<Section> = req
        .sections
        .iter()
        .map(|s| Section { start: s.start.max(0.0), end: s.end.min(info.duration), number: s.number })
        .collect();
    sections.sort_by(|a, b| a.start.total_cmp(&b.start));
    if let Some(i) = sections.iter().position(|s| s.end - s.start < MIN_SECTION) {
        return Err(EngineError::InvalidRequest(format!("section {} is empty", i + 1)));
    }

    let stem = info.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "clip".into());
    let source = Format { width: info.width, height: info.height, fps: info.fps };

    sections
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let clip_number = s.number.unwrap_or(i + 1);
            let (start, kind, ext) = match &req.mode {
                Mode::Original => (actual_start(&info.keyframes, s.start), JobKind::Copy, info.container_ext.as_str()),
                Mode::Shrink { target_bytes, format } => {
                    let duration = s.end - s.start;
                    // never bigger than the section already is in original quality
                    let original = (info.size_bytes as f64 * duration / info.duration) as u64;
                    let target_bytes = (*target_bytes).min(original.max(1));
                    let video_bitrate = quality::video_bitrate(target_bytes, duration, !audio.is_empty());
                    if video_bitrate < quality::MIN_VIDEO_BPS {
                        return Err(EngineError::InvalidRequest(format!(
                            "{:.1} MB is too small for a {:.0} s clip",
                            target_bytes as f64 / 1_048_576.0,
                            duration
                        )));
                    }
                    let kind = JobKind::Encode {
                        encoder,
                        video_bitrate,
                        target_bytes: Some(target_bytes),
                        scale: format.filter(|f| *f != source),
                        gop: None,
                    };
                    (s.start, kind, "mp4")
                }
            };
            Ok(ExportJob {
                clip_number,
                input: info.path.clone(),
                output: output_path(&req.out_dir, &stem, clip_number, ext, exists),
                start,
                duration: s.end - start,
                audio_tracks: audio.clone(),
                kind,
            })
        })
        .collect()
}

impl ExportJob {
    /// Full ffmpeg argument list (no shell involved, so any path is safe).
    pub fn args(&self) -> Vec<String> {
        let mut a: Vec<String> = ["-hide_banner", "-nostdin", "-y", "-v", "error", "-progress", "pipe:1", "-nostats"]
            .map(String::from)
            .to_vec();
        match &self.kind {
            JobKind::Copy => {
                // a hair past the keyframe, so float noise can't pick the one before it
                a.extend(["-ss".into(), secs(self.start + 0.001)]);
                a.extend(["-i".into(), path_arg(&self.input), "-t".into(), secs(self.duration)]);
                a.extend(["-map".into(), "0:v:0".into()]);
                for t in &self.audio_tracks {
                    a.extend(["-map".into(), format!("0:a:{t}")]);
                }
                a.extend(["-c", "copy", "-avoid_negative_ts", "make_zero"].map(String::from));
            }
            JobKind::Encode { encoder, video_bitrate, scale, gop, .. } => {
                a.extend(["-ss".into(), secs(self.start), "-i".into(), path_arg(&self.input), "-t".into(), secs(self.duration)]);
                let mut graph: Vec<String> = Vec::new();
                let video_map = match scale {
                    Some(f) => {
                        graph.push(format!("[0:v:0]scale={}:{}:flags=lanczos,fps={}[vout]", f.width, f.height, fps(f.fps)));
                        "[vout]".to_string()
                    }
                    None => "0:v:0".to_string(),
                };
                let audio_map = match self.audio_tracks.as_slice() {
                    [] => None,
                    [one] => Some(format!("0:a:{one}")),
                    many => {
                        let inputs: String = many.iter().map(|t| format!("[0:a:{t}]")).collect();
                        graph.push(format!("{inputs}amix=inputs={}:normalize=0[aout]", many.len()));
                        Some("[aout]".to_string())
                    }
                };
                if !graph.is_empty() {
                    a.extend(["-filter_complex".into(), graph.join(";")]);
                }
                a.extend(["-map".into(), video_map]);
                a.extend(encoder.video_args(*video_bitrate, *gop));
                match audio_map {
                    Some(m) => {
                        a.extend(["-map".into(), m]);
                        a.extend(["-c:a", "aac", "-b:a", "160k", "-ac", "2"].map(String::from));
                    }
                    None => a.push("-an".into()),
                }
            }
        }
        if self.is_mp4_family() {
            a.extend(["-movflags".into(), "+faststart".into()]);
        }
        a.push(path_arg(&self.output));
        a
    }

    fn is_mp4_family(&self) -> bool {
        let ext = self.output.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        matches!(ext.as_str(), "mp4" | "mov" | "m4v")
    }
}

fn secs(x: f64) -> String {
    format!("{x:.6}")
}

fn fps(x: f64) -> String {
    let s = format!("{x:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn path_arg(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::encoder::Encoder;
    use crate::engine::probe::{AudioTrack, ClipInfo};
    use crate::engine::quality::Format;
    use crate::engine::EngineError;
    use std::path::PathBuf;

    const MIB: u64 = 1024 * 1024;

    fn info(tracks: usize, ext: &str) -> ClipInfo {
        ClipInfo {
            path: PathBuf::from(format!("C:/clips/Replay 1.{ext}")),
            format_name: "mov,mp4,m4a,3gp,3g2,mj2".into(),
            container_ext: ext.into(),
            duration: 70.0,
            size_bytes: 200 * MIB,
            bitrate: 24_000_000,
            width: 1920,
            height: 1080,
            fps: 120.0,
            video_codec: "hevc".into(),
            audio_tracks: (0..tracks)
                .map(|i| AudioTrack { index: i, channels: 2, title: None })
                .collect(),
            keyframes: (0..70).map(|k| k as f64).collect(),
        }
    }

    fn req(sections: &[(f64, f64)], mode: Mode, audio: &[usize]) -> ExportRequest {
        ExportRequest {
            sections: sections.iter().map(|&(start, end)| Section { start, end, number: None }).collect(),
            mode,
            audio_tracks: audio.to_vec(),
            out_dir: PathBuf::from("C:/out"),
        }
    }

    fn shrink(mb: u64) -> Mode {
        Mode::Shrink { target_bytes: mb * MIB, format: None }
    }

    fn jobs(i: &ClipInfo, r: &ExportRequest) -> Vec<ExportJob> {
        plan(i, r, Encoder::Nvenc, &|_| false).unwrap()
    }

    fn pair(args: &[String], key: &str, val: &str) -> bool {
        args.windows(2).any(|w| w[0] == key && w[1] == val)
    }

    fn graph(args: &[String]) -> String {
        args.iter().skip_while(|x| *x != "-filter_complex").nth(1).cloned().unwrap_or_default()
    }

    fn invalid(r: std::result::Result<Vec<ExportJob>, EngineError>) -> String {
        match r {
            Err(EngineError::InvalidRequest(m)) => m,
            other => panic!("expected InvalidRequest, got {other:?}"),
        }
    }

    #[test]
    fn clips_are_numbered_in_timeline_order() {
        let j = jobs(&info(1, "mp4"), &req(&[(40.0, 50.0), (5.0, 9.0)], Mode::Original, &[0]));
        assert_eq!(j[0].clip_number, 1);
        assert!((j[0].start - 5.0).abs() < 1e-9);
        assert_eq!(j[1].clip_number, 2);
        assert!(j[1].output.ends_with("Replay 1 - clip 2.mp4"));
    }

    #[test]
    fn sections_are_clamped_to_the_clip() {
        let j = jobs(&info(1, "mp4"), &req(&[(-3.0, 5.0), (65.0, 99.0)], shrink(25), &[0]));
        assert_eq!(j[0].start, 0.0);
        assert!((j[0].duration - 5.0).abs() < 1e-9);
        assert!((j[1].duration - 5.0).abs() < 1e-9);
    }

    #[test]
    fn empty_or_inverted_sections_are_rejected() {
        let i = info(1, "mp4");
        invalid(plan(&i, &req(&[(10.0, 10.0)], Mode::Original, &[]), Encoder::Nvenc, &|_| false));
        invalid(plan(&i, &req(&[(20.0, 10.0)], Mode::Original, &[]), Encoder::Nvenc, &|_| false));
        invalid(plan(&i, &req(&[(80.0, 90.0)], Mode::Original, &[]), Encoder::Nvenc, &|_| false));
    }

    #[test]
    fn no_sections_is_rejected() {
        invalid(plan(&info(1, "mp4"), &req(&[], Mode::Original, &[]), Encoder::Nvenc, &|_| false));
    }

    #[test]
    fn actual_start_is_keyframe_at_or_before() {
        let k: Vec<f64> = (0..10).map(|x| x as f64).collect();
        assert_eq!(actual_start(&k, 9.5), 9.0);
        assert_eq!(actual_start(&k, 3.0), 3.0);
        assert_eq!(actual_start(&k, 0.2), 0.0);
        assert_eq!(actual_start(&[], 5.0), 0.0);
        // float noise just under a keyframe still counts as that keyframe
        assert_eq!(actual_start(&k, 2.9999999), 3.0);
    }

    #[test]
    fn original_starts_at_keyframe_and_keeps_length_to_end() {
        let j = jobs(&info(1, "mp4"), &req(&[(12.4, 20.0)], Mode::Original, &[0]));
        assert_eq!(j[0].start, 12.0);
        assert!((j[0].duration - 8.0).abs() < 1e-9);
        assert_eq!(j[0].kind, JobKind::Copy);
    }

    #[test]
    fn original_keeps_container_and_maps_tracks_separately() {
        let j = jobs(&info(3, "mkv"), &req(&[(1.0, 5.0)], Mode::Original, &[2, 1]));
        let a = j[0].args();
        assert!(j[0].output.ends_with("Replay 1 - clip 1.mkv"));
        assert!(pair(&a, "-c", "copy"));
        assert!(pair(&a, "-map", "0:a:1") && pair(&a, "-map", "0:a:2"));
        assert!(!a.contains(&"0:a:0".to_string()));
        assert!(!a.contains(&"-movflags".to_string()), "faststart is mp4-only");
    }

    #[test]
    fn original_seeks_just_past_the_keyframe() {
        let a = jobs(&info(0, "mp4"), &req(&[(12.4, 20.0)], Mode::Original, &[]))[0].args();
        assert!(pair(&a, "-ss", "12.001000"));
        assert!(pair(&a, "-t", "8.000000"));
        assert!(pair(&a, "-movflags", "+faststart"));
    }

    #[test]
    fn shrink_is_exact_mp4_with_target_bitrate() {
        let i = info(1, "mkv");
        let j = jobs(&i, &req(&[(12.4, 22.4)], shrink(25), &[0]));
        assert!((j[0].start - 12.4).abs() < 1e-9);
        assert!(j[0].output.ends_with("Replay 1 - clip 1.mp4"));
        let expected = crate::engine::quality::video_bitrate(25 * MIB, 10.0, true);
        match &j[0].kind {
            JobKind::Encode { video_bitrate, target_bytes, encoder, .. } => {
                assert_eq!(*video_bitrate, expected);
                assert_eq!(*target_bytes, Some(25 * MIB));
                assert_eq!(*encoder, Encoder::Nvenc);
            }
            k => panic!("{k:?}"),
        }
        let a = j[0].args();
        assert!(pair(&a, "-b:v", &expected.to_string()));
        assert!(pair(&a, "-c:a", "aac") && pair(&a, "-b:a", "160k"));
        assert!(pair(&a, "-movflags", "+faststart"));
    }

    #[test]
    fn shrink_mixes_two_tracks_into_one() {
        let a = jobs(&info(3, "mp4"), &req(&[(0.0, 10.0)], shrink(25), &[1, 2]))[0].args();
        let g = graph(&a);
        assert!(g.contains("[0:a:1][0:a:2]amix=inputs=2"), "{g}");
        assert!(pair(&a, "-map", "[aout]"));
    }

    #[test]
    fn shrink_single_track_needs_no_mix() {
        let a = jobs(&info(3, "mp4"), &req(&[(0.0, 10.0)], shrink(25), &[2]))[0].args();
        assert!(!a.contains(&"-filter_complex".to_string()));
        assert!(pair(&a, "-map", "0:a:2"));
    }

    #[test]
    fn no_audio_selected_drops_audio() {
        let a = jobs(&info(2, "mp4"), &req(&[(0.0, 10.0)], shrink(25), &[]))[0].args();
        assert!(a.contains(&"-an".to_string()));
        assert!(!a.iter().any(|x| x.starts_with("0:a")));
    }

    #[test]
    fn format_override_scales_and_changes_fps() {
        let mode = Mode::Shrink {
            target_bytes: 10 * MIB,
            format: Some(Format { width: 1280, height: 720, fps: 60.0 }),
        };
        let a = jobs(&info(1, "mp4"), &req(&[(0.0, 30.0)], mode, &[0]))[0].args();
        let g = graph(&a);
        assert!(g.contains("scale=1280:720"), "{g}");
        assert!(g.contains("fps=60"), "{g}");
        assert!(pair(&a, "-map", "[vout]"));
    }

    #[test]
    fn audio_index_out_of_range_is_rejected() {
        let m = invalid(plan(&info(2, "mp4"), &req(&[(0.0, 5.0)], Mode::Original, &[2]), Encoder::Nvenc, &|_| false));
        assert!(m.contains("track"), "{m}");
    }

    #[test]
    fn impossible_target_is_rejected() {
        // 1 MB for 60 s with audio leaves less than 100 kbps of video
        let m = invalid(plan(&info(1, "mp4"), &req(&[(0.0, 60.0)], shrink(1), &[0]), Encoder::Nvenc, &|_| false));
        assert!(m.contains("too small"), "{m}");
    }

    #[test]
    fn existing_files_are_never_overwritten() {
        let taken = PathBuf::from("C:/out/Replay 1 - clip 1.mp4");
        let j = plan(&info(1, "mp4"), &req(&[(0.0, 5.0)], Mode::Original, &[0]), Encoder::Nvenc, &|p| p == taken.as_path()).unwrap();
        assert!(j[0].output.ends_with("Replay 1 - clip 1 (2).mp4"));
    }

    #[test]
    fn every_job_reports_progress_and_never_prompts() {
        for mode in [Mode::Original, shrink(25)] {
            let a = jobs(&info(1, "mp4"), &req(&[(0.0, 5.0)], mode, &[0]))[0].args();
            assert!(pair(&a, "-progress", "pipe:1"));
            assert!(a.contains(&"-nostdin".to_string()) && a.contains(&"-y".to_string()));
            assert!(a.last().unwrap().ends_with("Replay 1 - clip 1.mp4"));
        }
    }

    #[test]
    fn shrink_target_never_exceeds_the_sections_original_size() {
        // 200 MB over 70 s: a 10 s section is ~28.6 MB in original quality
        let j = jobs(&info(1, "mp4"), &req(&[(0.0, 10.0)], shrink(100), &[0]));
        let cap = (200.0 * MIB as f64 * 10.0 / 70.0) as u64;
        match &j[0].kind {
            JobKind::Encode { target_bytes, video_bitrate, .. } => {
                assert_eq!(*target_bytes, Some(cap));
                assert_eq!(*video_bitrate, crate::engine::quality::video_bitrate(cap, 10.0, true));
            }
            k => panic!("{k:?}"),
        }
    }

    #[test]
    fn retried_sections_keep_their_clip_number() {
        let mut r = req(&[(40.0, 50.0)], Mode::Original, &[0]);
        r.sections[0].number = Some(3);
        let j = jobs(&info(1, "mp4"), &r);
        assert_eq!(j[0].clip_number, 3);
        assert!(j[0].output.ends_with("Replay 1 - clip 3.mp4"));
    }

    #[test]
    fn request_json_from_the_screen_deserializes() {
        let r: ExportRequest = serde_json::from_str(
            r#"{"sections":[{"start":1,"end":2}],"mode":{"kind":"shrink","targetBytes":1048576,"format":null},"audioTracks":[0],"outDir":"C:/out"}"#,
        )
        .unwrap();
        assert_eq!(r.mode, Mode::Shrink { target_bytes: MIB, format: None });
        let o: ExportRequest = serde_json::from_str(
            r#"{"sections":[],"mode":{"kind":"original"},"audioTracks":[],"outDir":"C:/out"}"#,
        )
        .unwrap();
        assert_eq!(o.mode, Mode::Original);
    }
}
