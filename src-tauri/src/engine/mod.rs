//! The video engine: everything that talks to ffmpeg/ffprobe or decides what to ask them.
//! Pure planning code lives apart from process-running code so it can be unit-tested.

pub mod encoder;
pub mod error;
pub mod naming;
pub mod plan;
pub mod preview;
pub mod probe;
pub mod process_job;
pub mod quality;
pub mod runner;
pub mod tools;

pub use error::{EngineError, Result};
