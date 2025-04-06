use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![unix_socket])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
