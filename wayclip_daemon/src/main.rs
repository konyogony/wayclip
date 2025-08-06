#![allow(clippy::expect_fun_call)]

use ashpd::desktop::{
    screencast::{CursorMode, Screencast, SourceType},
    PersistMode,
};
use futures::prelude::*;
use gst::prelude::{Cast, ElementExt, GstBinExt, GstObjectExt, ObjectExt};
use gstreamer as gst;
use gstreamer_app::AppSink;
use std::collections::VecDeque;
use std::fs::{metadata, remove_file};
use std::os::unix::io::AsRawFd;
use std::process::{exit, Stdio};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::process::Command;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use wayclip_shared::{err, log};

const FRAMES: usize = 3584;
const SOCKET_PATH: &str = "/tmp/wayclip.sock";
const WAYCLIP_TRIGGER_PATH: &str =
    "/home/kony/Documents/GitHub/wayclip/target/debug/wayclip_trigger";

type Frame = Vec<u8>;

struct RingBuffer {
    header: Vec<Frame>,
    header_complete: bool,
    buffer: VecDeque<Frame>,
    capacity: usize,
}

impl RingBuffer {
    fn new(capacity: usize) -> Self {
        log!([RING] => "init w/ cap {}", capacity);
        Self {
            header: Vec::new(),
            header_complete: false,
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    fn push(&mut self, data: Vec<u8>, is_header: bool) {
        if !self.header_complete && is_header {
            log!([RING] => "Header chunk captured, size: {}", data.len());
            self.header.push(data);
            return;
        }

        if !self.header_complete {
            log!([RING] => "First frame detected (non-header). Header is now complete with {} chunks.", self.header.len());
            log!([INIT] => "Recording successfully started, and live! Ctrl + C for graceful shutdown, ALT + C to save clip (hyprland only so far)");
            self.header_complete = true;
        }

        if self.buffer.len() == self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(data);
    }

    fn get_and_clear(&mut self) -> Vec<Frame> {
        if self.header.is_empty() {
            log!([RING] => "get_and_clear called but no header was ever captured.");
            return Vec::new();
        }

        let mut all_data = self.header.clone();
        all_data.extend(self.buffer.drain(..));

        log!([RING] => "get_and_clear, returning {} header chunks + {} frames", self.header.len(), all_data.len() - self.header.len());
        all_data
    }
}

async fn handle_bus_messages(pipeline: gst::Pipeline) {
    let bus = pipeline.bus().unwrap();
    let mut bus_stream = bus.stream();

    log!([GSTBUS] => "Started bus message handler.");
    while let Some(msg) = bus_stream.next().await {
        use gst::MessageView;
        match msg.view() {
            MessageView::Error(err) => {
                let src_name = err
                    .src()
                    .map_or("None".to_string(), |s| s.path_string().to_string());
                let error_msg = err.error().to_string();
                let debug_info = err.debug().map_or_else(
                    || "No debug info".to_string(),
                    |g_string| g_string.to_string(),
                );

                log!([GSTBUS] => "Error from element {}: {} ({})", src_name, error_msg, debug_info);
                break;
            }
            MessageView::Warning(warning) => {
                let src_name = warning
                    .src()
                    .map_or("None".to_string(), |s| s.path_string().to_string());
                let error_msg = warning.error().to_string();
                let debug_info = warning.debug().map_or_else(
                    || "No debug info".to_string(),
                    |g_string| g_string.to_string(),
                );
                log!([GSTBUS] => "Warning from element {}: {} ({})", src_name, error_msg, debug_info);
            }
            MessageView::Eos(_) => {
                log!([GSTBUS] => "Received End-Of-Stream");
                break;
            }
            MessageView::StateChanged(state) => {
                if state
                    .src()
                    .map_or(false, |s| s.downcast_ref::<gst::Pipeline>().is_some())
                {
                    log!([GSTBUS] => "Pipeline state changed from {:?} to {:?} ({:?})",
                        state.old(),
                        state.current(),
                        state.pending()
                    );
                }
            }
            _ => {}
        }
    }
    log!([GSTBUS] => "Stopped bus message handler.");
}

async fn cleanup(pipeline: &gst::Element) {
    log!([CLEANUP] => "Starting graceful shutdown...");

    if std::env::var("DESKTOP_SESSION") == Ok("hyprland".to_string()) {
        let output = Command::new("hyprctl")
            .args(["keyword", "unbind", "Alt_L,C"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect(err!([HYPR] => "failed to spawn hyprctl for unbind"))
            .wait()
            .await;
        if let Ok(output) = output {
            if output.success() {
                log!([HYPR] => "bind removed successfully");
            } else {
                log!([HYPR] => "failed to remove bind");
            }
        }
    }

    if let Err(e) = pipeline.set_state(gst::State::Null) {
        log!([GST] => "failed to set pipeline to null, {:?}", e);
    } else {
        log!([GST] => "pipeline set to null");
    }

    if let Err(e) = remove_file(SOCKET_PATH) {
        log!([UNIX] => "failed to remove socket file, {}", e);
    } else {
        log!([UNIX] => "socket file removed");
    }

    log!([CLEANUP] => "Graceful shutdown complete.");
}

async fn setup_hyprland() {
    let output = Command::new("hyprctl")
        .args([
            "keyword",
            "bind",
            format!("Alt_L,C,exec,{WAYCLIP_TRIGGER_PATH}").as_str(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect(err!([HYPR] => "failed to spawn hyprctl"))
        .wait()
        .await;

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
    if metadata(SOCKET_PATH).is_ok() {
        if let Err(e) = remove_file(SOCKET_PATH) {
            log!([UNIX] => "failed to remove existing socket file: {}", e);
            exit(1);
        } else {
            log!([UNIX] => "Removed existing socket file.");
        }
    }

    let listener =
        UnixListener::bind(SOCKET_PATH).expect(err!([UNIX] => "failed to bind unix socket"));

    if std::env::var("DESKTOP_SESSION") == Ok("hyprland".to_string()) {
        log!([HYPR] => "using hyprland");
        setup_hyprland().await;
    } else {
        log!([HYPR] => "not using hyprland");
        log!([HYPR] => "please bind LAlt + C to {} in your compositor", WAYCLIP_TRIGGER_PATH);
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

    let stream = response
        .streams()
        .first()
        .expect(err!([ASH] => "no streams found in response"));
    let node_id = stream.pipe_wire_node_id();
    log!([ASH] => "streams: {:?}", stream);

    let pipewire_fd = proxy
        .open_pipe_wire_remote(&session)
        .await
        .expect(err!([ASH] => "failed to open pipewire remote"));
    log!([ASH] => "pipewire fd: {:?}", pipewire_fd.as_raw_fd());

    let ring_buffer = Arc::new(Mutex::new(RingBuffer::new(FRAMES)));
    let is_saving = Arc::new(AtomicBool::new(false));

    // NVIDIA
    let pipeline_str = format!(
    "pipewiresrc fd={0} path={1} ! video/x-raw,format=BGRx ! queue ! videoconvert ! video/x-raw,format=I420 ! queue ! x264enc tune=zerolatency key-int-max=60 ! h264parse ! matroskamux ! appsink name=sink",
    pipewire_fd.as_raw_fd(),
    node_id
);

    // AMD:
    // let pipeline_str = format!(
    // "pipewiresrc fd={0} path={1} ! queue ! video/x-raw,format=BGRx ! queue ! videoconvert ! vaapih264enc ! h264parse ! matroskamux ! appsink name=sink",
    // pipewire_fd.as_raw_fd(),
    // node_id
    // );

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

    appsink.set_property("drop", true);
    appsink.set_property("max-buffers", 5_u32);

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

                let is_header = buffer.flags().contains(gst::BufferFlags::HEADER);

                let map = buffer
                    .map_readable()
                    .expect(err!([GST] => "failed to map buffer"));
                let data = map.as_slice().to_vec();

                if let Ok(mut rb) = rb_clone.try_lock() {
                    rb.push(data, is_header);
                }

                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    tokio::spawn(handle_bus_messages(
        pipeline.clone().dynamic_cast::<gst::Pipeline>().unwrap(),
    ));

    log!([GST] => "setting pipeline to playing for constant recording");
    if let Err(err) = pipeline.set_state(gst::State::Playing) {
        log!([GST] => "setting pipeline to playing, {:?}", err);
        pipeline
            .set_state(gst::State::Null)
            .expect(err!([GST] => "failed to set pipeline to null after error"));
        exit(1);
    } else {
        log!([GST] => "pipeline set to playing");
    }

    let (tx, mut rx): (Sender<String>, Receiver<String>) = channel(32);

    tokio::spawn(async move {
        loop {
            let (stream, _) = listener
                .accept()
                .await
                .expect(err!([UNIX] => "failed to accept unix socket"));
            let mut reader = BufReader::new(stream);
            let mut buf = String::new();
            loop {
                buf.clear();
                match reader.read_line(&mut buf).await {
                    Ok(n) => {
                        if n == 0 {
                            break;
                        }
                        let msg = buf.trim().to_string();
                        log!([UNIX] => "msg: {}", msg);
                        if let Err(e) = tx.send(msg).await {
                            log!([UNIX] => "failed to send message, {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        log!([UNIX] => "failed to read from socket: {}", e);
                        break;
                    }
                }
            }
        }
    });

    let job_id_counter = Arc::new(AtomicUsize::new(1));
    let mut last_save_time = Instant::now() - Duration::from_secs(10);
    const SAVE_COOLDOWN: Duration = Duration::from_secs(2);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                log!([INIT] => "Ctrl+C received, initiating shutdown.");
                break;
            },

            Some(msg) = rx.recv() => {
                match msg.as_str() {
                    "save" => {
                        if last_save_time.elapsed() < SAVE_COOLDOWN {
                            log!([UNIX] => "Ignoring save request: Cooldown active.");
                            continue;
                        }

                        if is_saving
                            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                            .is_ok()
                        {
                            last_save_time = Instant::now();
                            let job_id = job_id_counter.fetch_add(1, Ordering::SeqCst);
                            log!([UNIX] => "[JOB {}] Save command received, starting process.", job_id);

                            let saved_chunks: Vec<Frame>;
                            {
                                let mut buffer = ring_buffer.lock().unwrap();
                                saved_chunks = buffer.get_and_clear();
                            }

                            let is_saving_clone = is_saving.clone();

                            tokio::spawn(async move {
                                log!([FFMPEG] => "[JOB {}] Spawning to save {} Matroska chunks.", job_id, saved_chunks.len());

                                if saved_chunks.is_empty() {
                                    log!([FFMPEG] => "[JOB {}] No chunks in buffer. Aborting.", job_id);
                                    is_saving_clone.store(false, Ordering::SeqCst);
                                    return;
                                }

                                let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
                                let output_filename = format!("output_{timestamp}_{job_id}.mp4");

                                let mut ffmpeg_child = Command::new("ffmpeg")
                                    .args(["-y", "-i", "-", "-c:v", "copy", &output_filename])
                                    .stdin(Stdio::piped())
                                    .stdout(Stdio::null())
                                    .stderr(Stdio::piped())
                                    .spawn()
                                    .expect(err!([FFMPEG] => "failed to spawn ffmpeg"));

                                let mut stdin = ffmpeg_child
                                    .stdin
                                    .take()
                                    .expect("Failed to get ffmpeg stdin");

                                let mut stderr = BufReader::new(ffmpeg_child.stderr.take().unwrap());
                                let job_id_clone = job_id;
                                tokio::spawn(async move {
                                    let mut error_output = String::new();
                                    use tokio::io::AsyncReadExt;
                                    stderr.read_to_string(&mut error_output).await.unwrap();
                                    if !error_output.is_empty() {
                                        log!([FFMPEG] => "[JOB {}] {}", job_id_clone, error_output.trim());
                                    }
                                });

                                for chunk in saved_chunks {
                                    if let Err(e) = stdin.write_all(&chunk).await {
                                        log!([FFMPEG] => "[JOB {}] Process failed while writing chunks: {}", job_id, e);
                                        break;
                                    }
                                }

                                drop(stdin);

                                let output = ffmpeg_child.wait().await;
                                match output {
                                    Ok(status) => {
                                        if status.success() {
                                            log!([FFMPEG] => "[JOB {}] Done! Saved to {}", job_id, output_filename);
                                        } else {
                                            log!([FFMPEG] => "[JOB {}] Exited with error: {}", job_id, status);
                                        }
                                    }
                                    Err(e) => {
                                        log!([FFMPEG] => "[JOB {}] Process failed: {}", job_id, e);
                                    }
                                }
                                is_saving_clone.store(false, Ordering::SeqCst);
                                log!([FFMPEG] => "[JOB {}] Task finished and save lock released.", job_id);
                            });
                        } else {
                            log!([UNIX] => "Ignoring save request: A save is already in progress.");
                        }
                    }
                    "exit" => {
                        log!([UNIX] => "exit command received, initiating shutdown.");
                        break;
                    }
                    _ => {
                        log!([UNIX] => "unknown msg: {}", msg);
                    }
                }
            },

            else => {
                log!([UNIX] => "Listener channel closed. Shutting down.");
                break;
            }
        }
    }

    cleanup(&pipeline).await;
    log!([INIT] => "exiting main");
}
