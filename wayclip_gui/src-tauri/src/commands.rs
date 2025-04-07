use std::sync::Mutex;
use tauri::State;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use wayclip_shared::Settings; // Import the Settings struct

#[tauri::command]
pub fn update_settings(
    key: String,
    value: String,
    settings: State<'_, Mutex<Settings>>, // Accessing Settings via State
) -> Result<(), Box<dyn std::error::Error>> {
    let mut settings = settings.lock().unwrap();
    settings.update_settings(key, value); // Assuming your Settings struct has an update method
    Ok(())
}

#[tauri::command]
pub fn pull_settings(
    settings: State<'_, Mutex<Settings>>, // Accessing Settings via State
) -> serde_json::Value {
    let settings = settings.lock().unwrap();
    serde_json::to_value(&*settings).unwrap() // Convert the Settings struct to JSON
}

#[tauri::command]
fn unix_socket() -> Result<(), String> {
    tokio::task::block_in_place(|| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let mut socket = UnixStream::connect("/tmp/wayclip-daemon.sock")
                .await
                .map_err(|e| format!("Failed to connect: {}", e))?;

            let message = b"Hello, wayclip-daemon!";
            socket
                .write_all(message)
                .await
                .map_err(|e| format!("Write failed: {}", e))?;

            let mut buf = [0; 1024];
            let n = socket
                .read(&mut buf)
                .await
                .map_err(|e| format!("Read failed: {}", e))?;

            println!("Received from server: {:?}", &buf[..n]);

            Ok(())
        })
    })
}
