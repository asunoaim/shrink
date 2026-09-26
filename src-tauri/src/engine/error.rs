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

/// The screen only needs the message.
impl Serialize for EngineError {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, EngineError>;
