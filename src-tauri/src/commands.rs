use std::path::PathBuf;
use std::sync::Mutex;

use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

use crate::http_client::ReqwestClient;
use crate::library;
use crate::mod_sites;
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

fn mod_window_label(site_id: &str) -> String {
    format!("mods-{site_id}")
}

/// Shows `site`'s window (creating it the first time, otherwise just
/// re-showing/focusing it), hiding every other mod-site window first so only
/// one is ever visible at a time. `site` is either a built-in id
/// (`"nexusmods"`, `"curseforge"`) or the id of one of the user's own custom
/// mod sites from Settings — see `mod_sites::resolve_site`. Pass `site: None`
/// to hide all of them — used when the user navigates away from the Mods
/// tab, so a previously opened mod-site window doesn't linger on top while
/// they work elsewhere.
///
/// `width`/`height` are the logical-pixel size of the Mods tab's placeholder
/// area (as measured by the frontend via `getBoundingClientRect()`), used to
/// size the window so it roughly matches the space reserved for it.
///
/// This is a real, separate OS window rather than content embedded pixel-for
/// -pixel inside the main window: on Linux, Tauri's child-webview API packs
/// child webviews into the same vertical box as the main content instead of
/// letting them be freely positioned/sized (confirmed by testing — see
/// `API-Implementation.md`), so true inline embedding isn't reliably
/// achievable there today. Removing the window's decorations and hiding it
/// from the taskbar is the closest approximation of "it's all the same app"
/// that actually works. The window is reused (shown/focused, not recreated)
/// across visits, so switching sites or returning to the Mods tab is instant
/// and preserves each site's logged-in session.
///
/// Deliberately does *not* try to pin the window's on-screen *position* to
/// the placeholder (no `.position()`/`set_position()`, no `.parent()`):
/// extensive testing (both headless and under a real window manager) found
/// that any explicit position call on this window — at creation or
/// afterwards, parented or not — either left WebKitGTK's surface unpainted
/// (solid black) or was silently ignored by the window manager, depending on
/// the combination. Leaving position out entirely is the one configuration
/// that reliably renders across every environment tested. See
/// `API-Implementation.md` for the full investigation. Sizing the window
/// (which *does* work reliably) and leaving placement to the window
/// manager's own placement policy is the honest tradeoff here.
#[tauri::command]
pub fn set_active_mod_site(
    app: AppHandle,
    state: State<AppState>,
    site: Option<String>,
    width: f64,
    height: f64,
) -> Result<(), String> {
    // Round away any fractional CSS pixels from getBoundingClientRect():
    // fractional logical sizes reliably left the webview's content
    // unpainted (black) in testing under WebKitGTK, even though the window
    // itself ended up the right size.
    let width = width.round();
    let height = height.round();

    let active_label = site.as_deref().map(mod_window_label);
    for (label, window) in app.webview_windows() {
        if label.starts_with("mods-") && Some(&label) != active_label.as_ref() {
            window.hide().map_err(|e| e.to_string())?;
        }
    }

    let Some(site) = site else {
        return Ok(());
    };

    let config_dir = state.config_dir.lock().unwrap().clone();
    let settings = crate::settings::load(&config_dir).map_err(|e| e.to_string())?;
    let resolved =
        mod_sites::resolve_site(&site, &settings.custom_mod_sites).map_err(|e| e.to_string())?;
    let label = mod_window_label(&site);

    if let Some(window) = app.get_webview_window(&label) {
        window
            .set_size(LogicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let parsed_url: url::Url = resolved
        .url
        .parse()
        .map_err(|e: url::ParseError| e.to_string())?;
    let app_for_handler = app.clone();
    let site_for_handler = site.clone();
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(parsed_url))
        .title(format!("Zegra — {}", resolved.title))
        .inner_size(width, height)
        .decorations(false)
        .skip_taskbar(true)
        .on_page_load(move |_window, payload| {
            if matches!(payload.event(), PageLoadEvent::Finished) {
                let _ = app_for_handler.emit("mod-site-loaded", &site_for_handler);
            }
        })
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
