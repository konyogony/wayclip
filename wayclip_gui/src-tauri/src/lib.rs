use std::io::{BufRead, BufReader, Read};
use std::os::unix::{fs::FileTypeExt, net::UnixListener};
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, Wry, Listener
};
use tauri_plugin_store::{Store, StoreExt};
use wayclip_core::{
    gather_clip_data, generate_all_previews, log, settings::Settings, ClipData, Collect, Payload,
    PullClipsArgs, WAYCLIP_TRIGGER_PATH,
};

pub const DEEP_LINK_SOCKET_PATH: &str = "/tmp/wayclip_deep_link.sock";

pub mod auth;
pub mod commands;

pub struct AppState {
    pub clips: Mutex<Vec<ClipData>>,
    pub store: Arc<Store<Wry>>,
}

fn handle_deep_link(app: &AppHandle<Wry>, url_str: &str) {
    log!([TAURI] => "Central handler received deep link: {}", url_str);
    if let Ok(parsed_url) = url::Url::parse(url_str) {
        let token_opt = parsed_url
            .query_pairs()
            .find(|(key, _)| key == "token")
            .map(|(_, value)| value.into_owned());

        if let Some(token) = token_opt {
            log!([TAURI] => "Token found, attempting to store in '.store.bin'...");
            if let Ok(store) = app.store(".store.bin") {
                store.set("auth_token", serde_json::json!(token));
                if let Err(e) = store.save() {
                    log!([TAURI] => "[ERROR] Failed to save store: {}", e);
                } else {
                    log!([TAURI] => "Token stored. Emitting 'auth-state-changed'.");
                    let _ = app.emit("auth-state-changed", true);
                }
            } else {
                log!([TAURI] => "[ERROR] Failed to open store to save token.");
            }
        }
    }
}


fn setup_socket_listener(app: AppHandle<Wry>, socket_path: String) {
    std::thread::spawn(move || {
        log!([TAURI] => "Socket listener thread started.");
        if let Ok(metadata) = std::fs::metadata(&socket_path) {
            if metadata.file_type().is_socket() {
                if let Err(e) = std::fs::remove_file(&socket_path) {
                    log!([TAURI] => "[ERROR] Failed to remove old GUI socket file at {}: {}", socket_path, e);
                    app.emit(
                        "daemon-status-update",
                        Payload {
                            message: String::from("Inactive (Socket Cleanup Failed)"),
                        },
                    )
                    .unwrap();
                    return;
                } else {
                    log!([TAURI] => "Removed stale GUI socket file at {}", socket_path);
                }
            }
        }

        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(e) => {
                app.emit(
                    "daemon-status-update",
                    Payload {
                        message: String::from("Inactive"),
                    },
                )
                .unwrap();

                log!([TAURI] => "[ERROR] Failed to bind to socket: {}", e);
                return;
            }
        };

        log!([TAURI] => "Listening for daemon status on {}", socket_path);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let mut reader = BufReader::new(stream);
                    let mut line = String::new();
                    while reader.read_line(&mut line).unwrap_or(0) > 0 {
                        let status_message = line.trim().to_string();
                        log!([TAURI] => "Socket received status: {}", status_message);

                        app.emit(
                            "daemon-status-update",
                            Payload {
                                message: status_message,
                            },
                        )
                        .unwrap();

                        line.clear();
                    }
                }
                Err(e) => {
                    log!([TAURI] => "[ERROR] Socket connection failed: {}", e);
                }
            }
        }
    });
}



fn setup_deep_link_listener(app: AppHandle<Wry>) {
    let socket_path = DEEP_LINK_SOCKET_PATH;

    // Clean up old socket file from a crash
    if std::fs::metadata(socket_path).is_ok() {
        let _ = std::fs::remove_file(socket_path);
    }

    let listener = match UnixListener::bind(socket_path) {
        Ok(listener) => listener,
        Err(e) => {
            log!([TAURI] => "[CRITICAL_ERROR] Could not bind deep link socket: {}. Deep links will not work.", e);
            return;
        }
    };

    // This thread will own the listener and handle incoming connections.
    std::thread::spawn(move || {
        log!([TAURI] => "Deep link listener thread started at {}", socket_path);
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut url = String::new();
                    if stream.read_to_string(&mut url).is_ok() && !url.is_empty() {
                        log!([TAURI] => "Received deep link via socket: {}", url);
                        
                        // Process the URL
                        handle_deep_link(&app, &url);

                        // Bring the window to the front
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
                Err(err) => {
                    log!([TAURI] => "[ERROR] Deep link socket connection error: {}", err);
                }
            }
        }
    });
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    log!([TAURI] => "Application starting up...");


    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            log!([TAURI] => "Running setup hook...");

            setup_deep_link_listener(app.handle().clone());

            let store = app.store(".store.bin").unwrap();
            app.manage(AppState {
                clips: Mutex::new(Vec::new()),
                store: store.clone(),
            });
            log!([TAURI] => "App state managed.");
            
            let settings_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                log!([TAURI] => "Spawning task to load settings and start socket listener...");
                let settings = Settings::load().await.unwrap();
                log!([TAURI] => "Settings loaded successfully.");
                setup_socket_listener(settings_handle, settings.gui_socket_path);
            });
            
            let async_clips_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                log!([TAURI] => "Spawning background task: gather_clip_data");
                let initial_clips_result = gather_clip_data(
                    Collect::All,
                    PullClipsArgs {
                        page: 1,
                        page_size: 10000,
                        search_query: None,
                    },
                )
                .await;

                if let Ok(paginated_clips) = initial_clips_result {
                    let count = paginated_clips.clips.len();
                    let state = async_clips_handle.state::<AppState>();
                    let mut clips = state.clips.lock().unwrap();
                    *clips = paginated_clips.clips;
                    log!([TAURI] => "Background task 'gather_clip_data' finished. Loaded {} clips into state.", count);
                } else {
                    log!([TAURI] => "[ERROR] Background task 'gather_clip_data' failed.");
                }
            });

            tauri::async_runtime::spawn(async move {
                log!([TAURI] => "Spawning background task: generate_all_previews");
                if let Err(e) = generate_all_previews().await {
                    log!([TAURI] => "[ERROR] An error occurred during background preview generation: {}", e);
                } else {
                    log!([TAURI] => "Background task 'generate_all_previews' completed.");
                }
            });

            log!([TAURI] => "Creating tray menu...");
            let name_item = MenuItem::with_id(app, "name", "Wayclip GUI", false, None::<&str>)?;
            let open_item = MenuItem::with_id(app, "open", "Open Wayclip", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit Wayclip", true, None::<&str>)?;
            let clip_item = MenuItem::with_id(app, "clip", "Clip that!", true, None::<&str>)?;
            let daemon_status_item = MenuItem::with_id(
                app,
                "daemon_status",
                "Status: Inactive",
                false,
                None::<&str>,
            )?;

            let menu = Menu::with_items(
                app,
                &[
                    &name_item,
                    &open_item,
                    &quit_item,
                    &clip_item,
                    &daemon_status_item,
                ],
            )?;
            log!([TAURI] => "Tray menu created.");

            let tray_menu = menu.clone();

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "open" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                                log!([TAURI] => "Main window shown and focused.");
                            } else {
                                log!([TAURI] => "[ERROR] Could not get main window to show.");
                            }
                        }
                        "quit" => {
                            log!([TAURI] => "Quit event received. Exiting application.");
                            app.exit(0);
                        }
                        "clip" => {
                            log!([TAURI] => "Clip event received. Spawning trigger script.");
                            if let Err(e) = std::process::Command::new("sh")
                                .arg(WAYCLIP_TRIGGER_PATH)
                                .spawn()
                            {
                                log!([TAURI] => "[ERROR] failed to run script: {:?}", e);
                            } else {
                                log!([TAURI] => "Trigger script launched successfully.");
                            }
                        }
                        _ => {
                            log!([TAURI] => "Menu item {:?} not handled", event.id);
                        }
                    }
                })
                .show_menu_on_left_click(true)
                .build(app)?;
            log!([TAURI] => "Tray icon built and displayed.");

            log!([TAURI] => "Setting up 'daemon-status-update' listener.");

            let tray_menu_for_listener = tray_menu.clone();
            app.listen("daemon-status-update", move |event| {
                let payload_str = event.payload();
                log!([TAURI] => "Event 'daemon-status-update' received with payload: {:?}", payload_str);
                    match serde_json::from_str::<Payload>(payload_str) {
                        Ok(p) => {
                            if let Some(item) = tray_menu_for_listener.get("daemon_status") {
                                let new_text = format!("Status: {}", p.message);
                                log!([TAURI] => "Setting tray item 'daemon_status' text to: '{}'", &new_text);
                                if let Err(e) = item.as_menuitem().unwrap().set_text(new_text) {
                                    log!([TAURI] => "[ERROR] Failed to update tray text: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            log!([TAURI] => "[ERROR] Failed to parse Payload JSON: {}", e);
                        }
                }
            });

            log!([TAURI] => "Setup hook completed.");
            Ok(())
        })
        .on_window_event(|app, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                log!([TAURI] => "Window close requested. Preventing close and hiding window.");
                api.prevent_close();
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(e) = window.hide() {
                        log!([TAURI] => "[ERROR] Failed to hide window: {}", e);
                    } else {
                        log!([TAURI] => "Window hidden, app running in background.");
                    }
                } else {
                    log!([TAURI] => "[ERROR] Main window not found on close request.");
                }
            }
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::update_settings,
            commands::pull_settings,
            commands::pull_clips,
            commands::delete_clip,
            commands::like_clip,
            commands::rename_clip,
            commands::get_all_audio_devices_command,
            auth::check_auth_status,
            auth::get_me,
            auth::logout
        ])
        .run(tauri::generate_context!())
        .expect("Failed to run tauri application");
}
