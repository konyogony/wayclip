use std::io::{BufRead, BufReader};
use std::os::unix::{fs::FileTypeExt, net::UnixListener};
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Listener, Manager, Wry,
};
use wayclip_core::{
    gather_clip_data, generate_all_previews, log, settings::Settings, ClipData, Collect, Payload,
    PullClipsArgs, WAYCLIP_TRIGGER_PATH,
};

pub mod commands;

pub struct AppState {
    pub clips: Mutex<Vec<ClipData>>,
}

fn setup_socket_listener(app: AppHandle<Wry>, socket_path: String) {
    std::thread::spawn(move || {
        if let Ok(metadata) = std::fs::metadata(&socket_path) {
            if metadata.file_type().is_socket() {
                if let Err(e) = std::fs::remove_file(&socket_path) {
                    log!([TAURI] => "Failed to remove old GUI socket file at {}: {}", socket_path, e);
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

                log!([TAURI] => "Failed to bind to socket: {e}");
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
                        log!([TAURI] => "Received status: {}", status_message);

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
                    log!([TAURI] => "Connection failed: {e}");
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    let settings = Settings::load().await.unwrap();
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            app.manage(AppState {
                clips: Mutex::new(Vec::new()),
            });

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
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
                    let state = app_handle.state::<AppState>();
                    let mut clips = state.clips.lock().unwrap();
                    *clips = paginated_clips.clips;
                }
            });

            tauri::async_runtime::spawn(async move {
                if let Err(e) = generate_all_previews().await {
                    eprintln!("An error occurred during background preview generation: {e}",);
                }
            });

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

            let tray_menu = menu.clone();

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            window.show().unwrap();
                            window.set_focus().unwrap();
                            log!([TAURI] => "Open app");
                        }
                        log!([TAURI] => "Open app")
                    }
                    "quit" => {
                        log!([TAURI] => "Exiting app");
                        app.exit(0);
                    }
                    "clip" => {
                        log!([TAURI] => "Clip event recieved");

                        if let Err(e) = std::process::Command::new("sh")
                            .arg(WAYCLIP_TRIGGER_PATH)
                            .spawn()
                        {
                            log!([TAURI] => "failed to run script: {:?}", e);
                        } else {
                            log!([TAURI] => "script launched");
                        }
                    }
                    _ => {
                        log!([TAURI] => "Menu item {:?} not handled", event.id);
                    }
                })
                .show_menu_on_left_click(true)
                .build(app)?;

            setup_socket_listener(app.handle().clone(), settings.gui_socket_path);
            app.listen("daemon-status-update", move |event| {
                let payload = event.payload();
                log!([TAURI] => "Recieved message: {}", &payload);
                match serde_json::from_str::<Payload>(payload) {
                    Ok(p) => {
                        if let Some(item) = tray_menu.get("daemon_status") {
                            let new_text = format!("Status: {}", p.message);
                            log!([TAURI] => "Setting item daemon_status: {}", &new_text);
                            if let Err(e) = item.as_menuitem().unwrap().set_text(new_text) {
                                log!([TAURI] => "Failed to update tray text: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        log!([TAURI] => "Failed to parse Payload JSON: {e}");
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|app, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(e) = window.hide() {
                        log!([TAURI] => "Failed to hide window: {e}");
                    } else {
                        log!([TAURI] => "Window hidden, app running in background");
                    }
                } else {
                    log!([TAURI] => "Main window not found");
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
            commands::get_all_audio_devices_command
        ])
        .run(tauri::generate_context!())
        .expect("Failed to run tauri");
}
