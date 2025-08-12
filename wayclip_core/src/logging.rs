use chrono::Local;
use std::fmt;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const INFO_COLOR: &str = "\x1b[34m"; // Blue
const WARN_COLOR: &str = "\x1b[33m"; // Yellow
const ERROR_COLOR: &str = "\x1b[31m"; // Red
const DEBUG_COLOR: &str = "\x1b[35m"; // Purple
const RESET_COLOR: &str = "\x1b[0m";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

impl LogLevel {
    fn color(&self) -> &'static str {
        match self {
            LogLevel::Info => INFO_COLOR,
            LogLevel::Warn => WARN_COLOR,
            LogLevel::Error => ERROR_COLOR,
            LogLevel::Debug => DEBUG_COLOR,
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Info => write!(f, "[INFO]"),
            LogLevel::Warn => write!(f, "[WARN]"),
            LogLevel::Error => write!(f, "[ERROR]"),
            LogLevel::Debug => write!(f, "[DEBUG]"),
        }
    }
}

fn strip_ansi_codes(s: &str) -> String {
    String::from_utf8(strip_ansi_escapes::strip(s.as_bytes())).unwrap_or_default()
}

#[derive(Debug, Clone)]
pub struct Logger {
    log_path: PathBuf,
}

impl Logger {
    pub fn new(log_path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = log_path.as_ref();
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        Ok(Self {
            log_path: path.to_path_buf(),
        })
    }

    pub fn log(&self, level: LogLevel, tag: &str, message: &str) {
        if level != LogLevel::Debug {
            println!(
                "{}{} {}{} {}",
                level.color(),
                level,
                RESET_COLOR,
                tag,
                message
            );
        }

        if let Ok(mut file) = OpenOptions::new().append(true).open(&self.log_path) {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let plain_tag = strip_ansi_codes(tag);
            let file_message = format!("{timestamp} {level} {plain_tag} {message}\n");

            if let Err(e) = file.write_all(file_message.as_bytes()) {
                eprintln!("[LOGGER_ERROR] Failed to write to log file: {e}");
            }
        } else {
            eprintln!(
                "[LOGGER_ERROR] Failed to open log file: {:?}",
                self.log_path
            );
        }
    }
}
