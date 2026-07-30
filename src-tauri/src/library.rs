//! Aggregates every platform's scan into a single combined library.

use std::path::PathBuf;

use crate::models::{Game, LibraryScanResult, Platform, PlatformScanResult, PlatformScanStatus};
use crate::platforms::{epic, gamejolt, gog, itchio, steam};
use crate::settings::Settings;

/// Runs every platform scan and merges the results. Local scanners
/// (Steam/GOG/Epic) never fail outright — an unreadable/missing path is just
/// "found nothing there" — but itch.io can fail (missing/invalid API key,
/// network error) so its result is surfaced distinctly.
pub fn scan_all(settings: &Settings, http_client: &dyn itchio::HttpClient) -> LibraryScanResult {
    let mut platform_results = Vec::new();
    let mut all_games: Vec<Game> = Vec::new();

    // Steam
    let mut steam_roots = steam::default_steam_roots();
    steam_roots.extend(settings.extra_steam_roots.iter().map(PathBuf::from));
    let steam_games = steam::scan(&steam_roots);
    all_games.extend(steam_games.clone());
    platform_results.push(PlatformScanResult {
        platform: Platform::Steam,
        status: PlatformScanStatus::Ok,
        games: steam_games,
    });

    // GOG
    let mut gog_roots = gog::default_gog_roots();
    gog_roots.extend(settings.extra_gog_roots.iter().map(PathBuf::from));
    let gog_games = gog::scan(&gog_roots);
    all_games.extend(gog_games.clone());
    platform_results.push(PlatformScanResult {
        platform: Platform::Gog,
        status: PlatformScanStatus::Ok,
        games: gog_games,
    });

    // Epic (via Legendary/Heroic)
    let mut legendary_paths = epic::default_legendary_paths();
    legendary_paths.extend(settings.extra_legendary_paths.iter().map(PathBuf::from));
    let epic_games = epic::scan(&legendary_paths);
    all_games.extend(epic_games.clone());
    platform_results.push(PlatformScanResult {
        platform: Platform::Epic,
        status: PlatformScanStatus::Ok,
        games: epic_games,
    });

    // itch.io
    match &settings.itchio_api_key {
        None => platform_results.push(PlatformScanResult {
            platform: Platform::Itchio,
            status: PlatformScanStatus::Unsupported {
                reason: "no itch.io API key configured in Settings".to_string(),
            },
            games: vec![],
        }),
        Some(key) => match itchio::fetch_owned_games(http_client, key) {
            Ok(games) => {
                all_games.extend(games.clone());
                platform_results.push(PlatformScanResult {
                    platform: Platform::Itchio,
                    status: PlatformScanStatus::Ok,
                    games,
                });
            }
            Err(e) => platform_results.push(PlatformScanResult {
                platform: Platform::Itchio,
                status: PlatformScanStatus::Error {
                    message: e.to_string(),
                },
                games: vec![],
            }),
        },
    }

    // GameJolt: documented as unsupported (see platforms::gamejolt).
    platform_results.push(PlatformScanResult {
        platform: Platform::Gamejolt,
        status: PlatformScanStatus::Unsupported {
            reason: gamejolt::UNSUPPORTED_REASON.to_string(),
        },
        games: vec![],
    });

    LibraryScanResult {
        games: all_games,
        platform_results,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platforms::itchio::HttpClient;

    struct NoopHttpClient;
    impl HttpClient for NoopHttpClient {
        fn get_json(&self, _url: &str, _bearer_token: &str) -> Result<serde_json::Value, String> {
            Err("network disabled in test".to_string())
        }
    }

    #[test]
    fn scan_all_without_itchio_key_marks_it_unsupported_and_gamejolt_unsupported() {
        let settings = Settings::default();
        let result = scan_all(&settings, &NoopHttpClient);

        let itchio_result = result
            .platform_results
            .iter()
            .find(|r| r.platform == Platform::Itchio)
            .unwrap();
        assert!(matches!(
            itchio_result.status,
            PlatformScanStatus::Unsupported { .. }
        ));

        let gamejolt_result = result
            .platform_results
            .iter()
            .find(|r| r.platform == Platform::Gamejolt)
            .unwrap();
        assert!(matches!(
            gamejolt_result.status,
            PlatformScanStatus::Unsupported { .. }
        ));

        // Steam/GOG/Epic should still report Ok even though nothing exists on this machine.
        for platform in [Platform::Steam, Platform::Gog, Platform::Epic] {
            let r = result
                .platform_results
                .iter()
                .find(|r| r.platform == platform)
                .unwrap();
            assert_eq!(r.status, PlatformScanStatus::Ok);
        }
    }

    #[test]
    fn scan_all_surfaces_itchio_errors_when_key_is_present_but_request_fails() {
        let mut settings = Settings::default();
        settings.itchio_api_key = Some("bad-key".to_string());
        let result = scan_all(&settings, &NoopHttpClient);
        let itchio_result = result
            .platform_results
            .iter()
            .find(|r| r.platform == Platform::Itchio)
            .unwrap();
        assert!(matches!(
            itchio_result.status,
            PlatformScanStatus::Error { .. }
        ));
    }
}
