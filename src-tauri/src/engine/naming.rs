//! Output file names: `<stem> - clip N.<ext>`, never overwriting anything.

use std::path::{Path, PathBuf};

pub fn output_path(dir: &Path, stem: &str, clip_number: usize, ext: &str, exists: &dyn Fn(&Path) -> bool) -> PathBuf {
    let base = format!("{stem} - clip {clip_number}");
    let mut candidate = dir.join(format!("{base}.{ext}"));
    let mut n = 2;
    while exists(&candidate) {
        candidate = dir.join(format!("{base} ({n}).{ext}"));
        n += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn plain_name_when_free() {
        let p = output_path(Path::new("C:/out"), "Replay 1", 2, "mp4", &|_| false);
        assert_eq!(p, PathBuf::from("C:/out/Replay 1 - clip 2.mp4"));
    }

    #[test]
    fn adds_counter_until_free() {
        let taken = [
            PathBuf::from("C:/out/Replay 1 - clip 1.mkv"),
            PathBuf::from("C:/out/Replay 1 - clip 1 (2).mkv"),
        ];
        let p = output_path(Path::new("C:/out"), "Replay 1", 1, "mkv", &|p| taken.contains(&p.to_path_buf()));
        assert_eq!(p, PathBuf::from("C:/out/Replay 1 - clip 1 (3).mkv"));
    }
}
