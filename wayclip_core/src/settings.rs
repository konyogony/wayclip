use crate::config_dir;
use crate::get_default_audio_devices;
use crate::home_dir;
use crate::log;
use crate::PathBuf;
use crate::Value;
use anyhow::{Context, Result};
use std::collections::HashSet;
use tokio::fs;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Settings {
    pub api_url: String,
    pub auth_token: Option<String>,
    pub clip_name_formatting: String,
    pub clip_length_s: u64,
    pub clip_resolution: String,
    pub clip_fps: u16,
    pub video_bitrate: u16,
    pub video_codec: String,
    pub audio_codec: String,
    pub save_path_from_home_string: String,
    pub save_shortcut: String,
    pub open_gui_shortcut: String,
    pub toggle_notifications: bool,
    pub daemon_pid_path: String,
    pub daemon_socket_path: String,
    pub gui_socket_path: String,
    pub mic_node_name: String,
    pub bg_node_name: String,
    pub mic_volume: u8,
    pub bg_volume: u8,
    pub include_mic_audio: bool,
    pub include_bg_audio: bool,
}

impl Settings {
    pub async fn new() -> Result<Self> {
        let (default_source, default_sink) = get_default_audio_devices().await.unwrap_or_default();
        Ok(Self {
            api_url: String::from("http://127.0.0.1:8080"),
            auth_token: None,
            mic_node_name: default_source.unwrap_or_default(),
            bg_node_name: default_sink.unwrap_or_default(),
            clip_name_formatting: String::from("%Y-%m-%d_%H-%M-%S"),
            clip_length_s: 120,
            clip_resolution: String::from("1920x1080"),
            clip_fps: 60,
            video_bitrate: 15000,
            video_codec: String::from("h264"),
            audio_codec: String::from("aac"),
            save_path_from_home_string: String::from("Videos/wayclip"),
            save_shortcut: String::from("Alt+C"),
            open_gui_shortcut: String::from("Ctrl+Alt+C"),
            toggle_notifications: true,
            daemon_pid_path: String::from("/tmp/wayclipd.pid"),
            daemon_socket_path: String::from("/tmp/wayclipd.sock"),
            gui_socket_path: String::from("/tmp/wayclipg.sock"),
            mic_volume: 100,
            bg_volume: 75,
            include_mic_audio: true,
            include_bg_audio: true,
        })
    }

    pub fn config_path() -> PathBuf {
        config_dir().unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn home_path() -> PathBuf {
        home_dir().unwrap_or_else(|| PathBuf::from("."))
    }

    async fn create_and_save_new() -> Result<Self> {
        // log!([DEBUG] => "Creating and saving new default settings.");
        let settings = Self::new().await?;
        settings
            .save()
            .await
            .context("Failed to save newly created settings")?;
        Ok(settings)
    }

    pub async fn load() -> Result<Self> {
        let path = Self::config_path().join("wayclip").join("settings.json");

        if !path.exists() {
            log!([DEBUG] => "No settings file found.");
            return Self::create_and_save_new().await;
        }

        // log!([DEBUG] => "Found settings file at: {:?}", path);
        let data = fs::read_to_string(&path)
            .await
            .context("Failed to read existing settings file")?;

        let saved_value: Value = match serde_json::from_str(&data) {
            Ok(value) => value,
            Err(e) => {
                log!([TAURI] => "WARN: Settings file is corrupt or invalid, creating a new one. Error: {}", e);
                return Self::create_and_save_new().await;
            }
        };

        let saved_map = match saved_value.as_object() {
            Some(map) => map,
            None => {
                log!([TAURI] => "WARN: Settings JSON is not an object, creating a new one.");
                return Self::create_and_save_new().await;
            }
        };

        let default_settings = Self::new().await?;
        let mut default_value = serde_json::to_value(default_settings)?;
        let default_map = default_value.as_object().unwrap();

        let saved_keys: HashSet<_> = saved_map.keys().cloned().collect();
        let default_keys: HashSet<_> = default_map.keys().cloned().collect();

        if saved_keys == default_keys {
            //     log!([DEBUG] => "Settings file is up-to-date. Loading directly.");
            return serde_json::from_value(saved_value)
                .context("Failed to deserialize up-to-date settings");
        }

        // log!([DEBUG] => "Settings file is outdated or has extra keys. Merging with defaults.");

        let default_map_mut = default_value.as_object_mut().unwrap();

        for (key, value) in saved_map {
            default_map_mut.insert(key.clone(), value.clone());
        }

        for key in saved_keys.difference(&default_keys) {
            log!([TAURI] => "WARN: Unknown key '{}' found in settings.json. It will be ignored.", key);
        }

        let final_settings: Settings = serde_json::from_value(default_value)
            .context("Failed to create final settings from merged data")?;

        final_settings
            .save()
            .await
            .context("Failed to save merged settings")?;

        log!([DEBUG] => "Successfully merged and saved updated settings.");

        Ok(final_settings)
    }

    pub async fn save(&self) -> Result<()> {
        let path = Self::config_path().join("wayclip").join("settings.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let data = serde_json::to_string_pretty(self)?;
        fs::write(&path, data).await?;
        Ok(())
    }

    pub async fn update_key(key: &str, value: Value) -> Result<(), String> {
        let mut settings = Self::load().await.map_err(|e| e.to_string())?;

        match key {
            "api_url" => {
                settings.api_url = Self::get_str(&value)?;
            }
            "auth_token" => {
                settings.auth_token = Some(Self::get_str(&value)?);
            }
            "clip_name_formatting" => {
                settings.clip_name_formatting = Self::get_str(&value)?;
            }
            "clip_length_s" => {
                settings.clip_length_s = Self::get_u64(&value)?;
            }
            "clip_resolution" => {
                settings.clip_resolution = Self::get_str(&value)?;
            }
            "clip_fps" => {
                settings.clip_fps = Self::get_u16(&value)?;
            }
            "include_bg_audio" => {
                settings.include_bg_audio = Self::get_bool(&value)?;
            }
            "include_mic_audio" => {
                settings.include_mic_audio = Self::get_bool(&value)?;
            }
            "video_bitrate" => {
                settings.video_bitrate = Self::get_u16(&value)?;
            }
            "video_codec" => {
                settings.video_codec = Self::get_str(&value)?;
            }
            "audio_codec" => {
                settings.audio_codec = Self::get_str(&value)?;
            }
            "save_path_from_home_string" => {
                settings.save_path_from_home_string = Self::get_str_valid_path(&value)?;
            }
            "save_shortcut" => {
                settings.save_shortcut = Self::get_shortcut(&value)?;
            }
            "open_gui_shortcut" => {
                settings.open_gui_shortcut = Self::get_shortcut(&value)?;
            }
            "toggle_notifications" => {
                settings.toggle_notifications = Self::get_bool(&value)?;
            }
            "daemon_pid_path" => {
                settings.daemon_pid_path = Self::get_str(&value)?;
            }
            "gui_socket_path" => {
                settings.gui_socket_path = Self::get_str(&value)?;
            }
            "daemon_socket_path" => {
                settings.daemon_socket_path = Self::get_str(&value)?;
            }
            "mic_node_name" => {
                settings.mic_node_name = Self::get_str(&value)?;
            }
            "bg_node_name" => {
                settings.bg_node_name = Self::get_str(&value)?;
            }
            "mic_volume" => {
                settings.mic_volume = Self::get_u8(&value)?;
            }
            "bg_volume" => {
                settings.bg_volume = Self::get_u8(&value)?;
            }

            _ => return Err("Invalid key has been used!".into()),
        }

        settings.save().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    fn get_str(value: &Value) -> Result<String, String> {
        value
            .as_str()
            .map(|s| s.to_string())
            .ok_or("expected string".into())
    }

    fn get_u8(value: &Value) -> Result<u8, String> {
        value
            .as_f64()
            .map(|n| n as u8)
            .ok_or("expected an u8".into())
    }

    fn get_u16(value: &Value) -> Result<u16, String> {
        value
            .as_f64()
            .map(|n| n as u16)
            .ok_or("expected an u16".into())
    }

    fn get_u64(value: &Value) -> Result<u64, String> {
        value
            .as_f64()
            .map(|n| n as u64)
            .ok_or("expected an u64".into())
    }

    fn get_bool(value: &Value) -> Result<bool, String> {
        value.as_bool().ok_or("expected a boolean".into())
    }

    fn get_shortcut(value: &Value) -> Result<String, String> {
        let raw = value
            .as_str()
            .ok_or_else(|| "expected a string for shortcut".to_string())?;

        let cleaned = raw.replace(' ', "");
        let parts: Vec<&str> = cleaned.split('+').collect();

        if parts.is_empty() {
            return Err("shortcut cannot be empty".to_string());
        }

        let allowed_modifiers = ["Ctrl", "Alt", "Shift", "Meta"];
        let mut has_non_modifier = false;

        for part in &parts {
            if allowed_modifiers.contains(part) {
                continue;
            }

            if part.len() == 1 && part.chars().all(|c| c.is_ascii_alphanumeric()) {
                if has_non_modifier {
                    return Err("only one non-modifier key allowed".to_string());
                }
                has_non_modifier = true;
            } else {
                return Err(format!("invalid key in shortcut: {part}"));
            }
        }

        if !has_non_modifier {
            return Err("missing non-modifier key (like 'A', 'Z', '1', etc)".to_string());
        }

        Ok(cleaned)
    }

    fn get_str_valid_path(value: &Value) -> Result<String, String> {
        let rel_path = value
            .as_str()
            .ok_or_else(|| "expected a string for path".to_string())?;

        let clean_path = rel_path.trim_start_matches('/');
        if clean_path.starts_with("home/") {
            return Ok(clean_path.to_string());
        }
        let full_path = Self::home_path().join(clean_path);

        Ok(full_path.to_string_lossy().into_owned())
    }

    pub async fn to_json() -> Result<serde_json::Value> {
        let settings = Self::load().await?;
        serde_json::to_value(settings).context("Failed to serialize settings to JSON")
    }
}
