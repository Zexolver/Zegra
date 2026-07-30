//! Local GOG library detection.
//!
//! GOG's Linux and Windows installers drop a `goggame-<id>.info` JSON file
//! next to every installed game (this is the same file GOG Galaxy itself
//! reads, and the format is documented by community tooling such as the
//! `heroic-gogdl` and `gogdl` projects). We walk a small set of candidate
//! install roots looking for these files rather than depending on GOG Galaxy
//! being installed at all.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::models::{Game, Platform};

pub fn default_gog_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join("GOG Games"));
        roots.push(home.join("Games/GOG Games"));
    }
    #[cfg(target_os = "windows")]
    {
        roots.push(PathBuf::from(r"C:\GOG Games"));
        roots.push(PathBuf::from(
            r"C:\Program Files (x86)\GOG Galaxy\Games",
        ));
    }
    roots
}

#[derive(Debug, Deserialize)]
struct GogInfoFile {
    #[serde(rename = "gameId")]
    game_id: String,
    name: String,
}

/// Scans up to `max_depth` directory levels under each root for
/// `goggame-*.info` files.
pub fn scan(gog_roots: &[PathBuf]) -> Vec<Game> {
    let mut games = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for root in gog_roots {
        if !root.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(root)
            .max_depth(3)
            .into_iter()
            .flatten()
        {
            let path = entry.path();
            let is_info_file = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("goggame-") && n.ends_with(".info"))
                .unwrap_or(false);
            if !is_info_file {
                continue;
            }
            if let Some(game) = parse_info_file(path) {
                if seen_ids.insert(game.id.clone()) {
                    games.push(game);
                }
            }
        }
    }
    games
}

fn parse_info_file(path: &Path) -> Option<Game> {
    let contents = std::fs::read_to_string(path).ok()?;
    let info: GogInfoFile = serde_json::from_str(&contents).ok()?;
    let install_path = path.parent().map(|p| p.to_string_lossy().to_string());

    let mut game = Game::new(Platform::Gog, info.game_id, info.name);
    if let Some(install_path) = install_path {
        game = game.with_install_path(install_path);
    }
    Some(game)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn scans_installed_gog_game() {
        let root = tempdir().unwrap();
        let gog_root = root.path().join("GOG Games");
        let game_dir = gog_root.join("The Witcher 3");
        write(
            &game_dir.join("goggame-1495134320.info"),
            r#"{"language":"en-US","name":"The Witcher 3: Wild Hunt","gameId":"1495134320"}"#,
        );

        let games = scan(&[gog_root]);
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].id, "gog:1495134320");
        assert_eq!(games[0].name, "The Witcher 3: Wild Hunt");
        assert_eq!(
            games[0].install_path.as_deref().unwrap(),
            game_dir.to_string_lossy()
        );
    }

    #[test]
    fn ignores_unrelated_files_and_bad_json() {
        let root = tempdir().unwrap();
        let gog_root = root.path().join("GOG Games");
        write(&gog_root.join("SomeGame/readme.txt"), "hello");
        write(&gog_root.join("SomeGame/goggame-1.info"), "not json");
        let games = scan(&[gog_root]);
        assert!(games.is_empty());
    }

    #[test]
    fn dedupes_multiple_info_files_with_same_id() {
        let root = tempdir().unwrap();
        let gog_root = root.path().join("GOG Games");
        write(
            &gog_root.join("Game/goggame-42.info"),
            r#"{"name":"Game","gameId":"42"}"#,
        );
        write(
            // GOG sometimes ships a second locale-agnostic copy
            &gog_root.join("Game/goggame-42-fr.info"),
            r#"{"name":"Game","gameId":"42"}"#,
        );
        let games = scan(&[gog_root]);
        assert_eq!(games.len(), 1);
    }
}
