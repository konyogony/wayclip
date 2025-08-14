use serde_json::Value;
use wayclip_core::{
    check_if_exists, delete_file, gather_clip_data, log, rename_all_entries, update_liked, Collect,
    PaginatedClips, PullClipsArgs, Settings,
};

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
pub async fn pull_clips(
    page: usize,
    page_size: usize,
    search_query: Option<String>,
) -> Result<PaginatedClips, String> {
    gather_clip_data(
        Collect::All,
        PullClipsArgs {
            page,
            page_size,
            search_query,
        },
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub async fn delete_clip(path_str: &str) -> Result<(), String> {
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
    Ok(())
}

#[tauri::command(async)]
pub async fn like_clip(name: &str, liked: bool) -> Result<(), String> {
    if let Err(e) = update_liked(name, liked).await {
        let err_msg = format!("Failed to delete file: {e}");
        log!([TAURI] => "{}", &err_msg);
        return Err(err_msg);
    }
    Ok(())
}

#[tauri::command(async)]
pub async fn rename_clip(path_str: &str, new_name: &str) -> Result<(), String> {
    if !check_if_exists(path_str).await {
        let err_msg = format!("Path {path_str} doesnt exist");
        log!([TAURI] => "{}", &err_msg);
        return Err(err_msg);
    };
    if let Err(e) = rename_all_entries(path_str, new_name).await {
        let err_msg = format!("Failed to rename file : {e}",);
        log!([TAURI] => "{}", &err_msg);
        return Err(err_msg);
    };
    Ok(())
}
