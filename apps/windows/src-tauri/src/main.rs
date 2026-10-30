#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = zoomouse_desktop::run() {
        eprintln!("Zoomouse host failed to start: {error}");
        std::process::exit(1);
    }
}
