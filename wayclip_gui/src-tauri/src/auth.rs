use crate::AppState;
use std::sync::Arc;
use tauri::{command, AppHandle, Emitter, State, Wry};
use tauri_plugin_store::{Error as StoreError, Store, StoreBuilder};
use wayclip_core::log; // Assuming your log macro is accessible from here

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UserData {
    pub id: uuid::Uuid,
    pub username: String,
    pub avatar_url: Option<String>,
}

fn get_store(app_handle: &AppHandle<Wry>) -> Result<Arc<Store<Wry>>, StoreError> {
    StoreBuilder::new(app_handle, ".store.bin").build()
}

#[command]
pub fn check_auth_status(state: State<'_, AppState>) -> Result<bool, String> {
    log!([AUTH] => "Running command: check_auth_status");

    let store = state.store.clone();

    let has_token = store.has("auth_token");
    log!([AUTH] => "Auth token found: {}", has_token);
    Ok(has_token)
}

#[command]
pub async fn get_me(app_handle: AppHandle<Wry>) -> Result<UserData, String> {
    log!([AUTH] => "Running command: get_me");
    let store = get_store(&app_handle).map_err(|e| {
        let err_msg = format!("Failed to load store: {e}");
        log!([AUTH] => "[ERROR] {}", err_msg);
        err_msg
    })?;

    let token: String = store
        .get("auth_token")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .ok_or_else(|| {
            let err_msg = "Auth token not found or invalid in store".to_string();
            log!([AUTH] => "[ERROR] {}", err_msg);
            err_msg
        })?;

    log!([AUTH] => "Auth token found. Making API request to /api/me");
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8080/api/me")
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| {
            let err_msg = format!("Failed to send request: {e}");
            log!([AUTH] => "[ERROR] {}", err_msg);
            err_msg
        })?;

    let status = response.status();
    log!([AUTH] => "API request to /api/me completed with status: {}", status);

    if status.is_success() {
        let user_data = response.json::<UserData>().await.map_err(|e| {
            let err_msg = format!("Failed to parse user data: {e}");
            log!([AUTH] => "[ERROR] {}", err_msg);
            err_msg
        })?;
        log!([AUTH] => "Successfully fetched and parsed user data for user: {}", user_data.username);
        Ok(user_data)
    } else {
        let err_msg = format!("API request failed with status: {}", status);
        log!([AUTH] => "[ERROR] {}", err_msg);
        Err(err_msg)
    }
}

#[command]
pub async fn logout(app_handle: AppHandle<Wry>) -> Result<(), String> {
    log!([AUTH] => "Running command: logout");
    let store = get_store(&app_handle).map_err(|e| {
        let err_msg = format!("Failed to load store: {e}");
        log!([AUTH] => "[ERROR] {}", err_msg);
        err_msg
    })?;

    if store.has("auth_token") {
        log!([AUTH] => "Auth token found, proceeding with deletion.");
        store.delete("auth_token");
        store.save().map_err(|e| {
            let err_msg = format!("Failed to save store after deleting token: {e}");
            log!([AUTH] => "[ERROR] {}", err_msg);
            err_msg
        })?;
        log!([AUTH] => "Auth token successfully deleted from store.");
    } else {
        log!([AUTH] => "No auth token found in store, nothing to do.");
    }

    log!([AUTH] => "Emitting 'auth-state-changed' event with payload: false");
    app_handle.emit("auth-state-changed", false).map_err(|e| {
        let err_msg = format!("Failed to emit event: {e}");
        log!([AUTH] => "[ERROR] {}", err_msg);
        err_msg
    })?;

    log!([AUTH] => "Logout command completed successfully.");
    Ok(())
}
