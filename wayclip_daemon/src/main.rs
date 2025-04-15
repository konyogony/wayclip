use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType};
use ashpd::desktop::PersistMode;
use gstreamer as gst;
use gstreamer::prelude::Cast;
use gstreamer::prelude::{ElementExt, GstBinExt};
use gstreamer_app::AppSink;
use std::collections::VecDeque;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::process::Command;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Semaphore;
use tokio::time::{sleep_until, Duration, Instant};
use wayclip_shared::{err, log};

const FRAMES: usize = 3584;
const MAX_FFMPEG_PROCESS: usize = 2;

type Frame = (Vec<u8>, u64);

struct RingBuffer {
    buffer: VecDeque<Frame>,
    capacity: usize,
}

impl RingBuffer {
    fn new(capacity: usize) -> Self {
        log!([RING] => "init w/ cap {}", capacity);
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    fn push(&mut self, data: Vec<u8>, pts: u64) {
        if self.buffer.len() == self.capacity {
            log!([RING] => "buffer full, popping oldest");
            self.buffer.pop_front();
        }
        log!([RING] => "push frame: {} bytes", data.len());
        self.buffer.push_back((data.to_vec(), pts));
    }

    fn get_and_clear(&mut self) -> Vec<Frame> {
        let data: Vec<Frame> = self.buffer.drain(..).collect(); // get all data and clear buffer
        log!([RING] => "get_and_clear, returning {} frames", data.len());
        data
    }
}

async fn setup_hyprland() {
    let output = Command::new("hyprctl")
        .args([
            "keyword",
            "bind",
            "Alt_L,C,exec,/home/kony/Documents/GitHub/wayclip/target/debug/wayclip_trigger",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect(err!([HYPR] => "failed to spawn hyprctl"))
        .wait();

    if let Ok(output) = output {
        if output.success() {
            log!([HYPR] => "bind added successfully");
        } else {
            log!([HYPR] => "bind failed");
            log!([HYPR] => "error: {}", output.to_string());
        }
    } else {
        log!([HYPR] => "failed to add bind hyprctl");
    }
}

#[tokio::main]
async fn main() {
    log!([INIT] => "starting...");
    gst::init().expect(err!([INIT] => "failed to init gstreamer"));

    log!([UNIX] => "starting unix listener");
    let listener = UnixListener::bind("/tmp/wayclip.sock")
        .expect(err!([UNIX] => "failed to bind unix socket (hint: try `rm /tmp/wayclip.sock`)"));

    if std::env::var("DESKTOP_SESSION") == Ok("hyprland".to_string()) {
        log!([HYPR] => "using hyprland");
        setup_hyprland().await;
    } else {
        log!([HYPR] => "not using hyprland");
        log!([HYPR] => "please bind LAlt + C to /home/kony/Documents/GitHub/wayclip/target/debug/wayclip_trigger");
    }

    log!([ASH] => "creating screencast proxy");
    let proxy = Screencast::new()
        .await
        .expect(err!([ASH] => "failed to create screencast proxy"));
    log!([ASH] => "creating screencast session");
    let session = proxy
        .create_session()
        .await
        .expect(err!([ASH] => "failed to create screencast session"));
    log!([ASH] => "selecting sources for screencast");
    proxy
        .select_sources(
            &session,
            CursorMode::Hidden,
            enumflags2::BitFlags::from(SourceType::Monitor),
            false,
            None,
            PersistMode::Application,
        )
        .await
        .expect(err!([ASH] => "failed to select sources"));
    log!([ASH] => "starting screencast session");
    let response = proxy
        .start(&session, None)
        .await
        .expect(err!([ASH] => "failed to start screencast session"))
        .response()
        .expect(err!([ASH] => "failed to get screencast response"));
    log!([ASH] => "streams: {:?}", response.streams());
    let pipewire_fd = proxy
        .open_pipe_wire_remote(&session)
        .await
        .expect(err!([ASH] => "failed to open pipewire remote"));
    log!([ASH] => "pipewire fd: {:?}", pipewire_fd.as_raw_fd());

    let ring_buffer = Arc::new(Mutex::new(RingBuffer::new(FRAMES)));
    let ffmpeg_semaphore = Arc::new(Semaphore::new(MAX_FFMPEG_PROCESS));

    let pipeline_str = format!(
        "pipewiresrc fd={} ! videoconvert ! video/x-raw,format=I420 ! appsink name=sink",
        pipewire_fd.as_raw_fd()
    );
    log!([GST] => "parsing pipeline: {}", pipeline_str);
    let pipeline =
        gst::parse::launch(&pipeline_str).expect(err!([GST] => "failed to parse pipeline"));

    log!([GST] => "getting appsink element");
    let appsink = pipeline
        .clone()
        .dynamic_cast::<gst::Bin>()
        .expect(err!([GST] => "failed to cast pipeline to bin"))
        .by_name("sink")
        .expect(err!([GST] => "failed to get appsink"))
        .dynamic_cast::<AppSink>()
        .expect(err!([GST] => "failed to cast to appsink"));

    let rb_clone = ring_buffer.clone();
    log!([GST] => "setting appsink callbacks for constant recording");
    appsink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = sink
                    .pull_sample()
                    .expect(err!([GST] => "failed to pull sample"));
                let buffer = sample
                    .buffer()
                    .expect(err!([GST] => "failed to get buffer"));
                let map = buffer
                    .map_readable()
                    .expect(err!([GST] => "failed to map buffer"));
                let data = map.as_slice().to_vec();
                let pts = buffer.pts().unwrap().nseconds() / 1000;

                log!([GST] => "got sample, size: {}, pts: {}", data.len(), pts);
                if let Ok(mut rb) = rb_clone.lock() {
                    rb.push(data, pts);
                } else {
                    log!([RING] => "failed to lock ring buffer");
                }
                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    log!([GST] => "setting pipeline to playing for constant recording");
    if let Err(err) = pipeline.set_state(gst::State::Playing) {
        eprintln!(
            "{}: {:?}",
            err!([GST] => "failed to set pipeline to playing"),
            err
        );
        pipeline
            .set_state(gst::State::Null)
            .expect(err!([GST] => "failed to set pipeline to null after error"));
        std::process::exit(1);
    } else {
        log!([GST] => "pipeline set to playing");
    }

    let (tx, mut rx): (Sender<String>, Receiver<String>) = channel(32);

    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect(err!([UNIX] => "failed to accept unix socket"));
            let mut buf = [0u8; 64];
            let n = stream
                .read(&mut buf)
                .await
                .expect(err!([UNIX] => "failed to read from unix socket"));
            if n == 0 {
                continue;
            }
            let msg = std::str::from_utf8(&buf[..n]).unwrap_or("").trim();
            log!([UNIX] => "msg: {}", msg);

            tx.send(msg.to_string())
                .await
                .expect("Failed to send message to command queue");
        }
    });

    while let Some(msg) = rx.recv().await {
        match msg.as_str() {
            "save" => {
                log!([UNIX] => "save command received, starting saving process");

                let permit = ffmpeg_semaphore.clone().acquire_owned().await.unwrap();

                log!([GST] => "pausing pipeline for save");
                pipeline
                    .set_state(gst::State::Paused)
                    .expect(err!([GST] => "failed to set pipeline to paused"));

                let saved_frames: Vec<Frame>;
                {
                    let mut buffer = ring_buffer.lock().unwrap();
                    saved_frames = buffer.get_and_clear();
                }

                tokio::spawn(async move {
                    log!([FFMPEG] => "spawning ffmpeg to save clip");
                    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
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
                            format!("output_{}.mp4", timestamp).as_str(),
                        ])
                        .stdin(Stdio::piped())
                        .stdout(Stdio::null())
                        .spawn()
                        .expect(err!([FFMPEG] => "failed to spawn ffmpeg"));

                    let mut stdin = ffmpeg
                        .stdin
                        .take()
                        .expect(err!([FFMPEG] => "failed to get ffmpeg stdin"));
                    let mut last_pts = 0;
                    let start = Instant::now();

                    for (frame, pts) in saved_frames {
                        let rel = pts - last_pts;
                        let when = start + Duration::from_micros(rel);
                        sleep_until(when).await;
                        stdin
                            .write_all(&frame)
                            .expect(err!([FFMPEG] => "failed to write to ffmpeg stdin"));
                        last_pts = pts;
                    }

                    drop(stdin);
                    ffmpeg
                        .wait()
                        .expect(err!([FFMPEG] => "failed to wait for ffmpeg"));
                    log!([FFMPEG] => "ffmpeg done, check output file");
                    drop(permit);
                });

                if let Err(err) = pipeline.set_state(gst::State::Playing) {
                    eprintln!(
                        "{}: {:?}",
                        err!([GST] => "failed to set pipeline to playing"),
                        err
                    );
                    pipeline
                        .set_state(gst::State::Null)
                        .expect(err!([GST] => "failed to set pipeline to null after error"));
                    std::process::exit(1);
                } else {
                    log!([GST] => "pipeline resumed");
                }
            }
            "exit" => {
                log!([UNIX] => "exit command received, exiting cleanly");
                if let Err(e) = std::fs::remove_file("/tmp/wayclip.sock") {
                    eprintln!("Error removing socket file: {}", e);
                }
                if let Err(e) = pipeline.set_state(gst::State::Null) {
                    eprintln!("Error setting pipeline to null: {:?}", e);
                }
                break;
            }
            _ => {
                log!([UNIX] => "unknown msg: {}", msg);
            }
        }
    }
}
