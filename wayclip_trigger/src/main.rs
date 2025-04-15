use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use wayclip_shared::{err, log};

#[tokio::main]
async fn main() {
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
