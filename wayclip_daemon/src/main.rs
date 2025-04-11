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
use std::collections::VecDeque;
use std::io::Write;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::{fs::File, os::unix::io::AsRawFd, process::Command};
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

#[tokio::main]
async fn main() {
    println!("[init] starting...");
    gst::init().expect("[gst] init fail");

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

    // --- INPUT CAPTURE ---

    println!("[ashpd] creating a new input capture session");
    let input_capture = ashpd::desktop::input_capture::InputCapture::new()
        .await
        .expect("[ashpd] input capture creation fail");
    let (capture_session, capabilities) = input_capture
        .create_session(None, enumflags2::BitFlags::from(Capabilities::Keyboard))
        .await
        .expect("[ashpd] create_session input capture fail");
    eprintln!("capabilities: {capabilities}");

    let eifd = input_capture
        .connect_to_eis(&capture_session)
        .await
        .expect("[ashpd] connect_to_eis fail");
    eprintln!("eifd: {}", eifd.as_raw_fd());

    // --- GSTREAMER ---

    println!("[ashpd] creating a new ring buffer");
    let ring_buffer = Arc::new(Mutex::new(RingBuffer::new(512))); // 256 frames

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

    let device_state = DeviceState::new();
    println!("[rec] press LAlt + C to stop recording");

    loop {
        let keys = device_state.get_keys();
        if keys.contains(&Keycode::LAlt) && keys.contains(&Keycode::C) {
            println!("[rec] hotkey pressed, stopping...");
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
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

    println!("[done] check output.mp4");
}
