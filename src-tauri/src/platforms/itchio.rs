//! itch.io library integration via itch.io's real, documented server-side API
//! (<https://itch.io/docs/api/serverside>). This is a personal API key the
//! user generates themselves at itch.io/user/settings/api-keys — no store
//! partnership is required, just the same kind of personal access token
//! GitHub or GitLab issue.
//!
//! `GET https://api.itch.io/profile/owned-keys` returns every "download key"
//! for games the authenticated user owns, paginated.

use serde::Deserialize;

use crate::http_client::HttpClient;
use crate::models::{Game, Platform};

const API_BASE: &str = "https://api.itch.io";
const PER_PAGE_HINT: usize = 5; // itch.io's default/observed page size for this endpoint

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ItchIoError {
    #[error("no itch.io API key configured")]
    MissingApiKey,
    #[error("request to {0} failed: {1}")]
    Request(String, String),
    #[error("itch.io returned an error: {0}")]
    Api(String),
    #[error("could not parse itch.io response: {0}")]
    Parse(String),
}

#[derive(Debug, Deserialize)]
struct OwnedKeysResponse {
    #[serde(default)]
    owned_keys: Vec<OwnedKey>,
}

#[derive(Debug, Deserialize)]
struct OwnedKey {
    game: ItchGame,
}

#[derive(Debug, Deserialize)]
struct ItchGame {
    id: i64,
    title: String,
    #[serde(default)]
    cover_url: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

fn extract_error_message(body: &serde_json::Value) -> String {
    body.get("errors")
        .and_then(|e| e.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .unwrap_or("unknown error")
        .to_string()
}

/// Fetches every game the authenticated user owns, following pagination until
/// a page comes back with fewer than `PER_PAGE_HINT` entries.
pub fn fetch_owned_games(
    client: &dyn HttpClient,
    api_key: &str,
) -> Result<Vec<Game>, ItchIoError> {
    if api_key.trim().is_empty() {
        return Err(ItchIoError::MissingApiKey);
    }

    let mut games = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
    let mut page = 1;
    let auth_header = format!("Bearer {api_key}");

    loop {
        let url = format!("{API_BASE}/profile/owned-keys?page={page}");
        let resp = client
            .get_json(&url, &[("Authorization", &auth_header)])
            .map_err(|e| ItchIoError::Request(url.clone(), e))?;

        if !(200..300).contains(&resp.status) {
            return Err(ItchIoError::Api(extract_error_message(&resp.body)));
        }

        let parsed: OwnedKeysResponse =
            serde_json::from_value(resp.body).map_err(|e| ItchIoError::Parse(e.to_string()))?;

        let count = parsed.owned_keys.len();
        for key in parsed.owned_keys {
            let mut game = Game::new(Platform::Itchio, key.game.id.to_string(), key.game.title);
            game.installed = false; // itch.io library entries are "owned", not necessarily downloaded
            if let Some(cover) = key.game.cover_url {
                game = game.with_cover_url(cover);
            }
            if let Some(url) = key.game.url {
                // Store the store URL as install_path placeholder so the UI can link out.
                game.install_path = Some(url);
            }
            if seen_ids.insert(game.id.clone()) {
                games.push(game);
            }
        }

        if count < PER_PAGE_HINT {
            break;
        }
        page += 1;
        if page > 200 {
            // Safety valve against runaway pagination from an unexpected API shape.
            break;
        }
    }

    Ok(games)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_client::HttpResponse;
    use std::cell::RefCell;
    use std::collections::HashMap;

    struct MockClient {
        pages: RefCell<HashMap<String, serde_json::Value>>,
        calls: RefCell<Vec<(String, String)>>,
    }

    impl MockClient {
        fn new(pages: Vec<(&str, serde_json::Value)>) -> Self {
            MockClient {
                pages: RefCell::new(
                    pages
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v))
                        .collect(),
                ),
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl HttpClient for MockClient {
        fn get_json(&self, url: &str, headers: &[(&str, &str)]) -> Result<HttpResponse, String> {
            let auth = headers
                .iter()
                .find(|(name, _)| *name == "Authorization")
                .map(|(_, v)| v.to_string())
                .unwrap_or_default();
            self.calls.borrow_mut().push((url.to_string(), auth));
            self.pages
                .borrow()
                .get(url)
                .cloned()
                .map(|body| HttpResponse { status: 200, body })
                .ok_or_else(|| format!("unexpected url: {url}"))
        }
    }

    fn owned_key_json(id: i64, title: &str) -> serde_json::Value {
        serde_json::json!({
            "game": { "id": id, "title": title, "cover_url": format!("https://example.test/{id}.png"), "url": format!("https://x.itch.io/{title}") }
        })
    }

    #[test]
    fn missing_api_key_short_circuits() {
        let client = MockClient::new(vec![]);
        let err = fetch_owned_games(&client, "").unwrap_err();
        assert_eq!(err, ItchIoError::MissingApiKey);
    }

    #[test]
    fn parses_single_page_of_owned_games() {
        let client = MockClient::new(vec![(
            "https://api.itch.io/profile/owned-keys?page=1",
            serde_json::json!({ "owned_keys": [owned_key_json(1, "Celeste Classic"), owned_key_json(2, "Doukutsu")] }),
        )]);
        let games = fetch_owned_games(&client, "test-key").unwrap();
        assert_eq!(games.len(), 2);
        assert_eq!(games[0].id, "itchio:1");
        assert_eq!(games[0].name, "Celeste Classic");
        assert!(!games[0].installed);
        assert_eq!(client.calls.borrow()[0].1, "Bearer test-key");
    }

    #[test]
    fn follows_pagination_until_short_page() {
        let full_page: Vec<_> = (0..PER_PAGE_HINT)
            .map(|i| owned_key_json(i as i64, &format!("Game {i}")))
            .collect();
        let client = MockClient::new(vec![
            (
                "https://api.itch.io/profile/owned-keys?page=1",
                serde_json::json!({ "owned_keys": full_page }),
            ),
            (
                "https://api.itch.io/profile/owned-keys?page=2",
                serde_json::json!({ "owned_keys": [owned_key_json(999, "Last Game")] }),
            ),
        ]);
        let games = fetch_owned_games(&client, "test-key").unwrap();
        assert_eq!(games.len(), PER_PAGE_HINT + 1);
        assert_eq!(client.calls.borrow().len(), 2);
    }

    #[test]
    fn propagates_request_errors() {
        struct FailingClient;
        impl HttpClient for FailingClient {
            fn get_json(&self, _url: &str, _headers: &[(&str, &str)]) -> Result<HttpResponse, String> {
                Err("connection refused".to_string())
            }
        }
        let err = fetch_owned_games(&FailingClient, "key").unwrap_err();
        matches!(err, ItchIoError::Request(_, _));
    }

    #[test]
    fn non_success_status_becomes_api_error() {
        struct UnauthorizedClient;
        impl HttpClient for UnauthorizedClient {
            fn get_json(&self, _url: &str, _headers: &[(&str, &str)]) -> Result<HttpResponse, String> {
                Ok(HttpResponse {
                    status: 401,
                    body: serde_json::json!({ "errors": ["invalid key"] }),
                })
            }
        }
        let err = fetch_owned_games(&UnauthorizedClient, "bad-key").unwrap_err();
        assert_eq!(err, ItchIoError::Api("invalid key".to_string()));
    }
}
