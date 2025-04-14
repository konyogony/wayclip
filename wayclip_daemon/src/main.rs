use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType};
use ashpd::desktop::PersistMode;
use gstreamer as gst;
use gstreamer::glib::object::Cast;
use gstreamer::prelude::{ElementExt, GstBinExt};
use gstreamer_app::AppSink;
use std::collections::VecDeque;
use std::io::Write;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::{os::unix::io::AsRawFd, process::Command};
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::time::{sleep_until, Duration, Instant};

const FRAMES: usize = 3584;

const PREFIX_INIT: &str = "\x1b[35m[init]\x1b[0m"; // magenta
const PREFIX_UNIX: &str = "\x1b[36m[unix]\x1b[0m"; // cyan
const PREFIX_ASH: &str = "\x1b[34m[ashpd]\x1b[0m"; // blue
const PREFIX_GST: &str = "\x1b[32m[gst]\x1b[0m"; // green
const PREFIX_RING: &str = "\x1b[33m[ring]\x1b[0m"; // yellow
const PREFIX_HYPR: &str = "\x1b[31m[hypr]\x1b[0m"; // red
const PREFIX_FFMPEG: &str = "\x1b[95m[ffmpeg]\x1b[0m"; // white

type Frame = (Vec<u8>, u64);

struct RingBuffer {
    buffer: VecDeque<Frame>,
    capacity: usize,
}

impl RingBuffer {
    fn new(capacity: usize) -> Self {
        println!("{} init w/ cap {}", PREFIX_RING, capacity);
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    fn push(&mut self, data: Vec<u8>, pts: u64) {
        if self.buffer.len() == self.capacity {
            println!("{} cap hit, pop oldest", PREFIX_RING);
            self.buffer.pop_front();
        }
        println!("{} push frame: {} bytes", PREFIX_RING, data.len());
        self.buffer.push_back((data, pts));
    }

    fn iter(&self) -> impl Iterator<Item = &Frame> {
        self.buffer.iter()
    }
}

// Works only for hyprland, launches rust process to send data via unix socket
async fn setup_bind_hyprland() {
    let output = Command::new("hyprctl")
        .args([
            "keyword",
            "bind",
            "Alt_L,C,exec,/home/kony/Documents/GitHub/wayclip/target/debug/wayclip_trigger",
            // Temporary path
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect(format!("{} failed to add bind", PREFIX_HYPR).as_str())
        .wait();
    if let Ok(output) = output {
        // Check if the output is "ok"
        if output.success() {
            println!("{} bind added successfully", PREFIX_HYPR);
        } else {
            println!("{} failed to add bind", PREFIX_HYPR);
            println!("{} error: {}", PREFIX_HYPR, output.to_string());
        }
    } else {
        println!("{} failed to add bind", PREFIX_HYPR);
    }
}

#[tokio::main]
async fn main() {
    // --- INIT ---

    println!("{} starting...", PREFIX_INIT);
    gst::init().expect(format!("{} gst init fail", PREFIX_GST).as_str());

    println!("{} starting unix listener", PREFIX_UNIX);
    let listener = UnixListener::bind("/tmp/wayclip.sock")
        .expect(format!("{} unix listener fail", PREFIX_UNIX).as_str());

    // Check if using hyprland
    if std::env::var("DESKTOP_SESSION") == Ok("hyprland".to_string()) {
        println!("{} using hyprland", PREFIX_HYPR);
        setup_bind_hyprland().await;
    } else {
        println!("{} not using hyprland", PREFIX_HYPR);
        println!(
            "{} please bind LAlt + C to /usr/local/bin/wayclip_trigger",
            PREFIX_HYPR
        );
        // Or some other path
    }

    // --- SCREENCAST ---

    println!("{} creating screencast proxy", PREFIX_ASH);
    let proxy = Screencast::new()
        .await
        .expect(format!("{} screencast proxy creation fail", PREFIX_ASH).as_str());

    println!("{} creating screencast session", PREFIX_ASH);
    let session = proxy
        .create_session()
        .await
        .expect(format!("{} creating screencast session fail", PREFIX_ASH).as_str());

    println!("{} selecting sources for screencast...", PREFIX_ASH);
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
        .expect(format!("{} select_sources for screencast fail", PREFIX_ASH).as_str());

    println!("{} starting screencast session...", PREFIX_ASH);
    let response = proxy
        .start(&session, None)
        .await
        .expect(format!("{} starting screencast session fail", PREFIX_ASH).as_str())
        .response()
        .expect(format!("{} grabbing response fail", PREFIX_ASH).as_str());

    println!(
        "{} got {} streams, opening pipewire remote",
        PREFIX_ASH,
        response.streams().len()
    );
    let pipewire_fd = proxy
        .open_pipe_wire_remote(&session)
        .await
        .expect(format!("{} open_pipe_wire_remote fail", PREFIX_ASH).as_str());
    println!(
        "{} got pipewire file descriptor: {:?}",
        PREFIX_ASH,
        pipewire_fd.as_raw_fd()
    );

    // --- GSTREAMER ---

    println!("{} creating a new ring buffer", PREFIX_RING);
    let ring_buffer = Arc::new(Mutex::new(RingBuffer::new(FRAMES))); // 512 frames

    let pipeline_str = format!(
        "pipewiresrc fd={} ! videoconvert ! video/x-raw,format=I420 ! appsink name=sink",
        pipewire_fd.as_raw_fd()
    );
    println!("{} pipeline str:\n{}", PREFIX_GST, pipeline_str);

    println!("{} parsing pipeline", PREFIX_GST);
    let pipeline = gst::parse::launch(&pipeline_str)
        .expect(format!("{} failed to parse pipeline", PREFIX_GST).as_str());

    // Some magic going on here
    println!("{} getting appsink element", PREFIX_GST);
    let appsink = pipeline
        .clone()
        .dynamic_cast::<gst::Bin>()
        .expect(format!("{} cast to bin failed", PREFIX_GST).as_str())
        .by_name("sink")
        .expect(format!("{} couldn't find sink", PREFIX_GST).as_str())
        .dynamic_cast::<AppSink>()
        .expect(format!("{} cast to AppSink failed", PREFIX_GST).as_str());

    let rb_clone = ring_buffer.clone();

    println!("{} setting appsink callbacks", PREFIX_GST);
    appsink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = sink
                    .pull_sample()
                    .expect(format!("{} failed to pull sample", PREFIX_GST).as_str());
                let buffer = sample
                    .buffer()
                    .expect(format!("{} no buffer in sample", PREFIX_GST).as_str());
                let map = buffer
                    .map_readable()
                    .expect(format!("{} failed to map buffer", PREFIX_GST).as_str());
                let data = map.as_slice().to_vec();
                let pts = buffer.pts().unwrap().nseconds() / 1000; // convert to microseconds

                println!("{} got sample, size: {}", PREFIX_GST, data.len());
                if let Ok(mut rb) = rb_clone.lock() {
                    rb.push(data, pts);
                } else {
                    println!("{} failed to lock", PREFIX_RING);
                }

                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    println!("{} setting pipeline to playing", PREFIX_GST);
    pipeline
        .set_state(gst::State::Playing)
        .expect(format!("{} failed to play", PREFIX_GST).as_str());

    loop {
        let (mut stream, _) = listener
            .accept()
            .await
            .expect(format!("{} unix accept fail", PREFIX_UNIX).as_str());
        let mut buf = [0u8; 64]; // enough to read small msgs
        let n = stream
            .read(&mut buf)
            .await
            .expect(format!("{} read fail", PREFIX_UNIX).as_str());

        if n == 0 {
            continue;
        }

        let msg = std::str::from_utf8(&buf[..n]).unwrap_or("").trim();
        println!("{} got msg: {}", PREFIX_UNIX, msg);

        if msg == "save" {
            println!("{} saving clip", PREFIX_UNIX);
            break;
        }
    }

    println!("{} stopping pipeline", PREFIX_GST);
    pipeline
        .set_state(gst::State::Null)
        .expect(format!("{} failed to stop", PREFIX_GST).as_str());

    println!("{} spawning ffmpeg to consume from pipe", PREFIX_FFMPEG);

    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-y",
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
        .expect(format!("{} ffmpeg spawn fail", PREFIX_FFMPEG).as_str());

    let mut stdin = ffmpeg.stdin.take().unwrap();
    let buffer = ring_buffer.lock().unwrap();
    let first_pts = buffer.buffer.front().map(|(_, pts)| *pts).unwrap_or(0);

    let start = Instant::now();
    for (frame, pts) in buffer.iter() {
        let rel = *pts - first_pts;
        let when = start + Duration::from_micros(rel);
        sleep_until(when).await;
        stdin
            .write_all(frame)
            .expect(format!("{} write fail", PREFIX_FFMPEG).as_str());
    }

    drop(stdin);
    ffmpeg
        .wait()
        .expect(format!("{} ffmpeg wait fail", PREFIX_FFMPEG).as_str());

    std::fs::remove_file("/tmp/wayclip.sock")
        .expect(format!("{} failed to remove socket", PREFIX_INIT).as_str());
    println!("done: check output.mp4");
}
