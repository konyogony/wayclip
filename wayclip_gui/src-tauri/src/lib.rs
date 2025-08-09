use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixListener;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Listener, Manager, Wry,
};
use wayclip_shared::{err, log, Settings, WAYCLIP_TRIGGER_PATH};

pub mod commands;

#[derive(Clone, Serialize, Deserialize)]
struct Payload {
    message: String,
}

fn setup_socket_listener(app: AppHandle<Wry>, socket_path: String) {
    std::thread::spawn(move || {
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
pub fn run() {
    let settings = Settings::load();
    tauri::Builder::default()
        .setup(|app| {
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
            let app_handle = app.handle().clone();
            app.listen("daemon-status-update", move |event| {
                let payload = event.payload();
                if let Ok(p) = serde_json::from_str::<Payload>(payload) {
                    if let Some(menu) = app_handle.menu() {
                        if let Some(item) = menu.get("daemon_status") {
                            let new_text = format!("Status: {}", p.message);
                            if let Err(e) = item.as_menuitem().unwrap().set_text(new_text) {
                                log!([TAURI] => "Failed to update tray text: {e}");
                            }
                        }
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
            commands::pull_settings
        ])
        .run(tauri::generate_context!())
        .expect(err!([TAURI] => "failed to run tauri"));
}
