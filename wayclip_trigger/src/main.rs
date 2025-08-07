use rodio::{Decoder, OutputStreamBuilder, Sink};
use std::io::Cursor;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use wayclip_shared::{err, log};

static SOUND_BYTES: &[u8] = include_bytes!("../assets/save.oga");

#[tokio::main]
async fn main() {
    if let Ok(stream_handle) = OutputStreamBuilder::open_default_stream() {
        let sink = Sink::connect_new(stream_handle.mixer());

        let cursor = Cursor::new(SOUND_BYTES);
        let source = Decoder::new(cursor).unwrap();
        sink.append(source);

        sink.sleep_until_end();
    } else {
        log!([UNIX] => "couldn't open default audio stream, no audio output available");
    }
    if let Ok(mut stream) = UnixStream::connect("/tmp/wayclip.sock").await {
        stream
            .write_all(b"save\n")
            .await
            .expect(err!([UNIX] => "failed to write to socket"));
        stream
            .flush()
            .await
            .expect(err!([UNIX] => "failed to flush socket"));
        log!([UNIX] => "saved the clip!");
    } else {
        log!([UNIX] => "failed to connect to socket, likely the daemon is not running");
        std::process::exit(1);
    }
}
