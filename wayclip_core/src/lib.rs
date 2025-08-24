use crate::logging::Logger;
use crate::models::UnifiedClipData;
use crate::settings::Settings;
use anyhow::{anyhow, Context, Result};
use ashpd::desktop::{screencast::Screencast, Session};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Local};
use dirs::{config_dir, home_dir};
use ffmpeg_next::codec::Context as CodecContext;
use ffmpeg_next::format::{input, Pixel};
use ffmpeg_next::media::Type;
use ffmpeg_next::software::scaling::{context::Context as SwsContext, flag::Flags};
use ffmpeg_next::util::frame::video::Video;
use futures::stream::{FuturesUnordered, StreamExt};
use gstreamer::prelude::{Cast, ElementExt, GstObjectExt};
use image::{ImageFormat, RgbImage};
use mp4::Mp4Reader;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs::{remove_file, File};
use std::io::BufReader;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::process::Command;
use tokio::task;
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
pub const AUTH: &str = "\x1b[94m[auth]\x1b[0m"; // idk

pub mod api;
pub mod control;
pub mod logging;
pub mod models;
pub mod ring;
pub mod settings;

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
    pub clips: Vec<ClipData>,
    pub total_pages: usize,
    pub total_clips: usize,
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

#[derive(Debug, Serialize)]
pub struct AudioDevice {
    pub id: u32,
    pub name: String,
    pub node_name: String,
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

    if let Err(e) = remove_file(&settings.daemon_pid_path) {
        log_to!(logger, Warn, [UNIX] => "Failed to remove daemon PID file, {}", e);
    } else {
        log_to!(logger, Info, [UNIX] => "Daemon PID file removed");
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
    let settings = Settings::load().await?;
    let clips_dir_path = settings::Settings::home_path().join(&settings.save_path_from_home_string);
    let json_path = settings::Settings::config_path()
        .join("wayclip")
        .join("data.json");

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
    let data_json_path = Settings::config_path().join("wayclip").join("data.json");
    let previews_path = Settings::config_path().join("wayclip").join("previews");
    let path = Path::new(path_str);

    if let Err(e) = fs::remove_file(path).await {
        log!([TAURI] => "Failed to delete main file '{}': {}", path.display(), e);
    }

    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
        let preview_path = previews_path.join(format!("{stem}.mp4"));
        if fs::try_exists(&preview_path).await.unwrap_or(false) {
            if let Err(e) = fs::remove_file(&preview_path).await {
                log!([TAURI] => "Failed to delete preview file '{}': {e}", preview_path.display());
            }
        }
    }

    if let Ok(json_str) = fs::read_to_string(&data_json_path).await {
        if let Ok(mut json_val) = serde_json::from_str::<Value>(&json_str) {
            if let Some(obj) = json_val.as_object_mut() {
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    obj.remove(filename);
                }
            }
            if let Err(e) = fs::write(
                &data_json_path,
                serde_json::to_string_pretty(&json_val).unwrap(),
            )
            .await
            {
                log!([TAURI] => "Failed to write updated data.json after deletion: {}", e);
            }
        }
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
    let settings = Settings::load().await?;
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

pub async fn get_all_audio_devices() -> Result<Vec<AudioDevice>, String> {
    let output = Command::new("pw-cli")
        .args(["ls", "Node"])
        .output()
        .await
        .map_err(|e| format!("Failed to run pw-cli: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "pw-cli error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut devices = Vec::new();
    let mut current_id: Option<u32> = None;
    let mut current_name: Option<String> = None;
    let mut current_class: Option<String> = None;
    let mut current_node_name: Option<String> = None;

    for line in stdout.lines() {
        let trimmed_line = line.trim();

        if trimmed_line.starts_with("id ") {
            if let (Some(id), Some(name), Some(class), Some(node_name)) = (
                current_id,
                &current_name,
                &current_class,
                &current_node_name,
            ) {
                if class == "Audio/Source" || class == "Audio/Sink" {
                    devices.push(AudioDevice {
                        id,
                        name: name.clone(),
                        node_name: node_name.clone(),
                    });
                }
            }

            current_id = None;
            current_name = None;
            current_class = None;
            current_node_name = None;

            if let Some(rest) = trimmed_line.strip_prefix("id ") {
                if let Some((id_str, _)) = rest.split_once(',') {
                    if let Ok(id) = id_str.trim().parse::<u32>() {
                        current_id = Some(id);
                    }
                }
            }
        } else if trimmed_line.contains("media.class") {
            if let Some((_, class)) = trimmed_line.split_once('=') {
                current_class.get_or_insert_with(|| class.trim().trim_matches('"').to_string());
            }
        } else if trimmed_line.contains("node.description")
            || trimmed_line.contains("device.description")
        {
            if let Some((_, name)) = trimmed_line.split_once('=') {
                current_name.get_or_insert_with(|| name.trim().trim_matches('"').to_string());
            }
        } else if trimmed_line.contains("node.name") {
            if let Some((_, node_name)) = trimmed_line.split_once('=') {
                current_node_name
                    .get_or_insert_with(|| node_name.trim().trim_matches('"').to_string());
            }
        }
    }

    if let (Some(id), Some(name), Some(class), Some(node_name)) = (
        current_id,
        &current_name,
        &current_class,
        &current_node_name,
    ) {
        if class == "Audio/Source" || class == "Audio/Sink" {
            devices.push(AudioDevice {
                id,
                name: name.clone(),
                node_name: node_name.clone(),
            });
        }
    }

    Ok(devices)
}

async fn get_default_audio_devices() -> Result<(Option<String>, Option<String>), String> {
    let output = Command::new("pactl")
        .arg("info")
        .output()
        .await
        .map_err(|e| format!("Failed to run pactl: {e}. Is it installed?"))?;

    if !output.status.success() {
        return Err(format!(
            "pactl error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut default_source = None;
    let mut default_sink = None;

    for line in stdout.lines() {
        if let Some(name) = line.strip_prefix("Default Source: ") {
            default_source = Some(name.trim().to_string());
        } else if let Some(name) = line.strip_prefix("Default Sink: ") {
            default_sink = Some(name.trim().to_string());
        }
    }

    Ok((default_source, default_sink))
}

pub async fn get_pipewire_node_id(
    node_name: &String,
    logger: &Logger,
) -> Result<u32, Box<dyn Error>> {
    let output = Command::new("pw-cli")
        .arg("ls")
        .arg("Node")
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let err_msg = format!("'pw-cli ls Node' command failed: {stderr}");
        log_to!(logger, Error, [DAEMON] => "{}", err_msg);
        return Err(err_msg.into());
    }

    let stdout = String::from_utf8(output.stdout)?;

    let mut current_id: Option<u32> = None;
    for line in stdout.lines() {
        let trimmed_line = line.trim();

        if trimmed_line.starts_with("id ") {
            current_id = trimmed_line
                .split(|c: char| c.is_whitespace() || c == ',')
                .nth(1)
                .and_then(|id_str| id_str.parse::<u32>().ok());
        } else if let Some(id) = current_id {
            if trimmed_line.starts_with("node.name =") {
                let name_value = trimmed_line.split_once("=").map(|x| x.1);

                if let Some(name) = name_value {
                    let extracted_name = name.trim().trim_matches('"');
                    if extracted_name == node_name {
                        return Ok(id);
                    }
                }
            }
        }
    }

    let err_msg = format!("PipeWire node with name '{node_name}' not found");
    log_to!(logger, Error, [DAEMON] => "{}", err_msg);
    Err(err_msg.into())
}

pub async fn generate_frames<P: AsRef<Path>>(path: P, count: usize) -> Result<Vec<String>> {
    ffmpeg_next::init().context("Failed to initialize ffmpeg")?;

    let path = path.as_ref().to_owned();

    let thumbnails = task::spawn_blocking(move || -> Result<Vec<String>> {
        if count == 0 {
            return Ok(Vec::new());
        }

        let mut ictx = input(&path).context("Failed to open input file")?;
        let input_stream = ictx
            .streams()
            .best(Type::Video)
            .ok_or_else(|| anyhow!("No video stream found in file"))?;
        let video_stream_index = input_stream.index();

        let mut decoder = CodecContext::from_parameters(input_stream.parameters())?
            .decoder()
            .video()?;

        let duration = input_stream.duration();
        let step = if count > 0 {
            duration / count as i64
        } else {
            0
        };

        let mut frames_base64 = Vec::new();

        let mut scaler = SwsContext::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::RGB24,
            decoder.width(),
            decoder.height(),
            Flags::BILINEAR,
        )?;

        for i in 0..count {
            let timestamp = (i as i64) * step;
            ictx.seek(timestamp, ..)
                .context("Failed to seek in video")?;

            let decoded_frame_result: Option<Result<Video, anyhow::Error>> =
                ictx.packets().find_map(|(stream, packet)| {
                    if stream.index() == video_stream_index {
                        if let Err(e) = decoder.send_packet(&packet) {
                            return Some(Err(anyhow::Error::from(e)));
                        }

                        let mut decoded = Video::empty();
                        if decoder.receive_frame(&mut decoded).is_ok() {
                            return Some(Ok(decoded));
                        }
                    }
                    None
                });

            let decoded = decoded_frame_result
                .ok_or_else(|| anyhow!("Packet stream ended before a frame could be decoded"))??;

            let mut rgb_frame = Video::empty();
            scaler.run(&decoded, &mut rgb_frame)?;

            let image_buffer = RgbImage::from_raw(
                rgb_frame.width(),
                rgb_frame.height(),
                rgb_frame.data(0).to_vec(),
            )
            .ok_or_else(|| anyhow!("Failed to create image from raw frame data"))?;

            let mut bytes = Cursor::new(Vec::new());
            image_buffer.write_to(&mut bytes, ImageFormat::Png)?;
            let b64 = general_purpose::STANDARD.encode(bytes.into_inner());
            frames_base64.push(b64);
        }
        Ok(frames_base64)
    })
    .await
    .context("Failed to join blocking task")??;

    Ok(thumbnails)
}

pub async fn gather_unified_clips() -> Result<Vec<UnifiedClipData>> {
    let hosted_clips_index = match api::get_hosted_clips_index().await {
        Ok(index) => index
            .into_iter()
            .map(|c| (c.file_name, c.id))
            .collect::<HashMap<_, _>>(),
        Err(_) => HashMap::new(),
    };

    let local_clips = gather_clip_data(
        Collect::All,
        PullClipsArgs {
            page: 1,
            page_size: 999,
            search_query: None,
        },
    )
    .await?
    .clips;

    let mut unified_map: HashMap<String, UnifiedClipData> = HashMap::new();

    for local_clip in local_clips {
        let full_filename = Path::new(&local_clip.path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        unified_map.insert(
            full_filename.clone(),
            UnifiedClipData {
                name: local_clip.name,
                full_filename,
                local_path: Some(local_clip.path),
                local_data: Some(ClipJsonData {
                    tags: local_clip.tags,
                    liked: local_clip.liked,
                }),
                created_at: local_clip.created_at,
                is_hosted: false,
                hosted_id: None,
            },
        );
    }

    for (filename, hosted_id) in hosted_clips_index {
        if let Some(existing_clip) = unified_map.get_mut(&filename) {
            existing_clip.is_hosted = true;
            existing_clip.hosted_id = Some(hosted_id);
        } else {
            let name_without_ext = filename
                .strip_suffix(".mp4")
                .unwrap_or(&filename)
                .to_string();
            unified_map.insert(
                filename.clone(),
                UnifiedClipData {
                    name: name_without_ext,
                    full_filename: filename,
                    local_path: None,
                    local_data: None,
                    created_at: Local::now(),
                    is_hosted: true,
                    hosted_id: Some(hosted_id),
                },
            );
        }
    }

    Ok(unified_map.into_values().collect())
}
