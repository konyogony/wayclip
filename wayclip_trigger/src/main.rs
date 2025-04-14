use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

#[tokio::main]
async fn main() {
    if let Ok(mut stream) = UnixStream::connect("/tmp/wayclip.sock").await {
        stream.write_all(b"save\n").await.unwrap();
        println!("saved");
    } else {
        eprintln!("daemon not running");
    }
}
