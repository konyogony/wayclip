use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use wayclip_shared::{err, log, WAYCLIP_TRIGGER_PATH};

pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let clip_item = MenuItem::with_id(app, "clip", "Clip that!", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&quit_item, &clip_item])?;

            TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
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
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::update_settings,
            commands::pull_settings
        ])
        .run(tauri::generate_context!())
        .expect(err!([TAURI] => "failed to run tauri"));
}
