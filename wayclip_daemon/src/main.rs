use ashpd::{
    desktop::{
        screencast::{CursorMode, Screencast, SourceType as SourceTypeA},
        PersistMode,
    },
    zbus::fdo::PropertiesProxy,
};
use enumflags2::BitFlags;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, OwnedFd};
use std::sync::{Arc, Mutex};
use std::{collections::VecDeque, os::fd::FromRawFd};
use subprocess::{Exec, Redirection};
use tokio::main;
use xdg_portal::common::SourceType;
use xdg_portal::portal::Portal;
use xdg_portal::screencast::ScreencastReq;

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
                self.buffer.pop_front(); // Remove oldest element
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
        self.buffer.iter().cloned().collect() // Return the current buffer data
    }
}

// Create an Arc<Mutex<OwnedFd>> so that it's shared safely across async tasks.
async fn capture_stream(
    fd: Arc<Mutex<OwnedFd>>,
    buffer: &mut CircularBuffer,
) -> std::io::Result<()> {
    println!("Starting capture_stream");

    // Log the current status of the file descriptor
    let fd_status = fd.lock().unwrap();
    println!("Acquired lock, file descriptor status: {:?}", fd_status);

    let mut stream = unsafe { File::from_raw_fd(fd_status.as_raw_fd()) };
    let mut temp_buffer = [0u8; 1024]; // Adjust the buffer size as needed

    // Log the initial stream info
    println!("Stream started with file descriptor: {:?}", stream);

    loop {
        let bytes_read = stream.read(&mut temp_buffer)?;

        if bytes_read == 0 {
            // If no data was read, log and break the loop
            println!("No more data to read, ending stream capture.");
            break; // End of stream or no more data
        }

        println!("Read {} bytes from stream", bytes_read); // Log the amount of data read

        buffer.push(&temp_buffer[..bytes_read]);

        // If the buffer is full, log that the buffer is full and break
        if buffer.is_full() {
            println!("Buffer is full, ready to dump to disk");
            break;
        }
    }

    // Log when we're done with the capture
    println!("Capture stream ended, buffer size: {}", buffer.len());

    Ok(())
}

fn dump_to_mp4(raw_data: Vec<u8>, output_filename: &str) -> Result<(), String> {
    // Create a temporary file to store raw data
    let temp_raw_path = "temp_raw_data.raw";
    std::fs::write(temp_raw_path, raw_data)
        .map_err(|e| format!("Failed to write raw data to file: {:?}", e))?;

    // Use ffmpeg to convert the raw data into an MP4 file
    let command = std::process::Command::new("ffmpeg")
        .arg("-f")
        .arg("rawvideo")
        .arg("-pix_fmt")
        .arg("yuv420p") // Use appropriate pixel format for raw video
        .arg("-s")
        .arg("1920x1080") // Set the resolution to match the raw video
        .arg("-r")
        .arg("30") // Set the framerate to 30 fps (adjust as needed)
        .arg("-i")
        .arg(temp_raw_path)
        .arg("-c:v")
        .arg("libx264")
        .arg("-y") // Overwrite output file without asking
        .arg(output_filename)
        .output()
        .map_err(|e| format!("Failed to execute ffmpeg: {:?}", e))?;

    if !command.status.success() {
        return Err("ffmpeg failed to encode the video".to_string());
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    // Create a circular buffer to hold the raw stream data
    let mut buffer = CircularBuffer::new(1024 * 1024 * 10); // 10MB buffer

    // Stream 1: Capture data
    match stream_1().await {
        Ok(fd) => {
            if let Err(e) = capture_stream(fd, &mut buffer).await {
                eprintln!("Error capturing stream: {:?}", e);
            }
        }
        Err(e) => eprintln!("Error starting stream 1: {:?}", e),
    }

    // Stream 2: Capture data
    // match stream_2().await {
    //     Ok(fd) => {
    //         if let Err(e) = capture_stream(fd, &mut buffer).await {
    //             eprintln!("Error capturing stream: {:?}", e);
    //         }
    //     }
    //     Err(e) => eprintln!("Error starting stream 2: {:?}", e),
    // }

    // When ready to dump to MP4:
    if buffer.len() == buffer.max_size {
        println!("Buffer is full, ready to dump to MP4");
        let raw_data = buffer.dump(); // Get the raw data
        if let Err(e) = dump_to_mp4(raw_data, "output.mp4") {
            eprintln!("Error dumping data to MP4: {:?}", e);
        }
    } else {
        println!("Buffer is not full yet");
    }
}

async fn stream_1() -> ashpd::Result<Arc<Mutex<OwnedFd>>> {
    let proxy = Screencast::new().await?;
    let session = proxy.create_session().await?;
    let source_types = BitFlags::<SourceTypeA>::from_flag(SourceTypeA::Monitor);
    proxy
        .select_sources(
            &session,
            CursorMode::Embedded,
            source_types,
            false,
            None,
            PersistMode::DoNot,
        )
        .await?;

    let res = proxy.start(&session, None).await?.response()?;
    let fd: OwnedFd = proxy.open_pipe_wire_remote(&session).await?;

    println!("Stream 1 started with response: {:?}", res);
    println!("Stream 1 file descriptor: {:?}", fd);
    Ok(Arc::new(Mutex::new(fd)))
}

async fn stream_2() -> Result<Arc<Mutex<OwnedFd>>, String> {
    let portal = Portal::new().await.unwrap();
    let mut screencast_portal = portal.screencast().await.unwrap();
    let screencast_req = ScreencastReq::new().source_type(SourceType::Window | SourceType::Monitor);

    let res = match screencast_portal.screencast(screencast_req).await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            return Err("Failed to create screencast session".to_string());
        }
    };

    println!("Stream 2 response: {:?}", res);
    let fd = res.fd;
    println!("Stream 2 file descriptor: {:?}", fd);

    Ok(Arc::new(Mutex::new(fd)))
}
