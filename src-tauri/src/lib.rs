pub mod clipboard;
pub mod commands;
pub mod engine;
pub mod settings;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(commands::AppState::default())
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || commands::warm_up(&handle));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_clip,
            commands::clip_keyframes,
            commands::clip_thumbs,
            commands::size_advice,
            commands::start_export,
            commands::cancel_export,
            commands::make_proxy,
            commands::initial_file,
            commands::copy_to_clipboard,
            commands::get_settings,
            commands::set_settings,
            commands::folder_exists,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
