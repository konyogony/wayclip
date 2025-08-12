use crate::types::{ClipData, ClipJsonData};
use crate::{get_video_duration, write_json_data};
use chrono::{DateTime, Local};
use serde_json::{json, Value};
use tokio::fs;
use wayclip_core::{log, Settings};

#[tauri::command]
pub fn update_settings(key: &str, value: Value) -> Result<(), String> {
    match Settings::update_key(key, value) {
        Ok(_) => Ok(()),
        Err(e) => {
            let err_msg = format!("Failed to update settings: {}", &e);
            log!([TAURI] => "{}", &err_msg);
            Err(err_msg)
        }
    }
}

#[tauri::command]
pub fn pull_settings() -> serde_json::Value {
    Settings::to_json()
}

#[tauri::command(async)]
pub async fn pull_clips() -> Vec<ClipData> {
    let settings = Settings::load();
    let clips_dir_path = Settings::home_path().join(&settings.save_path_from_home_string);
    let json_path = Settings::config_path().join("wayclip").join("data.json");

    let mut data: Value = if json_path.exists() {
        let contents = match fs::read_to_string(&json_path).await {
            Ok(c) => c,
            Err(e) => {
                log!([TAURI] => "Failed to read data.json: {}", e);
                return Vec::new();
            }
        };
        serde_json::from_str(&contents).unwrap_or_else(|e| {
            log!([TAURI] => "Failed to parse data.json: {}. A new one will be created.", e);
            json!({})
        })
    } else {
        json!({})
    };

    let mut clips: Vec<ClipData> = Vec::new();
    let mut data_modified = false;

    let mut dir = match fs::read_dir(&clips_dir_path).await {
        Ok(d) => d,
        Err(e) => {
            log!([TAURI] => "Failed to read directory {}: {}", clips_dir_path.display(), e);
            return Vec::new();
        }
    };

    loop {
        let entry = match dir.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(e) => {
                log!([TAURI] => "Error reading directory entry: {}", e);
                continue;
            }
        };

        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.to_lowercase().ends_with(".mp4") {
            continue;
        }

        let metadata = match entry.metadata().await {
            Ok(m) => m,
            Err(e) => {
                log!([TAURI] => "Failed to read metadata for {}: {}", name, e);
                continue;
            }
        };

        if metadata.len() == 0 {
            continue;
        }

        if data.get(&name).is_none() {
            data_modified = true;
            if let Some(obj) = data.as_object_mut() {
                obj.insert(name.clone(), json!({ "tags": [], "liked": false }));
            }
        }

        let clip_json_value = data.get(&name).cloned().unwrap_or_default();
        let clip_specific_data: ClipJsonData = serde_json::from_value(clip_json_value)
            .unwrap_or_else(|e| {
                log!([TAURI] => "Couldn't parse JSON for {}: {}. Using default.", name, e);
                ClipJsonData {
                    tags: Vec::new(),
                    liked: false,
                }
            });

        let created_at: DateTime<Local> = metadata
            .created()
            .map(Into::into)
            .unwrap_or_else(|_| Local::now());
        let updated_at: DateTime<Local> = metadata
            .modified()
            .map(Into::into)
            .unwrap_or_else(|_| Local::now());
        let size = metadata.len();

        let length = match get_video_duration(&path).await {
            Ok(duration) => duration,
            Err(e) => {
                log!([TAURI] => "Failed to get duration for {}: {}", name, e);
                continue;
            }
        };

        clips.push(ClipData {
            name: name
                .strip_suffix(".mp4")
                .unwrap_or(name.as_str())
                .to_string(),
            path: path.to_string_lossy().into_owned(),
            length,
            size,
            created_at,
            updated_at,
            tags: clip_specific_data.tags,
            liked: clip_specific_data.liked,
        });
    }

    if data_modified {
        if let Err(e) = write_json_data(&json_path, &data).await {
            log!([TAURI] => "{}", e);
        }
    }

    clips
}
