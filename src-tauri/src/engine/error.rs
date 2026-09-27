use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("ffmpeg/ffprobe not found ({0})")]
    ToolNotFound(String),
    #[error("can't read this file: {0}")]
    Probe(String),
    #[error("{0}")]
    InvalidRequest(String),
    #[error("ffmpeg failed (exit code {code:?}): {log}")]
    Ffmpeg { code: Option<i32>, log: String },
    #[error("cancelled")]
    Cancelled,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl EngineError {
    /// One plain sentence for the screen. `to_string()` keeps the technical detail.
    pub fn user_message(&self) -> String {
        use std::io::ErrorKind;
        match self {
            EngineError::ToolNotFound(_) => "shrink can't find ffmpeg. Reinstalling shrink fixes this.".into(),
            EngineError::Probe(m) if m.starts_with("file not found") => {
                "That file isn't there anymore. It may have been moved or deleted.".into()
            }
            EngineError::Probe(_) => "This file doesn't look like a video shrink can read.".into(),
            EngineError::InvalidRequest(m) => sentence(m),
            EngineError::Ffmpeg { .. } => "The encoder stopped with an error on this clip.".into(),
            EngineError::Cancelled => "Cancelled.".into(),
            EngineError::Io(e) => match e.kind() {
                ErrorKind::PermissionDenied => "Windows didn't allow shrink to write there. Try another folder.".into(),
                ErrorKind::NotFound => "A file or folder shrink needed is missing.".into(),
                ErrorKind::StorageFull => "The disk is full.".into(),
                _ => "Couldn't read or write a file.".into(),
            },
        }
    }
}

/// "clip 1 already exists" → "Clip 1 already exists."
fn sentence(m: &str) -> String {
    let mut c = m.trim().chars();
    let mut s = match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect::<String>(),
        None => return "Something went wrong.".into(),
    };
    if !s.ends_with(['.', '!', '?']) {
        s.push('.');
    }
    s
}

/// The screen gets the plain sentence.
impl Serialize for EngineError {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.user_message())
    }
}

pub type Result<T> = std::result::Result<T, EngineError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error, ErrorKind};

    #[test]
    fn every_error_has_a_plain_sentence() {
        let cases = [
            EngineError::ToolNotFound("x".into()),
            EngineError::Probe("Invalid data found when processing input".into()),
            EngineError::Probe("file not found: C:/a.mp4".into()),
            EngineError::InvalidRequest("mark at least one section".into()),
            EngineError::Ffmpeg { code: Some(1), log: "Conversion failed!".into() },
            EngineError::Cancelled,
            EngineError::Io(Error::new(ErrorKind::PermissionDenied, "denied")),
            EngineError::Io(Error::new(ErrorKind::NotFound, "gone")),
            EngineError::Io(Error::other("weird")),
        ];
        for e in cases {
            let m = e.user_message();
            assert!(m.ends_with('.'), "{m}");
            // the brand is written lowercase; every other sentence starts with a capital
            assert!(m.starts_with("shrink") || m.chars().next().unwrap().is_uppercase(), "{m}");
            assert!(!m.contains("Conversion failed"), "raw ffmpeg log leaked: {m}");
            assert!(!m.contains("weird"), "raw OS error text leaked: {m}");
        }
    }

    #[test]
    fn missing_file_is_not_called_unreadable() {
        let m = EngineError::Probe("file not found: C:/a.mp4".into()).user_message();
        assert!(m.contains("moved or deleted"), "{m}");
    }

    #[test]
    fn requests_keep_their_reason() {
        let m = EngineError::InvalidRequest("clip 1.mp4 already exists".into()).user_message();
        assert_eq!(m, "Clip 1.mp4 already exists.");
    }

    #[test]
    fn serializes_as_the_sentence() {
        let json = serde_json::to_string(&EngineError::Cancelled).unwrap();
        assert_eq!(json, r#""Cancelled.""#);
    }
}
