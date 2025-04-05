use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = UnixListener::bind("/tmp/wayclip-daemon.sock")?;

    println!("Server listening on /tmp/wayclip-daemon.sock");

    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            let mut socket = socket;
            let mut buf = [0; 1024];

            match socket.read(&mut buf).await {
                Ok(0) => return, // connection closed
                Ok(n) => {
                    println!("Received {} bytes: {:?}", n, &buf[..n]);
                    if let Err(e) = socket.write_all(&buf[..n]).await {
                        println!("failed to write to socket: {}", e);
                    }
                }
                Err(e) => println!("failed to read from socket: {}", e),
            }
        });
    }
}
