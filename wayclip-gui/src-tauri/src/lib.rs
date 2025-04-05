use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

#[tauri::command]
pub fn unix_socket() -> Result<(), Box<dyn std::error::Error>> {
    // Wrapping the async code in a blocking task
    tokio::task::block_in_place(async {
        let mut socket = UnixStream::connect("/tmp/wayclip-daemon.sock").await?;

        let message = b"Hello, wayclip-daemon!";
        socket.write_all(message).await?;

        let mut buf = [0; 1024];
        let n = socket.read(&mut buf).await?;

        println!("Received from server: {:?}", &buf[..n]);

        Ok::<(), Box<dyn std::error::Error>>(()) // This returns the result
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
