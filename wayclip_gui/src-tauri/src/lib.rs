use std::sync::Mutex;
use tauri::{Manager, State};
use wayclip_shared::Settings;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let settings = Settings::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(settings))
        .invoke_handler(tauri::generate_handler![
            commands::update_settings,
            commands::pull_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
