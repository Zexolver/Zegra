use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Steam,
    Gog,
    Epic,
    Itchio,
    Gamejolt,
}

impl Platform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Steam => "steam",
            Platform::Gog => "gog",
            Platform::Epic => "epic",
            Platform::Itchio => "itchio",
            Platform::Gamejolt => "gamejolt",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Game {
    /// Namespaced unique id, e.g. "steam:440"
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub install_path: Option<String>,
    pub cover_url: Option<String>,
    pub installed: bool,
}

impl Game {
    pub fn new(platform: Platform, platform_id: impl Into<String>, name: impl Into<String>) -> Self {
        let platform_id = platform_id.into();
        Game {
            id: format!("{}:{}", platform.as_str(), platform_id),
            name: name.into(),
            platform,
            install_path: None,
            cover_url: None,
            installed: true,
        }
    }

    pub fn with_install_path(mut self, path: impl Into<String>) -> Self {
        self.install_path = Some(path.into());
        self
    }

    pub fn with_cover_url(mut self, url: impl Into<String>) -> Self {
        self.cover_url = Some(url.into());
        self
    }
}

/// Result of attempting to scan a single platform's local/remote library.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum PlatformScanStatus {
    /// Scan ran successfully (may still find zero games).
    Ok,
    /// This platform has no viable integration path (documented reason), so it was skipped.
    Unsupported { reason: String },
    /// Scan was attempted but failed (bad config, unreachable, parse error, etc).
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlatformScanResult {
    pub platform: Platform,
    pub status: PlatformScanStatus,
    pub games: Vec<Game>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LibraryScanResult {
    pub games: Vec<Game>,
    pub platform_results: Vec<PlatformScanResult>,
}
