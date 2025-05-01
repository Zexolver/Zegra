#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

// Basic Tauri setup
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![]) // Add your Rust API calls here
        .run(tauri::generate_context!())
        .expect("error while running Zegra");
}
