#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use log;
use env_logger;

fn main() {
    // `graphite --calendar-doctor`: check the calendar setup, print JSON, exit. Never opens the app.
    if std::env::args().any(|a| a == "--calendar-doctor") {
        std::process::exit(app_lib::calendar::doctor::run_cli());
    }
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    // Async logger will be initialized lazily when first needed (after Tauri runtime starts)
    log::info!("Starting application...");
    app_lib::run();
}
