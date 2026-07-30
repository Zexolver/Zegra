use std::path::PathBuf;
use std::sync::Mutex;

use tauri::State;

use crate::library;
use crate::models::LibraryScanResult;
use crate::platforms::itchio::ReqwestClient;
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
