//! Local Epic Games Store library detection.
//!
//! Epic has no official Linux client and no public "list my installed games"
//! API. On Linux (and for many Windows users too) the de-facto standard is
//! the open-source **Legendary** CLI, whose install-state format is also
//! consumed by Heroic Games Launcher. Both write an `installed.json` file
//! mapping app name -> metadata; we read that file directly instead of
//! requiring an official Epic SDK/credentials.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

use crate::models::{Game, Platform};

/// Candidate locations for a Legendary-format `installed.json`.
pub fn default_legendary_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs::home_dir() {
        // Standalone Legendary CLI.
        paths.push(home.join(".config/legendary/installed.json"));
        // Heroic Games Launcher bundles its own Legendary config directory.
        paths.push(home.join(".config/heroic/legendaryConfig/legendary/installed.json"));
    }
    if let Some(config_dir) = dirs::config_dir() {
        paths.push(config_dir.join("legendary/installed.json"));
    }
    paths
}

#[derive(Debug, Deserialize)]
struct LegendaryInstalledEntry {
    #[allow(dead_code)]
    app_name: Option<String>,
    title: String,
    install_path: Option<String>,
}

/// Reads each candidate `installed.json` path and merges the games found.
pub fn scan(installed_json_paths: &[PathBuf]) -> Vec<Game> {
    let mut games = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for path in installed_json_paths {
        let Ok(contents) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(entries) = serde_json::from_str::<HashMap<String, LegendaryInstalledEntry>>(&contents)
        else {
            continue;
        };
        for (app_name, entry) in entries {
            let mut game = Game::new(Platform::Epic, app_name, entry.title);
            if let Some(install_path) = entry.install_path {
                game = game.with_install_path(install_path);
            }
            if seen_ids.insert(game.id.clone()) {
                games.push(game);
            }
        }
    }
    games
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn scans_legendary_installed_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("installed.json");
        fs::write(
            &path,
            r#"{
                "Fortnite": {
                    "app_name": "Fortnite",
                    "title": "Fortnite",
                    "install_path": "/home/user/Games/Heroic/Fortnite"
                }
            }"#,
        )
        .unwrap();

        let games = scan(&[path]);
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].id, "epic:Fortnite");
        assert_eq!(games[0].name, "Fortnite");
        assert_eq!(
            games[0].install_path.as_deref().unwrap(),
            "/home/user/Games/Heroic/Fortnite"
        );
    }

    #[test]
    fn missing_file_yields_no_games_without_error() {
        let games = scan(&[PathBuf::from("/does/not/exist/installed.json")]);
        assert!(games.is_empty());
    }

    #[test]
    fn merges_multiple_candidate_files_without_duplicates() {
        let dir = tempdir().unwrap();
        let legendary = dir.path().join("legendary/installed.json");
        let heroic = dir.path().join("heroic/installed.json");
        fs::create_dir_all(legendary.parent().unwrap()).unwrap();
        fs::create_dir_all(heroic.parent().unwrap()).unwrap();
        fs::write(
            &legendary,
            r#"{"GameA": {"title": "Game A", "install_path": "/games/A"}}"#,
        )
        .unwrap();
        fs::write(
            &heroic,
            r#"{"GameA": {"title": "Game A", "install_path": "/games/A"}, "GameB": {"title": "Game B", "install_path": "/games/B"}}"#,
        )
        .unwrap();

        let games = scan(&[legendary, heroic]);
        assert_eq!(games.len(), 2);
    }
}
