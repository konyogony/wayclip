use chrono::Local;
use daemonize::Daemonize;
use device_query::{DeviceQuery, DeviceState, Keycode};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use wayclip_shared::Settings;

struct Recorder {
    pub settings: Settings,
    pub temp_output: String,
    pub process: Option<std::process::Child>,
}

impl Recorder {
    pub fn new() -> Self {
        let settings = Settings::load(); // Assuming the settings are loaded here
        Self {
            settings,
            temp_output: String::from("/tmp/wayclip"), // Temporary output folder
            process: None,
        }
    }

    fn start_recording(&mut self) {
        let device_state = DeviceState::new();

        loop {
            // Check for key press: ALT + C
            if self.is_alt_c_pressed(&device_state) {
                println!("ALT+C pressed, stopping current recording (if any)...");
                // Stop the current recording if there is one
                if let Some(ref mut process) = self.process {
                    process.kill().expect("Failed to kill the process");
                    println!("Current recording stopped.");
                }
                // Start recording and save the file when ALT + C is pressed
                println!("Starting new recording...");
                self.record_and_save();
            } else {
                // Continue recording for the fixed duration (2 minutes)
                if self.process.is_none() {
                    println!("No recording in progress. Starting new recording...");
                    self.record();
                }
            }

            // Sleep for a short time to prevent high CPU usage while checking keys
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn is_alt_c_pressed(&self, device_state: &DeviceState) -> bool {
        // Check if the ALT key and C key are pressed together
        let keys = device_state.get_keys();
        if keys.contains(&Keycode::LAlt) && keys.contains(&Keycode::C) {
            println!("ALT + C keys detected.");
            true
        } else {
            false
        }
    }

    fn record(&mut self) {
        Self::output_settings(&self); // Output settings before starting the recording

        fs::create_dir_all(&self.temp_output).expect("Failed to create temp output directory");

        let timestamp = Local::now().format("%Y%m%d%H%M%S").to_string();
        let filename = format!("{}/temporary_recording_{}.mp4", self.temp_output, timestamp);

        println!("Recording started, saving to: {}", filename);

        let process = Command::new("ffmpeg")
            .arg("-f")
            .arg("pipewire") // Video input
            .arg("-i")
            .arg("video=screen") // Screen capture
            .arg("-i")
            .arg("pipewire:audio") // Audio input
            .arg("-c:v")
            .arg("libx264")
            .arg("-c:a")
            .arg("aac")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("-preset")
            .arg("ultrafast")
            .arg("-t")
            .arg(self.settings.clip_length_s.to_string()) // Record for the set duration (in seconds)
            .arg("-f")
            .arg("mp4")
            .arg("-y")
            .arg(&filename)
            .stdout(std::fs::File::create("/tmp/wayclip-daemon-1.out").unwrap())
            .stderr(std::fs::File::create("/tmp/wayclip-daemon-1.err").unwrap())
            .spawn()
            .expect("Failed to start ffmpeg");

        self.process = Some(process); // Save the process to kill later if needed
    }

    fn record_and_save(&self) {
        // Save the temporary recording with a unique filename
        let timestamp = Local::now().format("%Y%m%d%H%M%S").to_string();
        let filename = format!(
            "{}/saved_recording_{}.mp4",
            self.settings.save_path_from_home_string, timestamp
        );

        // Rename the temporary recording file to save it with the new name
        let temp_file = format!("{}/temporary_recording_{}.mp4", self.temp_output, timestamp);
        if Path::new(&temp_file).exists() {
            fs::rename(temp_file, &filename).expect("Failed to rename file");
            println!("Recording saved to: {}", filename);
        } else {
            eprintln!("Temporary recording file not found: {}", temp_file);
        }
    }

    fn output_settings(&self) {
        println!("Settings:");
        println!("Clip Length: {} seconds", self.settings.clip_length_s);
        println!("Save Path: {}", self.settings.save_path_from_home_string);
        println!("Temp Output: {}", self.temp_output);
    }
}
fn main() {
    let daemonize = Daemonize::new()
        .pid_file("/tmp/wayclip-daemon.pid")
        .chown_pid_file(true)
        .working_directory("/tmp")
        .umask(0o027)
        .stdout(std::fs::File::create("/tmp/wayclip-daemon.out").unwrap())
        .stderr(std::fs::File::create("/tmp/wayclip-daemon.err").unwrap());

    match daemonize.start() {
        Ok(_) => {
            println!("Daemonized");
            let mut recorder = Recorder::new();
            recorder.start_recording(); // Start continuous recording
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
