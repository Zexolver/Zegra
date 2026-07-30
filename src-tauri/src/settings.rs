//! Persisted user settings: API keys and extra scan locations the user has
//! configured, plus their chosen custom theme file. Stored as a plain JSON
//! file inside the app's config directory so it's easy to inspect/edit by
//! hand if something goes wrong.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Settings {
    #[serde(default)]
    pub itchio_api_key: Option<String>,
    #[serde(default)]
    pub nexus_api_key: Option<String>,
    #[serde(default)]
    pub custom_theme_path: Option<String>,
    #[serde(default)]
    pub extra_steam_roots: Vec<String>,
    #[serde(default)]
    pub extra_gog_roots: Vec<String>,
    #[serde(default)]
    pub extra_legendary_paths: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("failed to read settings file: {0}")]
    Read(String),
    #[error("failed to write settings file: {0}")]
    Write(String),
    #[error("failed to parse settings file: {0}")]
    Parse(String),
}

fn settings_file_path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.json")
}

/// Loads settings from `<config_dir>/settings.json`. If the file does not
/// exist yet, returns default (empty) settings rather than erroring, since
/// that's simply the state of a fresh install.
pub fn load(config_dir: &Path) -> Result<Settings, SettingsError> {
    let path = settings_file_path(config_dir);
    if !path.exists() {
        return Ok(Settings::default());
    }
    let contents = std::fs::read_to_string(&path).map_err(|e| SettingsError::Read(e.to_string()))?;
    serde_json::from_str(&contents).map_err(|e| SettingsError::Parse(e.to_string()))
}

/// Writes settings to `<config_dir>/settings.json`, creating the config
/// directory if necessary.
pub fn save(config_dir: &Path, settings: &Settings) -> Result<(), SettingsError> {
    std::fs::create_dir_all(config_dir).map_err(|e| SettingsError::Write(e.to_string()))?;
    let path = settings_file_path(config_dir);
    let contents =
        serde_json::to_string_pretty(settings).map_err(|e| SettingsError::Parse(e.to_string()))?;
    std::fs::write(&path, contents).map_err(|e| SettingsError::Write(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_missing_file_returns_defaults() {
        let dir = tempdir().unwrap();
        let settings = load(dir.path()).unwrap();
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempdir().unwrap();
        let mut settings = Settings::default();
        settings.itchio_api_key = Some("secret-key".to_string());
        settings.extra_steam_roots.push("/mnt/games/Steam".to_string());

        save(dir.path(), &settings).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded, settings);
    }

    #[test]
    fn save_creates_missing_config_dir() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested/config/dir");
        save(&nested, &Settings::default()).unwrap();
        assert!(nested.join("settings.json").exists());
    }

    #[test]
    fn corrupt_file_yields_parse_error() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), "not json").unwrap();
        assert!(matches!(load(dir.path()), Err(SettingsError::Parse(_))));
    }
}
