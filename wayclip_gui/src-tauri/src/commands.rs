use serde_json::Value;
use wayclip_shared::Settings;

#[tauri::command]
pub fn update_settings(key: &str, value: Value) -> Result<(), String> {
    match Settings::update_key(key, value) {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("[TAURI]: Failed to update settings: {}", &e);
            Err("[TAURI]: Error updating settings: ".to_string() + e.as_str())
        }
    }
}

#[tauri::command]
pub fn pull_settings() -> serde_json::Value {
    Settings::to_json()
}
