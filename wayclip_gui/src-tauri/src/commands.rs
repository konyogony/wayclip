use crate::AppState;
use serde_json::Value;
use std::path::Path;
use tauri::State;
use wayclip_core::{
    check_if_exists, delete_file, get_all_audio_devices, log, rename_all_entries,
    settings::Settings, update_liked, AudioDevice, PaginatedClips,
};

#[tauri::command(async)]
pub async fn update_settings(key: &str, value: Value) -> Result<(), String> {
    match Settings::update_key(key, value).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let err_msg = format!("Failed to update settings: {}", &e);
            log!([TAURI] => "{}", &err_msg);
            Err(err_msg)
        }
    }
}

#[tauri::command(async)]
pub async fn pull_settings() -> Result<serde_json::Value, String> {
    match Settings::to_json().await {
        Ok(s) => Ok(s),
        Err(e) => {
            let err_msg = format!("Failed to pull settings: {}", &e);
            log!([TAURI] => "{}", &err_msg);
            Err(err_msg)
        }
    }
}

#[tauri::command(async)]
pub async fn pull_clips(
    page: usize,
    page_size: usize,
    search_query: Option<String>,
    state: State<'_, AppState>,
) -> Result<PaginatedClips, String> {
    let all_clips_guard = state.clips.lock().map_err(|e| e.to_string())?;
    let filtered_clips: Vec<_> = if let Some(query) = search_query.filter(|q| !q.is_empty()) {
        all_clips_guard
            .iter()
            .filter(|clip| clip.name.to_lowercase().contains(&query.to_lowercase())) // Case-insensitive search
            .cloned()
            .collect()
    } else {
        all_clips_guard.clone()
    };

    let total_clips = filtered_clips.len();
    let total_pages = (total_clips as f64 / page_size as f64).ceil() as usize;

    let start = (page - 1) * page_size;
    let paginated_clip_data = filtered_clips
        .into_iter()
        .skip(start)
        .take(page_size)
        .collect();

    Ok(PaginatedClips {
        clips: paginated_clip_data,
        total_pages,
        total_clips,
    })
}

#[tauri::command(async)]
pub async fn delete_clip(path_str: &str, state: State<'_, AppState>) -> Result<(), String> {
    if !check_if_exists(path_str).await {
        let err_msg = format!("Path {path_str} doesnt exist");
        log!([TAURI] => "{}", &err_msg);
        return Err(err_msg);
    };
    if let Err(e) = delete_file(path_str).await {
        let err_msg = format!("Failed to delete file: {e}");
        log!([TAURI] => "{}", &err_msg);
        return Err(err_msg);
    };
    let mut clips = state.clips.lock().map_err(|e| e.to_string())?;
    clips.retain(|clip| clip.path != path_str);

    Ok(())
}

#[tauri::command(async)]
pub async fn like_clip(name: &str, liked: bool, state: State<'_, AppState>) -> Result<(), String> {
    if let Err(e) = update_liked(name, liked).await {
        let err_msg = format!("Failed to delete file: {e}");
        log!([TAURI] => "{}", &err_msg);
        return Err(err_msg);
    }

    let mut clips_guard = state.clips.lock().map_err(|e| e.to_string())?;
    if let Some(clip) = clips_guard.iter_mut().find(|c| c.name == name) {
        clip.liked = liked;
    } else {
        log!([TAURI] => "Warning: Clip '{}' updated on disk but not found in memory state.", name);
    }
    Ok(())
}

#[tauri::command(async)]
pub async fn rename_clip(
    path_str: &str,
    new_name: &str,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if !check_if_exists(path_str).await {
        let err_msg = format!("Path {path_str} doesnt exist");
        log!([TAURI] => "{}", &err_msg);
        return Err(err_msg);
    };
    let old_path = Path::new(path_str);
    let extension = old_path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("");
    let new_filename = if extension.is_empty() {
        new_name.to_string()
    } else {
        format!("{new_name}.{extension}")
    };
    let new_path_buf = old_path.with_file_name(new_filename);
    let new_path_str = new_path_buf
        .to_str()
        .ok_or("Failed to create new path string")?
        .to_string();

    if let Err(e) = rename_all_entries(path_str, new_name).await {
        let err_msg = format!("Failed to rename file: {e}");
        log!([TAURI] => "{}", &err_msg);
        return Err(err_msg);
    };

    let mut clips_guard = state.clips.lock().map_err(|e| e.to_string())?;

    if let Some(clip) = clips_guard.iter_mut().find(|c| c.path == path_str) {
        clip.name = new_name.to_string();
        clip.path = new_path_str;
    } else {
        log!([TAURI] => "Warning: Clip at '{}' renamed on disk but not found in memory state.", path_str);
    }
    Ok(())
}

#[tauri::command(async)]
pub async fn get_all_audio_devices_command() -> Result<Vec<AudioDevice>, String> {
    get_all_audio_devices().await
}
