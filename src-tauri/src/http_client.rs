//! Shared "fetch JSON over HTTP with some headers" abstraction, used by both
//! the itch.io and Nexus Mods clients. Kept generic over headers (rather than
//! baking in a single auth scheme) since itch.io uses a Bearer token while
//! Nexus Mods uses a custom `apikey` header.
//!
//! Real network access is isolated behind this trait so unit tests can supply
//! canned responses instead of hitting the real network.

pub trait HttpClient {
    fn get_json(&self, url: &str, headers: &[(&str, &str)]) -> Result<HttpResponse, String>;
}

pub struct HttpResponse {
    pub status: u16,
    pub body: serde_json::Value,
}

pub struct ReqwestClient {
    client: reqwest::blocking::Client,
}

impl Default for ReqwestClient {
    fn default() -> Self {
        ReqwestClient {
            client: reqwest::blocking::Client::builder()
                .user_agent("Zegra/0.1 (+https://github.com/Zexolver/Zegra)")
                .build()
                .expect("failed to build HTTP client"),
        }
    }
}

impl HttpClient for ReqwestClient {
    fn get_json(&self, url: &str, headers: &[(&str, &str)]) -> Result<HttpResponse, String> {
        let mut req = self.client.get(url);
        for (name, value) in headers {
            req = req.header(*name, *value);
        }
        let resp = req.send().map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let body: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
        Ok(HttpResponse { status, body })
    }
}
