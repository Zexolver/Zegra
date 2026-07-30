pub mod commands;
pub mod library;
pub mod models;
pub mod platforms;
pub mod settings;
pub mod theme;
pub mod vdf;

use std::sync::Mutex;

use tauri::Manager;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config_dir = app
                .path()
                .app_config_dir()
                .expect("failed to resolve app config directory");
            app.manage(AppState {
                config_dir: Mutex::new(config_dir),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::scan_library,
            commands::load_custom_theme,
            commands::preview_theme,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Zegra");
}
