use chrono::{DateTime, Local};
use dirs::{config_dir, home_dir};
use mp4::Mp4Reader;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::io::Cursor;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use wayclip_shared::{log, Settings};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tag {
    name: String,
    color: String,
}

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

#[derive(Serialize)]
pub struct ClipData {
    name: String,
    path: String,
    length: f64,
    size: u64,
    created_at: DateTime<Local>,
    updated_at: DateTime<Local>,
    tags: Vec<Tag>,
    liked: bool,
}

#[derive(Deserialize, Clone)]
struct ClipJsonData {
    #[serde(default)]
    tags: Vec<Tag>,
    #[serde(default)]
    liked: bool,
}

#[tauri::command(async)]
pub async fn pull_clips() -> Vec<ClipData> {
    let settings = Settings::load();
    let home_dir = home_dir().unwrap_or_else(|| PathBuf::from("."));
    let json_path = config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("wayclip")
        .join("data.json");
    let clips_dir_path = home_dir.join(&settings.save_path_from_home_string);

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

async fn get_video_duration(path: &PathBuf) -> Result<f64, Box<dyn std::error::Error>> {
    let file_bytes = fs::read(path).await?;
    let size = file_bytes.len() as u64;
    let reader = Cursor::new(file_bytes);
    let mp4 = Mp4Reader::read_header(reader, size)?;

    let duration = mp4.moov.mvhd.duration;
    let timescale = mp4.moov.mvhd.timescale;

    if timescale > 0 {
        Ok(duration as f64 / timescale as f64)
    } else {
        Ok(0.0)
    }
}

async fn write_json_data(path: &PathBuf, data: &Value) -> Result<(), String> {
    let content =
        serde_json::to_string_pretty(data).map_err(|e| format!("Failed to serialize JSON: {e}"))?;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)
        .await
        .map_err(|e| format!("Failed to open {} for writing: {}", path.display(), e))?;

    file.write_all(content.as_bytes())
        .await
        .map_err(|e| format!("Failed to write to {}: {}", path.display(), e))?;

    Ok(())
}
