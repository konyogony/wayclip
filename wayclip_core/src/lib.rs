use crate::logging::Logger;
use anyhow::{Context, Result};
use ashpd::desktop::{screencast::Screencast, Session};
use chrono::{DateTime, Local};
use dirs::{config_dir, home_dir};
use futures::stream::{FuturesUnordered, StreamExt};
use gstreamer::prelude::{Cast, ElementExt, GstObjectExt};
use mp4::Mp4Reader;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt;
use std::fs::{create_dir_all, remove_file, write, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::process::Command;
use tokio::task::JoinHandle;

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
pub const DEBUG: &str = "\x1b[93m[debug]\x1b[0m"; // idk

pub mod logging;
pub mod ring;

pub const WAYCLIP_TRIGGER_PATH: &str = "/home/kony/Documents/GitHub/wayclip/target/debug/trigger";

#[derive(Deserialize)]
pub struct PullClipsArgs {
    pub page: usize,
    pub page_size: usize,
    pub search_query: Option<String>,
    // sort_by: Option<String>,
    // sort_order: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct PaginatedClips {
    clips: Vec<ClipData>,
    total_pages: usize,
    total_clips: usize,
}

#[derive(Serialize, Clone)]
pub struct ClipData {
    pub name: String,
    pub path: String,
    pub length: f64,
    pub size: u64,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
    pub tags: Vec<Tag>,
    pub liked: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tag {
    pub name: String,
    pub color: String,
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct ClipJsonData {
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub liked: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Payload {
    pub message: String,
}

// Former shared lib related

// Logs to file & console, Ex: log_to!(logger, Warn, [TAURI] => "Message")
// If Debug is selected, logs only to file
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

// Logs only to console
#[macro_export]
macro_rules! log {
    ([$tag:ident] => $($arg:tt)*) => {
        println!("{} {}", $crate::$tag, format!($($arg)*))
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
            clip_fps: 60,                                            // done
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
    pub fn config_path() -> PathBuf {
        config_dir().unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn home_path() -> PathBuf {
        home_dir().unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn load() -> Self {
        let path = Self::config_path().join("wayclip").join("settings.json");
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
        let path = Self::config_path().join("wayclip").join("settings.json");
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

        let clean_path = rel_path.trim_start_matches('/');
        if clean_path.starts_with("home/") {
            return Ok(clean_path.to_string());
        }
        let full_path = Self::home_path().join(clean_path);

        Ok(full_path.to_string_lossy().into_owned())
    }

    pub fn to_json() -> serde_json::Value {
        serde_json::to_value(Self::load()).unwrap()
    }
}

// Recording related

pub fn send_status_to_gui(socket_path: String, message: String, logger: &Logger) {
    let logger_clone = logger.clone();
    tokio::spawn(async move {
        if let Ok(mut stream) = UnixStream::connect(socket_path).await {
            let _ = stream.write_all(format!("{message}\n").as_bytes()).await;
            let _ = stream.flush().await;
            log_to!(logger_clone, Info, [UNIX] => "Sent status '{}' to GUI", message);
        } else {
            log_to!(logger_clone, Warn, [UNIX] => "GUI not running or socket not ready. Couldn't send status.");
        }
    });
}

pub async fn handle_bus_messages(pipeline: gstreamer::Pipeline, logger: Logger) {
    let bus = pipeline.bus().unwrap();
    let mut bus_stream = bus.stream();

    log_to!(logger, Info, [GSTBUS] => "Started bus message handler.");
    while let Some(msg) = bus_stream.next().await {
        use gstreamer::MessageView;
        match msg.view() {
            MessageView::Error(err) => {
                let src_name = err
                    .src()
                    .map_or_else(|| "None".to_string(), |s| s.path_string().to_string());
                let error_msg = err.error().to_string();
                let debug_info = err.debug().map_or_else(
                    || "No debug info".to_string(),
                    |g_string| g_string.to_string(),
                );
                log_to!(logger, Error, [GSTBUS] => "Error from element {}: {} ({})", src_name, error_msg, debug_info);
                if error_msg.to_lowercase().contains("unhandled format")
                    || debug_info
                        .to_lowercase()
                        .contains("format negotiation failed")
                {
                    log_to!(logger, Warn, [GSTBUS] => "Detected format negotiation failure (PipeWire -> GStreamer). Consider allowing automatic format negotiation (remove rigid caps on pipewiresrc) or recreating the pipeline.");
                }
                break;
            }
            MessageView::Warning(warning) => {
                let src_name = warning
                    .src()
                    .map_or_else(|| "None".to_string(), |s| s.path_string().to_string());
                let error_msg = warning.error().to_string();
                let debug_info = warning.debug().map_or_else(
                    || "No debug info".to_string(),
                    |g_string| g_string.to_string(),
                );
                log_to!(logger, Warn, [GSTBUS] => "Warning from element {}: {} ({})", src_name, error_msg, debug_info);
            }
            MessageView::Eos(_) => {
                log_to!(logger, Info, [GSTBUS] => "Received End-Of-Stream");
                break;
            }
            MessageView::StateChanged(state) => {
                if state
                    .src()
                    .and_then(|s| s.downcast_ref::<gstreamer::Pipeline>())
                    .is_some()
                {
                    log_to!(logger, Debug, [GSTBUS] => "Pipeline state changed from {:?} to {:?} ({:?})", state.old(), state.current(), state.pending());
                }
            }
            _ => {}
        }
    }
    log_to!(logger, Info, [GSTBUS] => "Stopped bus message handler.");
}

pub async fn setup_hyprland(logger: &Logger) {
    let output = Command::new("hyprctl")
        .args([
            "keyword",
            "bind",
            format!("Alt_L,C,exec,{WAYCLIP_TRIGGER_PATH}").as_str(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn hyprctl")
        .wait()
        .await;
    if let Ok(output) = output {
        if output.success() {
            log_to!(*logger, Info, [HYPR] => "Bind added successfully");
        } else {
            log_to!(*logger, Error, [HYPR] => "Bind failed");
            log_to!(*logger, Error, [HYPR] => "Error: {}", output.to_string());
        }
    } else {
        log_to!(*logger, Error, [HYPR] => "Failed to add bind hyprctl");
    }
}

pub async fn cleanup(
    pipeline: &gstreamer::Element,
    session: &Session<'_, Screencast<'_>>,
    settings: Settings,
    logger: Logger,
) {
    send_status_to_gui(
        settings.gui_socket_path.clone(),
        String::from("Shuting down..."),
        &logger,
    );

    log_to!(logger, Info, [CLEANUP] => "Starting graceful shutdown...");

    if std::env::var("DESKTOP_SESSION") == Ok("hyprland".to_string()) {
        let output = Command::new("hyprctl")
            .args(["keyword", "unbind", "Alt_L,C"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn hyprctl for unbind")
            .wait()
            .await;
        if let Ok(output) = output {
            if output.success() {
                log_to!(logger, Info, [HYPR] => "Bind removed successfully");
            } else {
                log_to!(logger, Error, [HYPR] => "Failed to remove bind");
            }
        }
    }

    if let Err(e) = pipeline.set_state(gstreamer::State::Null) {
        log_to!(logger, Error, [GST] => "Failed to set pipeline to null, {:?}", e);
    } else {
        log_to!(logger, Info, [GST] => "Pipeline set to null");
    }

    if let Err(e) = session.close().await {
        log_to!(logger, Error, [ASH] => "Failed to close screencast session, {}", e);
    } else {
        log_to!(logger, Info, [ASH] => "Screencast session closed successfully");
    }

    if let Err(e) = remove_file(settings.daemon_socket_path.clone()) {
        log_to!(logger, Warn, [UNIX] => "Failed to remove daemon socket file, {}", e);
    } else {
        log_to!(logger, Info, [UNIX] => "Daemon socket file removed");
    }

    send_status_to_gui(
        settings.gui_socket_path.clone(),
        String::from("Inactive"),
        &logger,
    );
    log_to!(logger, Info,[CLEANUP] => "Graceful shutdown complete.");
}

// Other misc stuff

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Collect {
    Names,
    Basic,
    All,
}

pub async fn gather_clip_data(level: Collect, args: PullClipsArgs) -> Result<PaginatedClips> {
    let settings = Settings::load();
    let clips_dir_path = Settings::home_path().join(&settings.save_path_from_home_string);
    let json_path = Settings::config_path().join("wayclip").join("data.json");

    if !clips_dir_path.exists() {
        return Ok(PaginatedClips {
            clips: Vec::new(),
            total_pages: 0,
            total_clips: 0,
        });
    }

    let data_val: Value = if json_path.exists() {
        let contents = fs::read_to_string(&json_path)
            .await
            .context("Failed to read data.json")?;
        serde_json::from_str(&contents).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };
    let data = Arc::new(Mutex::new(data_val));
    let data_modified = Arc::new(Mutex::new(false));

    let mut dir = fs::read_dir(&clips_dir_path)
        .await
        .context("Failed to read clips directory")?;
    let mut all_file_paths = Vec::new();
    while let Some(entry) = dir.next_entry().await? {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("mp4") {
            all_file_paths.push(path);
        }
    }
    all_file_paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let filtered_paths = if let Some(query) = args.search_query.as_ref() {
        let lower_query = query.to_lowercase();
        all_file_paths
            .into_iter()
            .filter(|path| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&lower_query)
            })
            .collect()
    } else {
        all_file_paths
    };

    let total_clips = filtered_paths.len();
    let total_pages = (total_clips as f64 / args.page_size as f64).ceil() as usize;
    let start_index = (args.page - 1) * args.page_size;

    let paths_for_page = if start_index < total_clips {
        let end_index = (start_index + args.page_size).min(total_clips);
        &filtered_paths[start_index..end_index]
    } else {
        &[]
    };

    let mut tasks: FuturesUnordered<JoinHandle<Result<Option<ClipData>>>> = FuturesUnordered::new();

    for path in paths_for_page {
        let path_clone = path.clone();
        let data_clone = Arc::clone(&data);
        let data_modified_clone = Arc::clone(&data_modified);

        tasks.push(tokio::spawn(async move {
            let name = path_clone
                .file_name()
                .context("Failed to get file name")?
                .to_string_lossy()
                .into_owned();

            let clip_json_data = {
                let mut data_guard = data_clone.lock().unwrap();
                if let Some(clip_info) = data_guard.get(&name) {
                    serde_json::from_value(clip_info.clone()).unwrap_or_default()
                } else {
                    if let Some(obj) = data_guard.as_object_mut() {
                        obj.insert(name.clone(), json!({ "tags": [], "liked": false }));
                        let mut modified_guard = data_modified_clone.lock().unwrap();
                        *modified_guard = true;
                    }
                    ClipJsonData::default()
                }
            };

            let metadata = fs::metadata(&path_clone)
                .await
                .with_context(|| format!("Failed to read metadata for {name}"))?;

            if metadata.len() == 0 {
                return Ok(None);
            }

            let length = if level == Collect::All {
                get_video_duration(&path_clone).await.unwrap_or(0.0)
            } else {
                0.0
            };

            let created_at: DateTime<Local> = metadata
                .created()
                .map(Into::into)
                .unwrap_or_else(|_| Local::now());
            let updated_at: DateTime<Local> = metadata
                .modified()
                .map(Into::into)
                .unwrap_or_else(|_| Local::now());

            Ok(Some(ClipData {
                name: name.strip_suffix(".mp4").unwrap_or(&name).to_string(),
                path: path_clone.to_str().unwrap_or_default().to_owned(),
                length,
                size: metadata.len(),
                created_at,
                updated_at,
                tags: clip_json_data.tags,
                liked: clip_json_data.liked,
            }))
        }));
    }

    let mut clips_on_page = Vec::new();
    while let Some(result) = tasks.next().await {
        match result {
            Ok(Ok(Some(clip_data))) => clips_on_page.push(clip_data),
            Ok(Ok(None)) => {}
            Ok(Err(e)) => log!([DEBUG] => "Error processing a clip: {:?}", e),
            Err(e) => log!([DEBUG] => "Error in spawned task: {:?}", e),
        }
    }

    let was_modified = *data_modified.lock().unwrap();
    if was_modified {
        if let Some(parent) = json_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .context("Could not create config directory")?;
            }
            let data_to_write = {
                let data_guard = data.lock().unwrap();
                data_guard.clone()
            };
            write_json_data(&json_path, &data_to_write).await?;
        }
    }

    Ok(PaginatedClips {
        clips: clips_on_page,
        total_clips,
        total_pages,
    })
}

pub async fn get_video_duration(path: &Path) -> Result<f64> {
    let path_buf = path.to_path_buf();

    let result = tokio::task::spawn_blocking(move || -> Result<f64> {
        let file = File::open(&path_buf)
            .with_context(|| format!("Failed to open file for duration check: {path_buf:?}"))?;
        let size = file.metadata()?.len();
        let reader = BufReader::new(file);

        let mp4 = Mp4Reader::read_header(reader, size)
            .with_context(|| format!("Failed to read MP4 header for: {path_buf:?}"))?;

        let duration = mp4.moov.mvhd.duration;
        let timescale = mp4.moov.mvhd.timescale;

        if timescale > 0 {
            Ok(duration as f64 / timescale as f64)
        } else {
            Ok(0.0)
        }
    })
    .await;

    match result {
        Ok(Ok(duration)) => Ok(duration),
        Ok(Err(e)) => Err(e),
        Err(join_error) => Err(anyhow::anyhow!(
            "Task for get_video_duration panicked: {}",
            join_error
        )),
    }
}

async fn write_json_data(path: &Path, data: &Value) -> Result<()> {
    let content = serde_json::to_string_pretty(data)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)
        .await?;
    file.write_all(content.as_bytes()).await?;
    Ok(())
}

// Functions for tauri, hence why path is a str instead of Path

pub async fn check_if_exists(path_str: &str) -> bool {
    Path::new(path_str).exists()
}

pub async fn delete_file(path_str: &str) -> Result<(), String> {
    let path = Path::new(path_str);
    if let Err(e) = fs::remove_file(path).await {
        return Err(format!("Failed to delete file: {e}"));
    }
    Ok(())
}

pub async fn update_liked(name: &str, liked: bool) -> Result<()> {
    let json_path = Settings::config_path().join("wayclip").join("data.json");

    let mut data: Value = if json_path.exists() {
        let contents = fs::read_to_string(&json_path)
            .await
            .context("Failed to read data.json")?;
        serde_json::from_str(&contents).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    if let Some(obj) = data.as_object_mut() {
        if let Some(clip) = obj.get_mut(name) {
            if let Some(clip_obj) = clip.as_object_mut() {
                clip_obj.insert("liked".to_string(), json!(liked));
            }
        } else {
            obj.insert(name.to_string(), json!({ "tags": [], "liked": liked }));
        }
    }

    write_json_data(&json_path, &data).await?;

    Ok(())
}

pub async fn generate_preview_clip(video_path: &Path, previews_dir: &Path) -> Result<()> {
    let file_stem = video_path
        .file_stem()
        .context("Could not get file stem from video path")?
        .to_string_lossy();

    let preview_path = previews_dir.join(format!("{file_stem}.mp4"));

    if preview_path.exists() {
        return Ok(());
    }

    tokio::fs::create_dir_all(previews_dir)
        .await
        .context("Failed to create preview cache directory")?;

    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-i",
        video_path.to_str().context("Invalid video path format")?,
        "-t",
        "3",
        "-an",
        "-vf",
        "scale=480:-2",
        "-crf",
        "30",
        "-y",
        preview_path
            .to_str()
            .context("Invalid preview path format")?,
    ]);

    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::piped());

    let child = cmd.spawn().context("Failed to spawn ffmpeg process")?;

    let output = child
        .wait_with_output()
        .await
        .context("ffmpeg command failed to complete")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffmpeg failed for {video_path:?}: {stderr}");
    }

    Ok(())
}

pub async fn generate_all_previews() -> Result<()> {
    let settings = Settings::load();
    let clips_dir_path = Settings::home_path().join(&settings.save_path_from_home_string);
    let previews_path = Settings::config_path().join("wayclip").join("previews");

    if !clips_dir_path.exists() {
        log!([FFMPEG] => "Preview for {clips_dir_path:?} exists, skipping...");

        return Ok(());
    }

    let mut dir = fs::read_dir(&clips_dir_path)
        .await
        .context("Failed to read clips directory")?;

    let mut tasks = FuturesUnordered::new();

    while let Some(entry) = dir
        .next_entry()
        .await
        .context("Failed to read directory entry")?
    {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();

        if !path.is_file() || !name.to_lowercase().ends_with(".mp4") {
            continue;
        }

        let previews_path_clone = previews_path.clone();
        tasks.push(tokio::spawn(async move {
            if let Err(e) = generate_preview_clip(&path, &previews_path_clone).await {
                eprintln!("Could not generate preview for '{name}': {e}");
            }
        }));
    }

    while tasks.next().await.is_some() {}

    log!([FFMPEG] => "Background preview generation scan completed.");
    Ok(())
}

pub async fn rename_all_entries(path_str: &str, new_name: &str) -> Result<(), String> {
    let data_json_path = Settings::config_path().join("wayclip").join("data.json");
    let previews_path = Settings::config_path().join("wayclip").join("previews");

    let original_path = Path::new(path_str);
    let new_path = original_path.with_file_name(new_name);

    // Main
    if let Err(e) = fs::rename(&original_path, &new_path).await {
        let err_msg = format!(
            "Failed to rename file from '{}' to '{}': {}",
            original_path.display(),
            new_path.display(),
            e
        );
        log!([TAURI] => "{}", &err_msg);
        return Err(err_msg);
    }

    // Previews
    if let Some(orig_stem) = original_path.file_stem().and_then(|s| s.to_str()) {
        let preview_old = previews_path.join(format!("{orig_stem}.mp4"));
        if fs::try_exists(&preview_old).await.unwrap_or(false) {
            if let Some(new_stem) = new_path.file_stem().and_then(|s| s.to_str()) {
                let preview_new = previews_path.join(format!("{new_stem}.mp4"));
                if let Err(e) = fs::rename(&preview_old, &preview_new).await {
                    log!([TAURI] => "Failed to rename preview '{}': {}", preview_old.display(), e);
                }
            }
        }
    }

    // Json
    if let Ok(json_str) = fs::read_to_string(&data_json_path).await {
        if let Ok(mut json_val) = serde_json::from_str::<Value>(&json_str) {
            if let Some(obj) = json_val.as_object_mut() {
                if let Some(original_filename) = original_path.file_name().and_then(|s| s.to_str())
                {
                    if let Some(clip_data) = obj.remove(original_filename) {
                        if let Some(new_filename) = new_path.file_name().and_then(|s| s.to_str()) {
                            obj.insert(new_filename.to_string(), clip_data);
                        }
                    }
                }
            }

            if let Err(e) = fs::write(
                &data_json_path,
                serde_json::to_string_pretty(&json_val).unwrap(),
            )
            .await
            {
                let err_msg = format!("Failed to write updated data.json: {e}");
                log!([TAURI] => "{}", &err_msg);
            }
        }
    }

    Ok(())
}
