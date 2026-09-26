pub mod clipboard;
pub mod commands;
pub mod engine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::open_clip,
            commands::size_advice,
            commands::start_export,
            commands::cancel_export,
            commands::make_proxy,
            commands::initial_file,
            commands::copy_to_clipboard,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
