#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::Write;
use std::os::unix::net::UnixStream;

fn main() {
    if let Some(url) = std::env::args().nth(1) {
        if url.starts_with("wayclip://") {
            if let Ok(mut stream) = UnixStream::connect(gui_lib::DEEP_LINK_SOCKET_PATH) {
                if stream.write_all(url.as_bytes()).is_ok() {
                    println!("[wayclip] Deep link sent to main instance. Exiting now.");
                    std::process::exit(0);
                }
            }
        }
    }

    gui_lib::run();
}
