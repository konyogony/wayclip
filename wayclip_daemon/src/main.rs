use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType as SourceTypeA};
use ashpd::desktop::PersistMode;
use enumflags2::BitFlags;
use std::collections::VecDeque;
use std::fs::File;
use std::io::ErrorKind;
use std::os::fd::FromRawFd;
use std::os::unix::io::{AsRawFd, OwnedFd};
use std::sync::{Arc, Mutex};
use tokio::io::AsyncReadExt;
use tokio::main;
use tokio::time::{sleep, Duration, Instant};

struct CircularBuffer {
    buffer: VecDeque<u8>,
    max_size: usize,
}

impl CircularBuffer {
    fn new(max_size: usize) -> Self {
        CircularBuffer {
            buffer: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    fn push(&mut self, data: &[u8]) {
        for &byte in data.iter() {
            if self.buffer.len() == self.max_size {
                self.buffer.pop_front();
            }
            self.buffer.push_back(byte);
        }
    }

    fn len(&self) -> usize {
        self.buffer.len()
    }

    fn is_full(&self) -> bool {
        self.buffer.len() == self.max_size
    }

    fn dump(&self) -> Vec<u8> {
        self.buffer.iter().cloned().collect()
    }
}

async fn capture_stream(
    fd: Arc<Mutex<OwnedFd>>,
    buffer: &mut CircularBuffer,
) -> std::io::Result<()> {
    println!("Starting capture_stream");

    let stream_fd = {
        let fd_lock = fd.lock().unwrap();
        let raw_fd = fd_lock.as_raw_fd();

        let duped_fd = unsafe { libc::dup(raw_fd) };
        if duped_fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        duped_fd
    };

    let stream = unsafe { File::from_raw_fd(stream_fd) };
    println!("Stream started with file descriptor: {:?}", stream);

    let mut async_stream = tokio::fs::File::from(stream);
    let mut temp_buffer = [0u8; 576];
    let mut last_read_time = Instant::now();
    let timeout_duration = Duration::from_secs(5);
    let mut retry_delay = Duration::from_millis(10); // Initial delay

    loop {
        let read_result = async_stream.read(&mut temp_buffer).await;

        match read_result {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    println!("No more data to read, ending stream capture.");
                    break;
                }

                println!("Read {} bytes from stream", bytes_read);
                buffer.push(&temp_buffer[..bytes_read]);
                last_read_time = Instant::now();

                // Reset retry delay on successful read
                retry_delay = Duration::from_millis(10);

                if buffer.is_full() {
                    println!("Buffer is full, ready to dump to disk");
                    break;
                }
            }
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    let elapsed = last_read_time.elapsed();
                    if elapsed > timeout_duration {
                        println!("Timeout: No data for {:?}, stopping capture", elapsed);
                        break;
                    }

                    println!("WouldBlock error, retrying after {:?}", retry_delay);
                    tokio::time::sleep(retry_delay).await;

                    // Increase retry delay, but with a maximum
                    retry_delay = (retry_delay * 2).min(Duration::from_millis(500));
                    continue;
                } else {
                    eprintln!("Error reading from stream: {:?}", e);
                    break;
                }
            }
        }
    }

    println!("Capture stream ended, buffer size: {}", buffer.len());
    Ok(())
}

fn dump_to_mp4(raw_data: Vec<u8>, output_filename: &str) -> Result<(), String> {
    let temp_raw_path = "temp_raw_data.raw";
    std::fs::write(temp_raw_path, raw_data)
        .map_err(|e| format!("Failed to write raw data: {}", e))?;

    let command = std::process::Command::new("ffmpeg")
        .args(&[
            "-f",
            "rawvideo",
            "-pix_fmt",
            "yuv420p",
            "-s",
            "2560x1440",
            "-r",
            "60",
            "-i",
            temp_raw_path,
            "-c:v",
            "libx264",
            "-y",
            output_filename,
        ])
        .output()
        .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;

    if !command.status.success() {
        eprintln!(
            "ffmpeg stderr: {:?}",
            String::from_utf8_lossy(&command.stderr)
        );
        return Err(format!("ffmpeg failed: {:?}", command.status));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> ashpd::Result<()> {
    let mut buffer = CircularBuffer::new(1024 * 1024 * 10);

    let proxy = Screencast::new().await?;
    let session = proxy.create_session().await?;
    let source_types = BitFlags::<SourceTypeA>::from_flag(SourceTypeA::Monitor);

    println!("Selecting sources...");
    proxy
        .select_sources(
            &session,
            CursorMode::Embedded,
            source_types,
            false,
            None,
            PersistMode::Application,
        )
        .await?;
    println!("Sources selected.");

    println!("Starting screencast...");
    let res = proxy.start(&session, None).await?.response()?;
    println!("Screencast started with response: {:?}", res);

    println!("Opening PipeWire remote...");
    let fd: OwnedFd = proxy.open_pipe_wire_remote(&session).await?;
    let fd_arc = Arc::new(Mutex::new(fd));
    println!("PipeWire remote opened, file descriptor: {:?}", fd_arc);

    println!("Starting capture stream...");
    if let Err(e) = capture_stream(fd_arc.clone(), &mut buffer).await {
        eprintln!("Error capturing stream: {:?}", e);
    }
    println!("Capture stream finished.");

    if buffer.is_full() {
        println!("Buffer is full, ready to dump to MP4");
        let raw_data = buffer.dump();
        if let Err(e) = dump_to_mp4(raw_data, "output.mp4") {
            eprintln!("Error dumping data to MP4: {:?}", e);
        }
    } else {
        println!("Buffer is not full yet");
    }

    Ok(())
}
