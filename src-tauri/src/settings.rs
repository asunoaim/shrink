//! What the user picked on the settings page, stored as settings.json in the app config dir.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SaveTo {
    #[default]
    Ask,
    NextToOriginal,
    Folder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioDefault {
    #[default]
    First,
    All,
    Last,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StartMode {
    #[default]
    Original,
    Shrink,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub save_to: SaveTo,
    /// The folder for `SaveTo::Folder`.
    pub folder: Option<PathBuf>,
    pub copy_to_clipboard: bool,
    pub audio: AudioDefault,
    /// Audio tracks of the last export, for `AudioDefault::Last`.
    pub last_audio: Vec<usize>,
    pub start_mode: StartMode,
    pub target_mb: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            save_to: SaveTo::Ask,
            folder: None,
            copy_to_clipboard: true,
            audio: AudioDefault::First,
            last_audio: vec![0],
            start_mode: StartMode::Original,
            target_mb: 25.0,
        }
    }
}

/// Defaults for anything missing; a file that can't be read at all is kept as `.bad`.
pub fn load(path: &Path) -> Settings {
    let Ok(bytes) = std::fs::read(path) else {
        return Settings::default();
    };
    match serde_json::from_slice::<Settings>(&bytes) {
        Ok(mut s) => {
            s.target_mb = if s.target_mb.is_finite() { s.target_mb.clamp(1.0, 1000.0) } else { 25.0 };
            s
        }
        Err(_) => {
            let _ = std::fs::rename(path, path.with_extension("json.bad"));
            Settings::default()
        }
    }
}

/// Write to a temp file first, so a crash mid-write never leaves half a file.
pub fn save(path: &Path, s: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(s)?)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {

    fn dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn missing_file_gives_defaults() {
        let d = dir();
        let s = super::load(&d.path().join("settings.json"));
        assert_eq!(s, super::Settings::default());
        assert!(s.copy_to_clipboard);
        assert_eq!(s.save_to, super::SaveTo::Ask);
        assert_eq!(s.target_mb, 25.0);
    }

    #[test]
    fn roundtrip_keeps_every_field() {
        let d = dir();
        let p = d.path().join("settings.json");
        let s = super::Settings {
            save_to: super::SaveTo::Folder,
            folder: Some(std::path::PathBuf::from(r"D:\clips")),
            copy_to_clipboard: false,
            audio: super::AudioDefault::Last,
            last_audio: vec![0, 2],
            start_mode: super::StartMode::Shrink,
            target_mb: 8.5,
        };
        super::save(&p, &s).unwrap();
        assert_eq!(super::load(&p), s);
        assert!(!d.path().join("settings.json.tmp").exists(), "temp file is renamed away");
    }

    #[test]
    fn missing_and_unknown_fields_are_fine() {
        let d = dir();
        let p = d.path().join("settings.json");
        std::fs::write(&p, r#"{"saveTo":"nextToOriginal","someFutureThing":42}"#).unwrap();
        let s = super::load(&p);
        assert_eq!(s.save_to, super::SaveTo::NextToOriginal);
        assert!(s.copy_to_clipboard);
        assert_eq!(s.audio, super::AudioDefault::First);
    }

    #[test]
    fn corrupt_file_gives_defaults_and_is_kept_aside() {
        let d = dir();
        let p = d.path().join("settings.json");
        std::fs::write(&p, "{ not json").unwrap();
        assert_eq!(super::load(&p), super::Settings::default());
        assert!(d.path().join("settings.json.bad").is_file());
    }

    #[test]
    fn non_utf8_file_is_kept_aside_too() {
        let d = dir();
        let p = d.path().join("settings.json");
        std::fs::write(&p, [0xff, 0xfe, 0x00, 0x7b]).unwrap();
        assert_eq!(super::load(&p), super::Settings::default());
        assert!(d.path().join("settings.json.bad").is_file());
    }

    #[test]
    fn serializes_camel_case_for_the_screen() {
        let json = serde_json::to_string(&super::Settings::default()).unwrap();
        assert!(json.contains(r#""saveTo":"ask""#), "{json}");
        assert!(json.contains(r#""copyToClipboard":true"#), "{json}");
        assert!(json.contains(r#""startMode":"original""#), "{json}");
    }

    #[test]
    fn nonsense_target_is_repaired() {
        let d = dir();
        let p = d.path().join("settings.json");
        std::fs::write(&p, r#"{"targetMb":-3}"#).unwrap();
        assert_eq!(super::load(&p).target_mb, 1.0);
        std::fs::write(&p, r#"{"targetMb":99999}"#).unwrap();
        assert_eq!(super::load(&p).target_mb, 1000.0);
    }
}
