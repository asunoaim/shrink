//! Putting finished files on the clipboard, so Ctrl+V in Discord or Explorer pastes them.

use std::path::PathBuf;

#[cfg(windows)]
pub fn copy_files(paths: &[PathBuf]) -> Result<(), String> {
    let list: Vec<String> = paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    let _clip = clipboard_win::Clipboard::new_attempts(10).map_err(|e| e.to_string())?;
    clipboard_win::raw::empty().map_err(|e| e.to_string())?;
    clipboard_win::raw::set_file_list(&list).map_err(|e| e.to_string())
}

#[cfg(not(windows))]
pub fn copy_files(_paths: &[PathBuf]) -> Result<(), String> {
    Err("copying files to the clipboard is only supported on Windows".into())
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn copied_files_can_be_pasted_back() {
        let a = std::env::current_dir().unwrap().join("Cargo.toml");
        let b = std::env::current_dir().unwrap().join("build.rs");
        super::copy_files(&[a.clone(), b.clone()]).unwrap();
        let _clip = clipboard_win::Clipboard::new_attempts(10).unwrap();
        let mut got = Vec::new();
        clipboard_win::raw::get_file_list(&mut got).unwrap();
        assert_eq!(got, vec![a.to_string_lossy().into_owned(), b.to_string_lossy().into_owned()]);
    }
}
