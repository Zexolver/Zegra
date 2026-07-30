pub mod commands;
pub mod http_client;
pub mod library;
pub mod models;
pub mod platforms;
pub mod settings;
pub mod theme;
pub mod vdf;

use std::sync::Mutex;

use tauri::{Emitter, Listener, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first: it's what lets a second app instance
        // (spawned by the OS when the user clicks an nxm:// link, since
        // Linux/Windows don't emit deep-link events directly — see the
        // deep-link plugin's docs) hand its CLI argument off to this
        // already-running instance instead of opening a second window.
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
            // The `deep-link` feature on this plugin automatically forwards
            // `_args` into the deep-link plugin before this callback runs,
            // which re-emits `deep-link://new-url` in *this* process.
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config_dir = app
                .path()
                .app_config_dir()
                .expect("failed to resolve app config directory");
            app.manage(AppState {
                config_dir: Mutex::new(config_dir),
            });

            // Best-effort: register Zegra as the nxm:// handler so clicking
            // a real Nexus Mods "Mod Manager Download" button launches
            // straight into Zegra. This can fail (e.g. missing xdg-mime on
            // an unusual Linux setup, or on an OS where dynamic registration
            // isn't supported) without that being fatal to the rest of the
            // app — the manual "paste a link" fallback in Settings still
            // works either way.
            if let Err(e) = app.deep_link().register_all() {
                eprintln!("could not register nxm:// deep link handler: {e}");
            }

            // Fires both when this process itself was launched via an
            // nxm:// link (cold start) and, thanks to the single-instance
            // plugin above, when a *second* launch handed its link off to
            // this already-running instance.
            let app_handle = app.handle().clone();
            app.listen("deep-link://new-url", move |event| {
                let urls: Vec<String> = serde_json::from_str(event.payload()).unwrap_or_default();
                let Some(nxm_url) = urls.into_iter().find(|u| u.starts_with("nxm://")) else {
                    return;
                };
                let message = match commands::handle_nxm_url(&app_handle, &nxm_url) {
                    Ok(url) => format!("resolved: {url}"),
                    Err(e) => format!("error: {e}"),
                };
                let _ = app_handle.emit("nxm-download-result", message);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::scan_library,
            commands::load_custom_theme,
            commands::preview_theme,
            commands::open_mod_site,
            commands::open_downloads_folder,
            commands::resolve_and_open_nxm_link,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Zegra");
}
