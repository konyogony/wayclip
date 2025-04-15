use serde_json::Value;
use wayclip_shared::{err, log, Settings};

#[tauri::command]
pub fn update_settings(key: &str, value: Value) -> Result<(), String> {
    match Settings::update_key(key, value) {
        Ok(_) => Ok(()),
        Err(e) => {
            log!([TAURI] => "Failed to update settings: {}", &e);
            Err(err!([TAURI] => "Failed to update settings: {}", &e).to_string())
        }
    }
}

#[tauri::command]
pub fn pull_settings() -> serde_json::Value {
    Settings::to_json()
}
