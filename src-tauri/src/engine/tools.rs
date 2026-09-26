use super::{EngineError, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Tools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

#[derive(Debug, Clone, Copy)]
pub enum Tool {
    Ffmpeg,
    Ffprobe,
}

impl Tools {
    /// Search order: `SHRINK_FFMPEG_DIR`, the bundled copy in `<exe dir>/ffmpeg`,
    /// the exe's own folder, `PATH`, then WinGet's Gyan.FFmpeg install.
    pub fn locate() -> Result<Self> {
        let dirs = candidate_dirs();
        Self::locate_in(&dirs).ok_or_else(|| {
            EngineError::ToolNotFound(format!("searched {} folders", dirs.len()))
        })
    }

    /// First directory that contains both executables.
    pub fn locate_in(dirs: &[PathBuf]) -> Option<Self> {
        dirs.iter().find_map(|d| {
            let ffmpeg = d.join(exe("ffmpeg"));
            let ffprobe = d.join(exe("ffprobe"));
            (ffmpeg.is_file() && ffprobe.is_file()).then_some(Tools { ffmpeg, ffprobe })
        })
    }

    /// A command for the tool that never flashes a console window.
    pub fn cmd(&self, tool: Tool) -> Command {
        let path: &Path = match tool {
            Tool::Ffmpeg => &self.ffmpeg,
            Tool::Ffprobe => &self.ffprobe,
        };
        #[allow(unused_mut)]
        let mut c = Command::new(path);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            c.creation_flags(CREATE_NO_WINDOW);
        }
        c
    }
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(d) = std::env::var_os("SHRINK_FFMPEG_DIR") {
        dirs.push(PathBuf::from(d));
    }
    if let Some(d) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        dirs.push(d.join("ffmpeg")); // where the installer puts the bundled copy
        dirs.push(d);
    }
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        dirs.extend(winget_ffmpeg_dirs(&PathBuf::from(local).join("Microsoft/WinGet/Packages")));
    }
    dirs
}

/// `<packages>/Gyan.FFmpeg*/<build>/bin`
fn winget_ffmpeg_dirs(packages: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(packages) else { return out };
    for pkg in entries.flatten() {
        if !pkg.file_name().to_string_lossy().starts_with("Gyan.FFmpeg") {
            continue;
        }
        if let Ok(builds) = std::fs::read_dir(pkg.path()) {
            out.extend(builds.flatten().map(|b| b.path().join("bin")));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_next_to_the_app_and_in_its_ffmpeg_folder() {
        // the installer puts the bundled copy in <install dir>/ffmpeg
        let exe_dir = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
        let dirs = candidate_dirs();
        let i = dirs.iter().position(|d| *d == exe_dir.join("ffmpeg")).expect("bundled dir");
        assert_eq!(dirs[i + 1], exe_dir, "bundled copy wins over loose files next to the exe");
    }
}
