//! Local Steam library detection.
//!
//! Steam does not offer a public "list my installed games" API, but its own
//! client relies entirely on plain-text files that any tool can read:
//! `steamapps/libraryfolders.vdf` lists every library folder the user has
//! configured, and each library folder contains one `appmanifest_<id>.acf`
//! file per installed game. This is the same approach used by community
//! launchers such as Lutris and Playnite.

use std::path::{Path, PathBuf};

use crate::models::{Game, Platform};
use crate::vdf;

/// Best-effort guesses for where Steam is installed, per OS.
pub fn default_steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".steam/steam"));
        roots.push(home.join(".steam/root"));
        roots.push(home.join(".local/share/Steam"));
        roots.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
        #[cfg(target_os = "macos")]
        roots.push(home.join("Library/Application Support/Steam"));
    }
    #[cfg(target_os = "windows")]
    {
        roots.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
        roots.push(PathBuf::from(r"C:\Program Files\Steam"));
    }
    roots
}

/// Reads `<steam_root>/steamapps/libraryfolders.vdf` and returns every library
/// path it references, including `steam_root` itself (Steam always treats its
/// own install directory as the first library).
fn library_folders(steam_root: &Path) -> Vec<PathBuf> {
    let mut libs = vec![steam_root.to_path_buf()];
    let vdf_path = steam_root.join("steamapps").join("libraryfolders.vdf");
    let Ok(contents) = std::fs::read_to_string(&vdf_path) else {
        return libs;
    };
    let Ok(root) = vdf::parse(&contents) else {
        return libs;
    };
    let Some(folders) = root.get("libraryfolders") else {
        return libs;
    };
    let Some(entries) = folders.as_obj() else {
        return libs;
    };
    for (_, entry) in entries {
        if let Some(path_str) = entry.get("path").and_then(|v| v.as_str()) {
            let path = PathBuf::from(path_str);
            if !libs.contains(&path) {
                libs.push(path);
            }
        }
    }
    libs
}

fn parse_appmanifest(path: &Path) -> Option<Game> {
    let contents = std::fs::read_to_string(path).ok()?;
    let root = vdf::parse(&contents).ok()?;
    let app_state = root.get("AppState")?;
    let fields = app_state.flat_strings();
    let appid = fields.get("appid")?.clone();
    let name = fields.get("name")?.clone();
    let installdir = fields.get("installdir").cloned();

    let library_root = path.parent()?.parent()?; // steamapps/ -> library root
    let install_path = installdir
        .map(|dir| library_root.join("steamapps").join("common").join(dir))
        .map(|p| p.to_string_lossy().to_string());

    let mut game = Game::new(Platform::Steam, appid.clone(), name);
    if let Some(install_path) = install_path {
        game = game.with_install_path(install_path);
    }
    game = game.with_cover_url(format!(
        "https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/library_600x900.jpg"
    ));
    Some(game)
}

/// Scans the given candidate Steam install roots for installed games.
/// Non-existent roots are silently skipped (most candidates won't exist on
/// any given machine).
pub fn scan(steam_roots: &[PathBuf]) -> Vec<Game> {
    let mut games = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for root in steam_roots {
        if !root.exists() {
            continue;
        }
        for lib in library_folders(root) {
            let steamapps = lib.join("steamapps");
            let Ok(entries) = std::fs::read_dir(&steamapps) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let is_manifest = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("appmanifest_") && n.ends_with(".acf"))
                    .unwrap_or(false);
                if !is_manifest {
                    continue;
                }
                if let Some(game) = parse_appmanifest(&path) {
                    if seen_ids.insert(game.id.clone()) {
                        games.push(game);
                    }
                }
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

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn scans_single_library_with_two_games() {
        let root = tempdir().unwrap();
        let steam_root = root.path().join("steam");

        write(
            &steam_root.join("steamapps/appmanifest_440.acf"),
            r#"
            "AppState"
            {
                "appid"         "440"
                "name"          "Team Fortress 2"
                "installdir"    "Team Fortress 2"
            }
            "#,
        );
        write(
            &steam_root.join("steamapps/appmanifest_730.acf"),
            r#"
            "AppState"
            {
                "appid"         "730"
                "name"          "Counter-Strike 2"
                "installdir"    "Counter-Strike Global Offensive"
            }
            "#,
        );

        let games = scan(&[steam_root.clone()]);
        assert_eq!(games.len(), 2);
        let tf2 = games.iter().find(|g| g.id == "steam:440").unwrap();
        assert_eq!(tf2.name, "Team Fortress 2");
        assert_eq!(
            tf2.install_path.as_deref().unwrap(),
            steam_root
                .join("steamapps/common/Team Fortress 2")
                .to_string_lossy()
        );
    }

    #[test]
    fn follows_additional_library_folders() {
        let root = tempdir().unwrap();
        let steam_root = root.path().join("steam");
        let extra_lib = root.path().join("extra_lib");

        write(
            &steam_root.join("steamapps/libraryfolders.vdf"),
            &format!(
                r#"
                "libraryfolders"
                {{
                    "0"
                    {{
                        "path" "{}"
                        "apps"
                        {{
                            "570" "1"
                        }}
                    }}
                }}
                "#,
                extra_lib.to_string_lossy().replace('\\', "\\\\")
            ),
        );
        write(
            &extra_lib.join("steamapps/appmanifest_570.acf"),
            r#"
            "AppState"
            {
                "appid"         "570"
                "name"          "Dota 2"
                "installdir"    "dota 2 beta"
            }
            "#,
        );

        let games = scan(&[steam_root]);
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].name, "Dota 2");
    }

    #[test]
    fn nonexistent_root_yields_no_games_without_error() {
        let games = scan(&[PathBuf::from("/does/not/exist")]);
        assert!(games.is_empty());
    }

    #[test]
    fn skips_malformed_manifest() {
        let root = tempdir().unwrap();
        let steam_root = root.path().join("steam");
        write(&steam_root.join("steamapps/appmanifest_1.acf"), "not valid vdf {{{");
        let games = scan(&[steam_root]);
        assert!(games.is_empty());
    }
}
