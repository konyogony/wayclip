use ashpd::desktop::{
    screencast::{CursorMode, Screencast, SourceType},
    PersistMode,
};
use gst::prelude::{Cast, ElementExt, GstBinExt, ObjectExt};
use gstreamer::{self as gst};
use gstreamer_app::AppSink;
use std::env;
use std::error::Error;
use std::fs::{create_dir_all, metadata, remove_file};
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
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use wayclip_core::{
    cleanup, generate_preview_clip, get_pipewire_node_id, handle_bus_messages, log_to,
    logging::Logger, ring::RingBuffer, send_status_to_gui, settings::Settings, setup_hyprland,
};

const SAVE_COOLDOWN: Duration = Duration::from_secs(2);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let settings = Settings::load().await?;
    let log_dir = "/tmp/wayclip";
    create_dir_all(log_dir).expect("Failed to create log directory");
    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();

    let logger = Logger::new(format!("{log_dir}/wayclip-{timestamp}.log"))
        .expect("Failed to create daemon logger");

    log_to!(logger, Info, [DAEMON] => "Starting...");
    log_to!(logger, Debug, [DAEMON] => "Settings loaded: {:?}", settings);

    env::set_var(
        "GST_DEBUG",
        "pipewiresrc:4,audiomixer:4,audioconvert:4,audioresample:4,opusenc:4,matroskamux:4,3",
    );
    gst::init().expect("Failed to init gstreamer");
    if metadata(&settings.daemon_socket_path).is_ok() {
        if let Err(e) = remove_file(&settings.daemon_socket_path) {
            log_to!(logger, Error, [UNIX] => "Failed to remove existing daemon socket file: {}", e);
            exit(1);
        }
    }

    send_status_to_gui(
        settings.gui_socket_path.clone(),
        String::from("Starting"),
        &logger,
    );

    let listener =
        UnixListener::bind(&settings.daemon_socket_path).expect("Failed to bind unix socket");

    if std::env::var("DESKTOP_SESSION") == Ok("hyprland".to_string()) {
        setup_hyprland(&logger).await;
    } else {
        log_to!(logger, Info, [HYPR] => "Not using hyprland. Please bind Alt+C to trigger save.");
    }

    let proxy = Screencast::new()
        .await
        .expect("Failed to create screencast proxy");
    let session = proxy
        .create_session()
        .await
        .expect("Failed to create screencast session");
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
        .expect("Failed to select sources");

    log_to!(logger, Info, [ASH] => "Starting screencast session");
    let response = proxy
        .start(&session, None)
        .await
        .expect("Failed to start screencast session")
        .response()
        .expect("Failed to get screencast response");
    let stream = response
        .streams()
        .first()
        .expect("No streams found in response");
    let node_id = stream.pipe_wire_node_id();
    log_to!(logger, Info, [ASH] => "Streams: {:?}", stream);

    let pipewire_fd = proxy
        .open_pipe_wire_remote(&session)
        .await
        .expect("Failed to open pipewire remote");
    log_to!(logger, Info, [ASH] => "Pipewire fd: {:?}", pipewire_fd.as_raw_fd());

    tokio::time::sleep(Duration::from_millis(200)).await;

    let clip_duration = gst::ClockTime::from_seconds(settings.clip_length_s);
    let ring_buffer = Arc::new(Mutex::new(RingBuffer::new(clip_duration, &logger)));
    let is_saving = Arc::new(AtomicBool::new(false));

    let mut pipeline_parts = Vec::new();
    let has_audio = settings.include_bg_audio || settings.include_mic_audio;
    pipeline_parts.push("matroskamux name=mux ! appsink name=sink".to_string());

    // Mine: Little outdated
    // pipeline_parts.push(format!(
    //     "pipewiresrc do-timestamp=true fd={fd} path={path} ! \
    //     queue ! \
    //     video/x-raw,format=BGRx ! \
    //     videoconvert ! \
    //     video/x-raw,format=NV12 ! \
    //     videorate ! \
    //     video/x-raw,framerate={fps}/1 ! \
    //     cudaupload ! \
    //     nvh264enc bitrate={bitrate} ! \
    //     h264parse ! \
    //     queue ! \
    //     mux.video_0",
    //     fd = pipewire_fd.as_raw_fd(),
    //     path = node_id,
    //     fps = settings.clip_fps,
    //     bitrate = settings.video_bitrate
    // ));

    // Mine: Last working one with frames
    // pipeline_parts.push(format!(
    //     "pipewiresrc do-timestamp=true fd={fd} path={path} ! \
    //     queue ! \
    //     video/x-raw,format=BGRx ! \
    //     videoconvert ! \
    //     video/x-raw,format=NV12 ! \
    //     videorate ! \
    //     video/x-raw,framerate={fps}/1 ! \
    //     cudaupload ! \
    //     nvh264enc bitrate={bitrate} ! \
    //     h264parse ! \
    //     queue ! \
    //     mux.video_0",
    //     fd = pipewire_fd.as_raw_fd(),
    //     path = node_id,
    //     fps = settings.clip_fps,
    //     bitrate = settings.video_bitrate
    // ));

    // GROK
    // pipeline_parts.push(format!(
    //     "pipewiresrc do-timestamp=true fd={fd} path={path} ! \
    // queue ! \
    // video/x-raw(memory:SystemMemory),format=BGRx ! \
    // videoconvert ! \
    // video/x-raw,format=NV12 ! \
    // videorate ! \
    // video/x-raw,framerate={fps}/1 ! \
    // cudaupload ! \
    // nvh264enc bitrate={bitrate} ! \
    // h264parse ! \
    // queue ! \
    // mux.video_0",
    //     fd = pipewire_fd.as_raw_fd(),
    //     path = node_id,
    //     fps = settings.clip_fps,
    //     bitrate = settings.video_bitrate
    // ));

    // GPT-5 MINI
    pipeline_parts.clear();
    // Removed  streamable=true
    pipeline_parts
        .push("matroskamux name=mux ! queue max-size-buffers=2 ! appsink name=sink".to_string());

    let (width, height) = {
        let parts: Vec<&str> = settings.clip_resolution.split('x').collect();
        if parts.len() == 2 {
            let w = parts[0].parse::<i32>().unwrap_or(1920);
            let h = parts[1].parse::<i32>().unwrap_or(1080);
            (w, h)
        } else {
            log_to!(logger, Warn, [DAEMON] => "Invalid video_resolution format '{}'. Using default 1920x1080.", settings.clip_resolution);
            (1920, 1080)
        }
    };

    log_to!(logger, Info, [GST] => "Setting output resolution to {}x{}", width, height);

    // Before resolution update
    // pipeline_parts.push(format!(
    //     "pipewiresrc do-timestamp=true fd={fd} path={path} ! \
    //     queue max-size-buffers=8 leaky=downstream ! \
    //     videoconvert ! videoscale ! \
    //     video/x-raw,format=(string)NV12 ! \
    //     videorate ! video/x-raw,framerate={fps}/1 ! \
    //     queue max-size-buffers=8 leaky=downstream ! \
    //     cudaupload ! nvh264enc bitrate={bitrate} ! \
    //     h264parse ! queue ! mux.video_0",
    //     fd = pipewire_fd.as_raw_fd(),
    //     path = node_id,
    //     fps = settings.clip_fps,
    //     bitrate = settings.video_bitrate,
    // ));

    pipeline_parts.push(format!(
        "pipewiresrc do-timestamp=true fd={fd} path={path} ! \
        queue max-size-buffers=8 leaky=downstream ! \
        videoconvert ! videoscale ! \
        video/x-raw,width={width},height={height},format=(string)NV12 ! \
        videorate ! video/x-raw,framerate={fps}/1 ! \
        queue max-size-buffers=8 leaky=downstream ! \
        cudaupload ! nvh264enc bitrate={bitrate} ! \
        h264parse  config-interval=-1 ! queue ! mux.video_0",
        fd = pipewire_fd.as_raw_fd(),
        path = node_id,
        fps = settings.clip_fps,
        width = width,
        height = height,
        bitrate = settings.video_bitrate,
    ));

    if has_audio {
        pipeline_parts
            .push("audiomixer name=mix ! audioconvert ! audio/x-raw,channels=2 ! opusenc ! opusparse ! queue ! mux.audio_0".to_string());

        if settings.include_bg_audio {
            log_to!(logger, Info,
                [GST] => "Enabling DESKTOP audio recording for device {}",
                settings.bg_node_name
            );
            match get_pipewire_node_id(&settings.bg_node_name, &logger).await {
                Ok(bg_node_id) => {
                    pipeline_parts.push(format!(
                        "pipewiresrc do-timestamp=true path={bg_node_id} ! \
                        queue ! \
                        audio/x-raw,rate=48000,channels=2 ! \
                        audioconvert ! audioresample ! mix.sink_0",
                    ));
                    //queue max-size-buffers=8 ! audioconvert ! audioresample ! mix.sink_1",
                }
                Err(e) => {
                    log_to!(logger, Error, [GST] => "Could not find monitor source '{}': {}. Background audio will not be recorded.", settings.bg_node_name, e);
                }
            }
        }

        if settings.include_mic_audio {
            log_to!(logger, Info,
                [GST] => "Enabling MICROPHONE audio recording for device {}",
                settings.mic_node_name
            );
            match get_pipewire_node_id(&settings.mic_node_name, &logger).await {
                Ok(mic_node_id) => {
                    pipeline_parts.push(format!(
                        "pipewiresrc do-timestamp=true path={mic_node_id} ! \
                        queue ! \
                        audio/x-raw,rate=48000,channels=2 ! \
                        audioconvert ! audioresample ! mix.sink_1",
                    ));
                    //queue max-size-buffers=8 ! audioconvert ! audioresample ! mix.sink_1",
                }
                Err(e) => {
                    log_to!(logger, Error, [GST] => "Could not find microphone source '{}': {}. Mic audio will not be recorded.", settings.mic_node_name, e);
                }
            }
        }
    }
    // pipeline_parts.push(
    //     "audiomixer name=mix ! \
    //       opusenc ! opusparse ! queue ! mux.audio_0"
    //         .to_string(),
    // );

    // if settings.include_desktop_audio {
    //     log_to!(logger, Info,
    //         [GST] => "Enabling DESKTOP audio recording for device {}",
    //         DESKTOP_AUDIO_DEVICE_ID
    //     );
    //     pipeline_parts.push(format!(
    //         "pipewiresrc do-timestamp=true path={DESKTOP_AUDIO_DEVICE_ID} ! \
    //           audio/x-raw ! queue ! audioconvert ! audioresample ! mix.sink_0"
    //     ));
    // }

    // if settings.include_mic_audio {
    //     log_to!(logger, Info,
    //         [GST] => "Enabling MICROPHONE audio recording for device {}",
    //         MIC_AUDIO_DEVICE_ID
    //     );
    //     pipeline_parts.push(format!(
    //         "pipewiresrc do-timestamp=true path={MIC_AUDIO_DEVICE_ID} ! \
    //           audio/x-raw ! queue ! audioconvert ! audioresample ! mix.sink_1"
    //     ));
    // }

    // pipeline_parts.push("matroskamux name=mux streamable=true ! appsink name=sink".to_string());

    // pipeline_parts.push(format!(
    //     "pipewiresrc do-timestamp=true fd={fd} path={path} ! "
    //     // REMOVED rigid format request. Let videoconvert negotiate automatically.
    //     // This is the key to fixing the "no more input formats" error.
    //     "videoconvert ! "
    //     "videoscale ! "
    //     "video/x-raw,format=NV12,framerate={fps}/1 ! "
    //     // Queues for synchronization and buffering.
    //     "queue max-size-time=3000000000 leaky=2 ! "
    //     "cudaupload ! nvh264enc bitrate={bitrate} ! h264parse ! "
    //     "queue max-size-time=3000000000 leaky=2 ! "
    //     "mux.video_0",
    //     fd = pipewire_fd.as_raw_fd(),
    //     path = node_id,
    //     fps = settings.clip_fps,
    //     bitrate = settings.video_bitrate
    // ));

    // if has_audio {
    //     let mut audio_pipeline_parts = Vec::new();
    //     audio_pipeline_parts.push(
    //         "audiomixer name=mix ! opusenc ! opusparse ! queue max-size-time=3000000000 ! mux.audio_0"
    //             .to_string(),
    //     );
    //     if settings.include_desktop_audio {
    //         log_to!(logger, Info, [GST] => "Enabling DESKTOP audio recording for device {}", DESKTOP_AUDIO_DEVICE_ID);
    //         audio_pipeline_parts.push(format!(
    //             "pipewiresrc do-timestamp=true path={DESKTOP_AUDIO_DEVICE_ID} ! audio/x-raw ! queue ! audioconvert ! audioresample ! mix.sink_0"
    //         ));
    //     }
    //     if settings.include_mic_audio {
    //         log_to!(logger, Info, [GST] => "Enabling MICROPHONE audio recording for device {}", MIC_AUDIO_DEVICE_ID);
    //         audio_pipeline_parts.push(format!(
    //             "pipewiresrc do-timestamp=true path={MIC_AUDIO_DEVICE_ID} ! audio/x-raw ! queue ! audioconvert ! audioresample ! mix.sink_1"
    //         ));
    //     }
    //     pipeline_parts.push(audio_pipeline_parts.join(" "));
    // }

    let pipeline_str = pipeline_parts.join(" ");

    log_to!(logger, Info, [GST] => "Parsing pipeline: {}", pipeline_str);
    let pipeline = gst::parse::launch(&pipeline_str).expect("Failed to parse pipeline");

    let pipeline_bin = pipeline
        .clone()
        .dynamic_cast::<gst::Bin>()
        .expect("Pipeline should be a Bin");

    if settings.include_bg_audio {
        let desktop_vol = settings.bg_volume as f64 / 100.0;
        let sink_0_pad = pipeline_bin
            .by_name("mix")
            .expect("Failed to get mixer")
            .static_pad("sink_0")
            .expect("Failed to get mixer sink_0");
        sink_0_pad.set_property("volume", desktop_vol);
        log_to!(logger, Info, [GST] => "Set desktop audio volume to {}", desktop_vol);
    }

    if settings.include_mic_audio {
        let mic_vol = settings.mic_volume as f64 / 100.0;
        let sink_1_pad = pipeline_bin
            .by_name("mix")
            .expect("Failed to get mixer")
            .static_pad("sink_1")
            .expect("Failed to get mixer sink_1");
        sink_1_pad.set_property("volume", mic_vol);
        log_to!(logger, Info, [GST] => "Set mic audio volume to {}", mic_vol);
    }

    let appsink = pipeline_bin
        .by_name("sink")
        .expect("Failed to get appsink")
        .dynamic_cast::<AppSink>()
        .expect("Failed to cast to appsink");

    appsink.set_property("drop", true);
    appsink.set_property("max-buffers", 5_u32);

    let rb_clone = ring_buffer.clone();
    let logger_clone_for_callback = logger.clone();
    appsink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                let pts = buffer.pts();
                let is_header = buffer.flags().contains(gst::BufferFlags::HEADER);
                let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;
                let data = map.as_slice().to_vec();

                log_to!(logger_clone_for_callback, Warn, [DEBUG] => "Writing chunk to file. PTS: {:?}, Size: {}, IsHeader: {}", pts, data.len(), is_header);
                if let Ok(mut rb) = rb_clone.try_lock() {
                    rb.push(data, is_header, pts);
                } else {
                    log_to!(logger_clone_for_callback, Warn, [RING] => "Failed to acquire lock on ring buffer, frame dropped.");
                }

                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    tokio::spawn(handle_bus_messages(
        pipeline.clone().dynamic_cast::<gst::Pipeline>().unwrap(),
        logger.clone(),
    ));

    log_to!(logger, Info, [GST] => "Setting pipeline to playing for constant recording");
    if let Err(err) = pipeline.set_state(gst::State::Playing) {
        log_to!(logger, Error, [GST] => "Failed to set pipeline to playing: {:?}", err);
        pipeline
            .set_state(gst::State::Null)
            .expect("Failed to set pipeline to null after error");
        exit(1);
    }
    send_status_to_gui(
        settings.gui_socket_path.clone(),
        String::from("Recording"),
        &logger,
    );

    let (tx, mut rx): (Sender<String>, Receiver<String>) = channel(32);

    let listener_logger = logger.clone();
    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let mut reader = BufReader::new(stream);
                let mut buf = String::new();
                loop {
                    buf.clear();
                    match reader.read_line(&mut buf).await {
                        Ok(0) => break,
                        Ok(_) => {
                            let msg = buf.trim().to_string();
                            log_to!(listener_logger, Info, [UNIX] => "Message received: {}", msg);
                            if tx.send(msg).await.is_err() {
                                log_to!(listener_logger, Error, [UNIX] => "Receiver dropped, cannot send message.");
                                break;
                            }
                        }
                        Err(e) => {
                            log_to!(listener_logger, Error, [UNIX] => "Failed to read from socket: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    });

    let job_id_counter = Arc::new(AtomicUsize::new(1));
    let mut last_save_time = Instant::now() - SAVE_COOLDOWN;
    let mut term_signal =
        signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                log_to!(logger, Info, [DAEMON] => "Ctrl+C received, initiating shutdown.");
                break;
            },
            _ = term_signal.recv() => {
                log_to!(logger, Info, [DAEMON] => "SIGTERM received, initiating shutdown.");
                break;
            },

            Some(msg) = rx.recv() => {
                match msg.as_str() {
                    "save" => {
                        if last_save_time.elapsed() < SAVE_COOLDOWN {
                            log_to!(logger, Warn, [UNIX] => "Ignoring save request: Cooldown active.");
                            continue;
                        }
                        send_status_to_gui(settings.gui_socket_path.clone(), String::from("Saving clip..."), &logger);

                        if is_saving.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                            last_save_time = Instant::now();
                            let job_id = job_id_counter.fetch_add(1, Ordering::SeqCst);
                            log_to!(logger, Info, [UNIX] => "[JOB {}] Save command received, starting process.", job_id);

                            let wait_ms = 1000u64;
                            let mut waited = 0u64;
                            let saved_chunks = loop {
                                let chunks = {
                                    let mut rb = ring_buffer.lock().unwrap();
                                    rb.get_and_clear()
                                };
                                if !chunks.is_empty() {
                                    break chunks;
                                }
                                if waited >= wait_ms {
                                    break Vec::new();
                                }
                                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                                waited += 50;
                            };

                            let is_saving_clone = is_saving.clone();
                            let settings_clone = settings.clone();
                            let ffmpeg_logger = logger.clone();
                            tokio::spawn(async move {
                                log_to!(ffmpeg_logger, Info, [FFMPEG] => "[JOB {}] Spawning to save {} Matroska chunks.", job_id, saved_chunks.len());


                                if saved_chunks.is_empty() {
                                    log_to!(ffmpeg_logger, Warn, [FFMPEG] => "[JOB {}] No chunks in buffer after waiting {}ms. Aborting.", job_id, wait_ms);
                                    is_saving_clone.store(false, Ordering::SeqCst);
                                    return;
                                }

                                let home_dir = env::var("HOME").expect("HOME not set");
                                let output_dir = std::path::Path::new(&home_dir).join(&settings_clone.save_path_from_home_string);
                                create_dir_all(&output_dir).expect("Failed to create output directory");
                                let output_filename = output_dir.join(format!("{}.mp4", chrono::Local::now().format(&settings_clone.clip_name_formatting)));

                                let mut ffmpeg_child = Command::new("ffmpeg").args([
                                    "-y",
                                    "-i",
                                    "-",
                                    "-c:v",
                                    "copy",
                                    "-c:a",
                                    "copy",
                                    output_filename.to_str().unwrap()
                                ])
                                    .stdin(Stdio::piped())
                                    .stdout(Stdio::null())
                                    .stderr(Stdio::piped())
                                    .spawn()
                                    .expect("Failed to spawn ffmpeg");
                                let mut stdin = ffmpeg_child.stdin.take().expect("Failed to get ffmpeg stdin");
                                let mut stderr = BufReader::new(ffmpeg_child.stderr.take().unwrap());

                                let logger_for_stderr = ffmpeg_logger.clone();
                                tokio::spawn(async move {
                                    let mut error_output = String::new();
                                    use tokio::io::AsyncReadExt;
                                    stderr.read_to_string(&mut error_output).await.unwrap();
                                    if !error_output.is_empty() {
                                        log_to!(logger_for_stderr, Warn, [FFMPEG] => "[JOB {}] {}", job_id, error_output.trim());
                                    }
                                });

                                for chunk in saved_chunks {
                                    if let Err(e) = stdin.write_all(&chunk).await {
                                        log_to!(ffmpeg_logger, Error, [FFMPEG] => "[JOB {}] Process failed while writing chunks: {}", job_id, e);
                                        break;
                                    }
                                }
                                drop(stdin);

                                match ffmpeg_child.wait().await {
                                    Ok(status) if status.success() => {
                                        log_to!(ffmpeg_logger, Info, [FFMPEG] => "[JOB {}] Done! Saved to {:?}", job_id, output_filename);
                                        send_status_to_gui(settings_clone.gui_socket_path.clone(), String::from("Saved!"), &ffmpeg_logger);
                                        let gui_path = settings_clone.gui_socket_path.clone();
                                        let ffmpeg_logger_clone = ffmpeg_logger.clone();
                                        tokio::spawn(async move {
                                            if let Err(e) = generate_preview_clip(&output_filename, &Settings::config_path().join("wayclip").join("previews")).await {
                                                log_to!(&ffmpeg_logger_clone, Error, [FFMPEG] => "Failed to generate preview, {}", e)
                                            };
                                            send_status_to_gui(gui_path, String::from("Saved!"), &ffmpeg_logger_clone);
                                        });
                                    },
                                    Ok(status) => {
                                        log_to!(ffmpeg_logger, Error, [FFMPEG] => "[JOB {}] Exited with error: {}", job_id, status);
                                        send_status_to_gui(settings_clone.gui_socket_path.clone(), String::from("Error during saving"), &ffmpeg_logger);
                                    },
                                    Err(e) => {
                                        log_to!(ffmpeg_logger, Error, [FFMPEG] => "[JOB {}] Process failed: {}", job_id, e);
                                    }
                                }
                                is_saving_clone.store(false, Ordering::SeqCst);
                                log_to!(ffmpeg_logger, Info, [FFMPEG] => "[JOB {}] Task finished and save lock released.", job_id);
                            });
                        } else {
                            log_to!(logger, Warn, [UNIX] => "Ignoring save request: A save is already in progress.");
                        }
                    }
                    "exit" => {
                        log_to!(logger, Info, [UNIX] => "Exit command received, initiating shutdown.");
                        break;
                    }
                    _ => {
                        log_to!(logger, Warn, [UNIX] => "Unknown message received: {}", msg);
                    }
                }
            },
            else => {
                log_to!(logger, Warn, [DAEMON] => "Listener channel closed. Shutting down.");
                break;
            }
        }
    }

    cleanup(&pipeline, &session, settings, logger).await;
    Ok(())
}
