use ashpd::desktop::input_capture::Capabilities;
use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType};
use ashpd::desktop::{PersistMode, Session};
use ashpd::WindowIdentifier;
use device_query::{DeviceQuery, DeviceState, Keycode};
use enumflags2::BitFlag;
use gstreamer as gst;
use gstreamer::glib::object::Cast;
use gstreamer::prelude::{ElementExt, GstBinExt};
use gstreamer_app::AppSink;
use libc::listen;
use std::collections::VecDeque;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::{fs::File, os::unix::io::AsRawFd, process::Command};
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::time::Duration;

struct RingBuffer {
    buffer: VecDeque<Vec<u8>>,
    capacity: usize,
}

impl RingBuffer {
    fn new(capacity: usize) -> Self {
        println!("[ring] init w/ cap {}", capacity);
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    fn push(&mut self, data: Vec<u8>) {
        if self.buffer.len() == self.capacity {
            println!("[ring] cap hit, pop oldest");
            self.buffer.pop_front();
        }
        println!("[ring] push frame: {} bytes", data.len());
        self.buffer.push_back(data);
    }
}

// Works only for hyprland, launches rust process to send data via unix socket
async fn setup_bind_hyprland() {
    let output = Command::new("hyprctl")
        .args([
            "keyword",
            "bind",
            "Alt_L,C,exec,/usr/local/bin/wayclip_trigger",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("[hypr] failed to add bind")
        .wait();
    if let Ok(output) = output {
        // Check if the output is "ok"
        if output.success() {
            println!("[hypr] bind added successfully");
        } else {
            println!("[hypr] failed to add bind");
            println!("[hypr] error: {}", output.to_string());
        }
    } else {
        println!("[hypr] failed to add bind");
    }
}

#[tokio::main]
async fn main() {
    // --- INIT ---

    println!("[init] starting...");
    gst::init().expect("[gst] init fail");

    println!("[unix] starting unix listener");
    let listener = UnixListener::bind("/tmp/wayclip.sock").expect("[unix] unix listener fail");

    // Check if using hyprland
    if std::env::var("DESKTOP_SESSION") == Ok("hyprland".to_string()) {
        println!("[init] using hyprland");
        setup_bind_hyprland().await;
    } else {
        println!("[init] not using hyprland");
        println!("[init] please bind LAlt + C to /usr/local/bin/wayclip_trigger");
    }

    // --- SCREENCAST ---

    println!("[ashpd] creating screencast proxy");
    let proxy = Screencast::new()
        .await
        .expect("[ashpd] screencast proxy creation fail");

    println!("[ashpd] creating screencast session");
    let session = proxy
        .create_session()
        .await
        .expect("[ashpd] creating screencast session fail");

    println!("[ashpd] selecting sources for screencast...");
    proxy
        .select_sources(
            &session,
            CursorMode::Hidden, // Doesnt matter what you set it, still will see cursor
            enumflags2::BitFlags::from(SourceType::Monitor), // Seems like a useless property to me
            false,
            None,
            // Since this is a cli, i cant really get the window id
            // meaning i dont have an identifier
            PersistMode::Application,
        )
        .await
        .expect("[ashpd] select_sources for screencast fail");

    println!("[ashpd] starting screencast session...");
    let response = proxy
        .start(&session, None)
        .await
        .expect("[ashpd] starting screencast session fail")
        .response()
        .expect("[ashpd] grabbing response fail");

    println!(
        "[ashpd] got {} streams, opening pipewire remote",
        response.streams().len()
    );
    let pipewire_fd = proxy
        .open_pipe_wire_remote(&session)
        .await
        .expect("[ashpd] open_pipe_wire_remote fail");
    println!(
        "[ashpd] got pipewire file descriptor: {:?}",
        pipewire_fd.as_raw_fd()
    );

    // --- GSTREAMER ---

    println!("[ashpd] creating a new ring buffer");
    let ring_buffer = Arc::new(Mutex::new(RingBuffer::new(512))); // 512 frames

    let pipeline_str = format!(
        "pipewiresrc fd={} ! videoconvert ! video/x-raw,format=I420 ! appsink name=sink",
        pipewire_fd.as_raw_fd()
    );
    println!("[gst] pipeline str:\n{}", pipeline_str);

    println!("[gst] parsing pipeline");
    let pipeline = gst::parse::launch(&pipeline_str).expect("[gst] failed to parse pipeline");

    // Some magic going on here
    println!("[gst] getting appsink element");
    let appsink = pipeline
        .clone()
        .dynamic_cast::<gst::Bin>()
        .expect("[gst] cast to bin failed")
        .by_name("sink")
        .expect("[gst] couldn't find sink")
        .dynamic_cast::<AppSink>()
        .expect("[gst] cast to AppSink failed");

    let rb_clone = ring_buffer.clone();

    println!("[gst] setting appsink callbacks");
    appsink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = sink.pull_sample().expect("[gst] failed to pull sample");
                let buffer = sample.buffer().expect("[gst] no buffer in sample");
                let map = buffer.map_readable().expect("[gst] failed to map buffer");
                let data = map.as_slice().to_vec();

                println!("[gst] got sample, size: {}", data.len());
                if let Ok(mut rb) = rb_clone.lock() {
                    rb.push(data);
                } else {
                    println!("[ring] failed to lock");
                }

                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    println!("[gst] setting pipeline to playing");
    pipeline
        .set_state(gst::State::Playing)
        .expect("[gst] failed to play");

    loop {
        let (mut stream, _) = listener.accept().await.expect("[unix] unix accept fail");
        let mut buf = [0u8; 64]; // enough to read small msgs
        let n = stream.read(&mut buf).await.expect("[unix] read fail");

        if n == 0 {
            continue;
        }

        let msg = std::str::from_utf8(&buf[..n]).unwrap_or("").trim();
        println!("got msg: {}", msg);

        if msg == "save" {
            println!("saving clip");
            break;
        }
    }

    println!("[gst] stopping pipeline");
    pipeline
        .set_state(gst::State::Null)
        .expect("[gst] failed to stop");

    println!("[ffmpeg] spawning ffmpeg to consume from pipe");
    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-y", // overwrite output file
            "-f",
            "rawvideo",
            "-pixel_format",
            "yuv420p",
            "-video_size",
            "2560x1440",
            "-framerate",
            "30",
            "-i",
            "-",
            "output.mp4",
        ])
        .stdin(Stdio::piped())
        .spawn()
        .expect("[ffmpeg] failed to start");

    let mut ffmpeg_stdin = ffmpeg.stdin.take().expect("[ffmpeg] no stdin");

    {
        let rb = ring_buffer.lock().expect("[ring] lock fail for ffmpeg");
        for (i, chunk) in rb.buffer.iter().enumerate() {
            println!("[ffmpeg] writing chunk {} size {}", i, chunk.len());
            ffmpeg_stdin.write_all(chunk).expect("[ffmpeg] write fail");
        }
    }

    drop(ffmpeg_stdin); // close stdin to signal ffmpeg we're done

    let status = ffmpeg.wait().expect("[ffmpeg] wait fail");
    println!("[ffmpeg] exited with status: {}", status);

    println!("[unix] deleting unix socket");
    std::fs::remove_file("/tmp/clip.sock").ok();

    println!("[done] check output.mp4");
}
