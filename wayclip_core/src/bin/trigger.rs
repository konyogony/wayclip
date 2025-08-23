use rodio::{Decoder, OutputStreamBuilder, Sink};
use std::env;
use std::io::Cursor;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use wayclip_core::{log, settings::Settings};

static SOUND_BYTES: &[u8] = include_bytes!("../../assets/save.oga");

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // For sounds
    let uid = unsafe { libc::getuid() };
    let runtime_dir = format!("/run/user/{uid}");
    env::set_var("XDG_RUNTIME_DIR", runtime_dir);

    let settings = Settings::load().await?;
    if let Ok(stream_handle) = OutputStreamBuilder::open_default_stream() {
        let sink = Sink::connect_new(stream_handle.mixer());

        let cursor = Cursor::new(SOUND_BYTES);
        let source = Decoder::new(cursor).unwrap();
        sink.append(source);

        sink.sleep_until_end();
    } else {
        log!([UNIX] => "Couldn't open default audio stream, no audio output available");
    }
    if let Ok(mut stream) = UnixStream::connect(settings.daemon_socket_path).await {
        stream
            .write_all(b"save\n")
            .await
            .expect("Failed to write to socket");
        stream.flush().await.expect("Failed to flush socket");
        log!([UNIX] => "saved the clip!");
    } else {
        log!([UNIX] => "failed to connect to socket, likely the daemon is not running");
        std::process::exit(1);
    }
    Ok(())
}
