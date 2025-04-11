use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType};
use ashpd::desktop::{PersistMode, Session};
use ashpd::WindowIdentifier;
use enumflags2::BitFlag;
use gstreamer as gst;
use gstreamer::glib::object::Cast;
use gstreamer::prelude::{ElementExt, GstBinExt};
use gstreamer_app::AppSink;
use std::collections::VecDeque;
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

    fn dump_to_file(&self, path: &str) {
        println!("[ring] dumping {} frames to {}", self.buffer.len(), path);
        let mut file = File::create(path).expect("[ring] failed to create file");
        for (i, chunk) in self.buffer.iter().enumerate() {
            println!("[ring] write chunk {} size {}", i, chunk.len());
            std::io::Write::write_all(&mut file, chunk).expect("[ring] write fail");
        }
    }
}

#[tokio::main]
async fn main() {
    println!("[init] starting...");
    gst::init().expect("[gst] init fail");

    println!("[ashpd] creating screencast proxy");
    let proxy = Screencast::new().await.expect("[ashpd] proxy fail");

    println!("[ashpd] creating session");
    let session = proxy.create_session().await.expect("[ashpd] session fail");

    println!("[ashpd] selecting sources...");
    proxy
        .select_sources(
            &session,
            CursorMode::Hidden,
            BitFlag::empty(),
            false,
            None,
            PersistMode::Application,
        )
        .await
        .expect("[ashpd] select_sources fail");

    println!("[ashpd] starting session...");
    let response = proxy
        .start(&session, None)
        .await
        .expect("[ashpd] start fail")
        .response()
        .expect("[ashpd] response fail");

    println!("[ashpd] got {} streams", response.streams().len());
    let pipewire_fd = proxy
        .open_pipe_wire_remote(&session)
        .await
        .expect("[ashpd] open_pipe_wire_remote fail");
    println!(
        "[ashpd] got pipewire file descriptor: {:?}",
        pipewire_fd.as_raw_fd()
    );

    let ring_buffer = Arc::new(Mutex::new(RingBuffer::new(256)));

    let pipeline_str = format!(
        "pipewiresrc fd={} ! videoconvert ! video/x-raw,format=I420 ! appsink name=sink",
        pipewire_fd.as_raw_fd()
    );
    println!("[gst] pipeline str:\n{}", pipeline_str);

    println!("[gst] parsing pipeline");
    let pipeline = gst::parse::launch(&pipeline_str).expect("[gst] failed to parse pipeline");

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

    println!("[rec] waiting 10 secs");
    tokio::time::sleep(Duration::from_secs(10)).await;

    println!("[gst] stopping pipeline");
    pipeline
        .set_state(gst::State::Null)
        .expect("[gst] failed to stop");

    println!("[file] dumping raw.yuv");
    {
        let rb = ring_buffer.lock().expect("[ring] lock fail for dump");
        rb.dump_to_file("raw.yuv");
    }

    println!("[ffmpeg] converting raw.yuv to output.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-f",
            "rawvideo",
            "-pixel_format",
            "yuv420p",
            "-video_size",
            "2560x1440", // adjust based on ur screen
            "-framerate",
            "30",
            "-i",
            "raw.yuv",
            "output.mp4",
        ])
        .status()
        .expect("[ffmpeg] failed to run");

    println!("[ffmpeg] finished with status: {}", status);

    println!("[done] check output.mp4");
}
