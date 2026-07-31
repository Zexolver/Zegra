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
    /// Mod sites the user has added beyond the built-in Nexus Mods/CurseForge
    /// ones (e.g. ModDB, GameBanana, Thunderstore, Modrinth, or any other
    /// modding site), shown as extra tabs on the Mods tab.
    #[serde(default)]
    pub custom_mod_sites: Vec<CustomModSite>,
}

/// A user-added modding site: `id` is a short slug used internally (as the
/// window label and to match the Mods tab's tab button to it), `name` is
/// what's shown on the tab, and `url` is where it opens.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomModSite {
    pub id: String,
    pub name: String,
    pub url: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("failed to read settings file: {0}")]
    Read(String),
    #[error("failed to write settings file: {0}")]
    Write(String),
    #[error("failed to parse settings file: {0}")]
    Parse(String),
    #[error("invalid settings: {0}")]
    Validation(String),
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
/// directory if necessary. Rejects invalid custom mod sites (bad/non-http
/// URLs, empty names, id collisions) rather than silently persisting them.
pub fn save(config_dir: &Path, settings: &Settings) -> Result<(), SettingsError> {
    crate::mod_sites::validate_all(&settings.custom_mod_sites)
        .map_err(|e| SettingsError::Validation(e.to_string()))?;
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
    fn save_roundtrips_custom_mod_sites() {
        let dir = tempdir().unwrap();
        let mut settings = Settings::default();
        settings.custom_mod_sites.push(CustomModSite {
            id: "moddb".to_string(),
            name: "ModDB".to_string(),
            url: "https://www.moddb.com".to_string(),
        });
        save(dir.path(), &settings).unwrap();
        assert_eq!(load(dir.path()).unwrap(), settings);
    }

    #[test]
    fn save_rejects_invalid_custom_mod_sites() {
        let dir = tempdir().unwrap();
        let mut settings = Settings::default();
        settings.custom_mod_sites.push(CustomModSite {
            id: "bad".to_string(),
            name: "Bad".to_string(),
            url: "javascript:alert(1)".to_string(),
        });
        assert!(matches!(
            save(dir.path(), &settings),
            Err(SettingsError::Validation(_))
        ));
        // And nothing should have been written.
        assert!(!dir.path().join("settings.json").exists());
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
