use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

use crate::http_client::ReqwestClient;
use crate::library;
use crate::models::LibraryScanResult;
use crate::platforms::nexus;
use crate::settings::Settings;
use crate::theme;

pub struct AppState {
    pub config_dir: Mutex<PathBuf>,
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    let config_dir = state.config_dir.lock().unwrap().clone();
    crate::settings::load(&config_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(state: State<AppState>, settings: Settings) -> Result<(), String> {
    let config_dir = state.config_dir.lock().unwrap().clone();
    crate::settings::save(&config_dir, &settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scan_library(state: State<AppState>) -> Result<LibraryScanResult, String> {
    let config_dir = state.config_dir.lock().unwrap().clone();
    let settings = crate::settings::load(&config_dir).map_err(|e| e.to_string())?;
    let client = ReqwestClient::default();
    Ok(library::scan_all(&settings, &client))
}

#[tauri::command]
pub fn load_custom_theme(state: State<AppState>) -> Result<Option<String>, String> {
    let config_dir = state.config_dir.lock().unwrap().clone();
    let settings = crate::settings::load(&config_dir).map_err(|e| e.to_string())?;
    match settings.custom_theme_path {
        None => Ok(None),
        Some(path) => theme::load_theme_css(&PathBuf::from(path))
            .map(Some)
            .map_err(|e| e.to_string()),
    }
}

/// Loads a CSS theme from an arbitrary path, without touching saved settings.
/// Used by the Settings UI to preview a theme before it's been saved.
#[tauri::command]
pub fn preview_theme(path: String) -> Result<String, String> {
    theme::load_theme_css(&PathBuf::from(path)).map_err(|e| e.to_string())
}

const MOD_SITES: &[(&str, &str)] = &[
    ("nexusmods", "https://www.nexusmods.com"),
    ("curseforge", "https://www.curseforge.com"),
];

/// Opens (or focuses, if already open) an embedded webview window pointing at
/// the real Nexus Mods or CurseForge website. This is just an embedded
/// browser tab — no scraping, no unofficial API needed — the user browses,
/// logs in, and downloads exactly as they would in a normal browser.
#[tauri::command]
pub fn open_mod_site(app: AppHandle, site: String) -> Result<(), String> {
    let (_, url) = MOD_SITES
        .iter()
        .find(|(name, _)| *name == site)
        .ok_or_else(|| format!("unknown mod site: {site}"))?;
    let label = format!("mods-{site}");

    if let Some(window) = app.get_webview_window(&label) {
        return window.set_focus().map_err(|e| e.to_string());
    }

    let parsed_url: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(parsed_url))
        .title(format!("Zegra — {site}"))
        .inner_size(1200.0, 800.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Opens the OS's default downloads folder in the system file manager, so
/// users can find mod files they just downloaded through an embedded mod
/// site.
#[tauri::command]
pub fn open_downloads_folder(app: AppHandle) -> Result<(), String> {
    let dir = dirs::download_dir().ok_or_else(|| "could not determine the downloads folder".to_string())?;
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<String>)
        .map_err(|e| e.to_string())
}

/// Resolves an `nxm://` link into a real download URL via the Nexus Mods API,
/// then hands it to the OS's default handler (normally the browser) to
/// actually download it. Returns the resolved URL so the caller can show what
/// happened. Shared by the `resolve_and_open_nxm_link` command (manual
/// "paste a link" fallback in the UI) and the `deep-link://new-url` event
/// handler in `lib.rs` (automatic handling of real nxm:// clicks).
pub fn handle_nxm_url(app: &AppHandle, nxm_url: &str) -> Result<String, String> {
    let state = app.state::<AppState>();
    let config_dir = state.config_dir.lock().unwrap().clone();
    let settings = crate::settings::load(&config_dir).map_err(|e| e.to_string())?;
    let api_key = settings
        .nexus_api_key
        .ok_or_else(|| "no Nexus Mods API key configured in Settings".to_string())?;

    let link = nexus::parse_nxm_url(nxm_url).map_err(|e| e.to_string())?;
    let client = ReqwestClient::default();
    let resolved_url =
        nexus::resolve_download_url(&client, &api_key, &link).map_err(|e| e.to_string())?;

    app.opener()
        .open_url(&resolved_url, None::<String>)
        .map_err(|e| e.to_string())?;

    Ok(resolved_url)
}

/// Manual fallback for the UI: paste an nxm:// link (e.g. copied instead of
/// clicked) and resolve+open it the same way an automatic deep-link would.
#[tauri::command]
pub fn resolve_and_open_nxm_link(app: AppHandle, nxm_url: String) -> Result<String, String> {
    handle_nxm_url(&app, &nxm_url)
}
