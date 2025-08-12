use dirs::{config_dir, home_dir};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{create_dir_all, write};
use std::path::PathBuf;

pub const DAEMON: &str = "\x1b[35m[daemon]\x1b[0m"; // magenta
pub const UNIX: &str = "\x1b[36m[unix]\x1b[0m"; // cyan
pub const ASH: &str = "\x1b[34m[ashpd]\x1b[0m"; // blue
pub const GST: &str = "\x1b[32m[gst]\x1b[0m"; // green
pub const RING: &str = "\x1b[33m[ring]\x1b[0m"; // yellow
pub const HYPR: &str = "\x1b[31m[hypr]\x1b[0m"; // red
pub const FFMPEG: &str = "\x1b[95m[ffmpeg]\x1b[0m"; // pink
pub const TAURI: &str = "\x1b[90m[tauri]\x1b[0m"; // gray
pub const GSTBUS: &str = "\x1b[94m[gst-bus]\x1b[0m"; // bright blue
pub const CLEANUP: &str = "\x1b[92m[cleanup]\x1b[0m"; // bright green
pub const DEBUG: &str = "\x1b[93m[debug]\x1b[0m";

pub mod logging;

pub const WAYCLIP_TRIGGER_PATH: &str =
    "/home/kony/Documents/GitHub/wayclip/wayclip_trigger/trigger_launcher.sh";

#[macro_export]
macro_rules! log_to {
    ($logger:expr, $level:ident, [$tag:ident] => $($arg:tt)*) => {
        {
            $logger.log(
                $crate::logging::LogLevel::$level,
                $crate::$tag,
                &format!($($arg)*),
            );
        }
    };
}

#[macro_export]
macro_rules! log {
    ([$tag:ident] => $($arg:tt)*) => {
        println!("{} {}", $crate::$tag, format!($($arg)*));
    };
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum JsonValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
    Object(JsonObject),
    Array(JsonArray),
}

pub type JsonArray = Vec<JsonValue>;
pub type JsonObject = HashMap<String, JsonValue>;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct Settings {
    pub clip_name_formatting: String,
    pub clip_length_s: u64,
    pub clip_resolution: String,
    pub clip_fps: u16,
    pub include_desktop_audio: bool,
    pub include_mic_audio: bool,
    pub video_bitrate: u16,
    pub video_codec: String,
    pub audio_codec: String,
    pub save_path_from_home_string: String,
    pub save_shortcut: String,
    pub open_gui_shortcut: String,
    pub toggle_notifications: bool,
    pub daemon_socket_path: String,
    pub gui_socket_path: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            clip_name_formatting: String::from("%Y-%m-%d_%H-%M-%S"), // done
            clip_length_s: 120,                                      // done
            clip_resolution: String::from("1920x1080"),              // needs work
            clip_fps: 60,                                            // prob should remove / or fix
            include_desktop_audio: true,                             // done
            include_mic_audio: true,                                 // done
            video_bitrate: 15000,                                    // done
            video_codec: String::from("h264"),                       // needs work
            audio_codec: String::from("aac"),                        // needs work
            save_path_from_home_string: String::from("Videos/wayclip"), // done
            save_shortcut: String::from("Alt+C"),                    // needs work
            open_gui_shortcut: String::from("Ctrl+Alt+C"),           // needs work
            toggle_notifications: true,                              // should remove / remake
            daemon_socket_path: String::from("/tmp/wayclipd.sock"),  // done
            gui_socket_path: String::from("/tmp/wayclipg.sock"),     // done
        }
    }
}

impl Settings {
    // TODO: Rewrite to return actual config dir and home dir path, later to be used in GUI
    pub fn config_path() -> PathBuf {
        config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("wayclip")
            .join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        log!([DEBUG] => "Attempting to read settings from: {:?}", path);

        match std::fs::read_to_string(&path) {
            Ok(data) => match serde_json::from_str(&data) {
                Ok(settings) => settings,
                Err(e) => {
                    log!([DEBUG] => "FATAL: Could not parse settings.json. The file is invalid. Error: {}", e);
                    panic!()
                }
            },
            Err(_) => {
                log!([DEBUG] => "No settings file found at {:?}, creating default settings.", path);
                Default::default()
            }
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = create_dir_all(parent);
        }
        let _ = write(&path, serde_json::to_string_pretty(self).unwrap());
    }

    pub fn update_key(key: &str, value: Value) -> Result<(), String> {
        let mut settings = Self::load();
        match key {
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
            "include_desktop_audio" => {
                settings.include_desktop_audio = Self::get_bool(&value)?;
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
            "gui_socket_path" => settings.gui_socket_path = Self::get_str(&value)?,
            "daemon_socket_path" => settings.daemon_socket_path = Self::get_str(&value)?,

            _ => return Err("Invalid key has been used!".into()),
        }
        settings.save();
        Ok(())
    }

    fn get_str(value: &Value) -> Result<String, String> {
        value
            .as_str()
            .map(|s| s.to_string())
            .ok_or("expected string".into())
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

        let home_dir = home_dir().ok_or_else(|| "home dir not found".to_string())?;
        let clean_path = rel_path.trim_start_matches('/');
        if clean_path.starts_with("home/") {
            return Ok(clean_path.to_string());
        }
        let full_path = home_dir.join(clean_path);

        Ok(full_path.to_string_lossy().into_owned())
    }

    pub fn to_json() -> serde_json::Value {
        serde_json::to_value(Self::load()).unwrap()
    }
}
