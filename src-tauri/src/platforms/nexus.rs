//! Resolves `nxm://` download links into real file URLs via the real Nexus
//! Mods API.
//!
//! Nexus Mods' "Mod Manager Download" buttons produce links shaped like
//! `nxm://{game_domain}/mods/{mod_id}/files/{file_id}?key={key}&expires={expires}&user_id={user_id}`
//! — the same link format Vortex and Mod Organizer 2 register themselves as
//! the OS handler for. Given a personal Nexus API key (from
//! nexusmods.com/users/myaccount?tab=api), Zegra turns one of these into a
//! real download URL by calling the documented Nexus Mods API:
//! `GET /v1/games/{domain}/mods/{id}/files/{file}/download_link.json`.
//! For non-Premium accounts this endpoint requires the `key`/`expires` query
//! parameters that came from the nxm link itself (a signed, short-lived token
//! Nexus embeds specifically to authorize this one download); Premium
//! accounts work without them.

use serde::Deserialize;

use crate::http_client::HttpClient;

const API_BASE: &str = "https://api.nexusmods.com";

#[derive(Debug, Clone, PartialEq)]
pub struct NxmLink {
    pub game_domain: String,
    pub mod_id: String,
    pub file_id: String,
    pub key: Option<String>,
    pub expires: Option<String>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum NexusError {
    #[error("no Nexus Mods API key configured")]
    MissingApiKey,
    #[error("not a valid nxm:// link: {0}")]
    InvalidLink(String),
    #[error("request to {0} failed: {1}")]
    Request(String, String),
    #[error("Nexus Mods returned an error: {0}")]
    Api(String),
    #[error("could not parse Nexus Mods response: {0}")]
    Parse(String),
    #[error("Nexus Mods returned no download mirrors for this file")]
    NoMirrors,
}

/// Parses an `nxm://` URL as produced by Nexus Mods' "Mod Manager Download"
/// buttons into its component parts.
pub fn parse_nxm_url(raw: &str) -> Result<NxmLink, NexusError> {
    let url = url::Url::parse(raw).map_err(|e| NexusError::InvalidLink(e.to_string()))?;
    if url.scheme() != "nxm" {
        return Err(NexusError::InvalidLink(format!(
            "expected an nxm:// link, got {}://",
            url.scheme()
        )));
    }
    let game_domain = url
        .host_str()
        .ok_or_else(|| NexusError::InvalidLink("missing game domain".to_string()))?
        .to_string();

    let segments: Vec<&str> = url.path_segments().map(|s| s.collect()).unwrap_or_default();
    let (mod_id, file_id) = match segments.as_slice() {
        ["mods", mod_id, "files", file_id] => (mod_id.to_string(), file_id.to_string()),
        _ => {
            return Err(NexusError::InvalidLink(format!(
                "expected /mods/<id>/files/<id>, got {}",
                url.path()
            )))
        }
    };

    let mut key = None;
    let mut expires = None;
    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "key" => key = Some(v.to_string()),
            "expires" => expires = Some(v.to_string()),
            _ => {}
        }
    }

    Ok(NxmLink {
        game_domain,
        mod_id,
        file_id,
        key,
        expires,
    })
}

#[derive(Debug, Deserialize)]
struct DownloadMirror {
    #[serde(rename = "URI")]
    uri: String,
}

fn extract_error_message(body: &serde_json::Value) -> String {
    body.get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown error")
        .to_string()
}

fn download_link_url(link: &NxmLink) -> String {
    let mut url = url::Url::parse(API_BASE).expect("API_BASE is a valid URL");
    url.set_path(&format!(
        "/v1/games/{}/mods/{}/files/{}/download_link.json",
        link.game_domain, link.mod_id, link.file_id
    ));
    // `query_pairs_mut()` sets the query to `Some("")` the moment it's called,
    // even if nothing is ever appended, so only touch it when there's
    // something to add (a Premium account's link has neither key nor expires).
    if link.key.is_some() || link.expires.is_some() {
        let mut query = url.query_pairs_mut();
        if let Some(key) = &link.key {
            query.append_pair("key", key);
        }
        if let Some(expires) = &link.expires {
            query.append_pair("expires", expires);
        }
    }
    url.to_string()
}

/// Resolves an `nxm://` link into a real, downloadable file URL by calling
/// the Nexus Mods API. Returns the first CDN mirror it's offered.
pub fn resolve_download_url(
    client: &dyn HttpClient,
    api_key: &str,
    link: &NxmLink,
) -> Result<String, NexusError> {
    if api_key.trim().is_empty() {
        return Err(NexusError::MissingApiKey);
    }

    let url = download_link_url(link);
    let resp = client
        .get_json(&url, &[("apikey", api_key)])
        .map_err(|e| NexusError::Request(url.clone(), e))?;

    if !(200..300).contains(&resp.status) {
        return Err(NexusError::Api(extract_error_message(&resp.body)));
    }

    let mirrors: Vec<DownloadMirror> =
        serde_json::from_value(resp.body).map_err(|e| NexusError::Parse(e.to_string()))?;
    mirrors.into_iter().next().map(|m| m.uri).ok_or(NexusError::NoMirrors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_client::HttpResponse;

    #[test]
    fn parses_a_real_shaped_nxm_link() {
        let link = parse_nxm_url(
            "nxm://skyrimspecialedition/mods/12345/files/67890?key=abc123&expires=1700000000&user_id=42",
        )
        .unwrap();
        assert_eq!(link.game_domain, "skyrimspecialedition");
        assert_eq!(link.mod_id, "12345");
        assert_eq!(link.file_id, "67890");
        assert_eq!(link.key.as_deref(), Some("abc123"));
        assert_eq!(link.expires.as_deref(), Some("1700000000"));
    }

    #[test]
    fn parses_a_premium_link_without_key_or_expires() {
        let link = parse_nxm_url("nxm://stardewvalley/mods/1/files/2").unwrap();
        assert_eq!(link.key, None);
        assert_eq!(link.expires, None);
    }

    #[test]
    fn rejects_non_nxm_scheme() {
        let err = parse_nxm_url("https://nexusmods.com/mods/1/files/2").unwrap_err();
        assert!(matches!(err, NexusError::InvalidLink(_)));
    }

    #[test]
    fn rejects_unexpected_path_shape() {
        let err = parse_nxm_url("nxm://skyrim/not-a-mod-path").unwrap_err();
        assert!(matches!(err, NexusError::InvalidLink(_)));
    }

    struct MockClient {
        expected_url_contains: Vec<&'static str>,
        response: Result<HttpResponse, String>,
    }

    impl HttpClient for MockClient {
        fn get_json(&self, url: &str, headers: &[(&str, &str)]) -> Result<HttpResponse, String> {
            for fragment in &self.expected_url_contains {
                assert!(url.contains(fragment), "expected {url} to contain {fragment}");
            }
            assert!(headers.iter().any(|(k, _)| *k == "apikey"));
            match &self.response {
                Ok(resp) => Ok(HttpResponse {
                    status: resp.status,
                    body: resp.body.clone(),
                }),
                Err(e) => Err(e.clone()),
            }
        }
    }

    fn sample_link() -> NxmLink {
        NxmLink {
            game_domain: "skyrimspecialedition".to_string(),
            mod_id: "12345".to_string(),
            file_id: "67890".to_string(),
            key: Some("abc123".to_string()),
            expires: Some("1700000000".to_string()),
        }
    }

    #[test]
    fn missing_api_key_short_circuits() {
        let client = MockClient {
            expected_url_contains: vec![],
            response: Err("should not be called".to_string()),
        };
        let err = resolve_download_url(&client, "", &sample_link()).unwrap_err();
        assert_eq!(err, NexusError::MissingApiKey);
    }

    #[test]
    fn builds_the_documented_download_link_url_with_key_and_expires() {
        let client = MockClient {
            expected_url_contains: vec![
                "/v1/games/skyrimspecialedition/mods/12345/files/67890/download_link.json",
                "key=abc123",
                "expires=1700000000",
            ],
            response: Ok(HttpResponse {
                status: 200,
                body: serde_json::json!([{ "name": "Nexus CDN", "short_name": "Nexus", "URI": "https://cdn.nexusmods.com/file.zip" }]),
            }),
        };
        let resolved = resolve_download_url(&client, "my-api-key", &sample_link()).unwrap();
        assert_eq!(resolved, "https://cdn.nexusmods.com/file.zip");
    }

    #[test]
    fn premium_link_omits_key_and_expires_from_the_request() {
        let link = NxmLink {
            game_domain: "stardewvalley".to_string(),
            mod_id: "1".to_string(),
            file_id: "2".to_string(),
            key: None,
            expires: None,
        };
        let client = MockClient {
            expected_url_contains: vec!["/v1/games/stardewvalley/mods/1/files/2/download_link.json"],
            response: Ok(HttpResponse {
                status: 200,
                body: serde_json::json!([{ "URI": "https://cdn.nexusmods.com/other.zip" }]),
            }),
        };
        let url_before = download_link_url(&link);
        assert!(!url_before.contains('?'));
        let resolved = resolve_download_url(&client, "my-api-key", &link).unwrap();
        assert_eq!(resolved, "https://cdn.nexusmods.com/other.zip");
    }

    #[test]
    fn non_success_status_becomes_api_error() {
        let client = MockClient {
            expected_url_contains: vec![],
            response: Ok(HttpResponse {
                status: 403,
                body: serde_json::json!({ "message": "not premium and key/expires invalid" }),
            }),
        };
        let err = resolve_download_url(&client, "my-api-key", &sample_link()).unwrap_err();
        assert_eq!(
            err,
            NexusError::Api("not premium and key/expires invalid".to_string())
        );
    }

    #[test]
    fn empty_mirror_list_is_a_distinct_error() {
        let client = MockClient {
            expected_url_contains: vec![],
            response: Ok(HttpResponse {
                status: 200,
                body: serde_json::json!([]),
            }),
        };
        let err = resolve_download_url(&client, "my-api-key", &sample_link()).unwrap_err();
        assert_eq!(err, NexusError::NoMirrors);
    }

    #[test]
    fn propagates_transport_errors() {
        let client = MockClient {
            expected_url_contains: vec![],
            response: Err("connection refused".to_string()),
        };
        let err = resolve_download_url(&client, "my-api-key", &sample_link()).unwrap_err();
        assert!(matches!(err, NexusError::Request(_, _)));
    }
}
