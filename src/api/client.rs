use super::types::{ContentItem, EnvVar};
use crate::error::AppError;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct ConnectClient {
    inner: Arc<ClientInner>,
}

struct ClientInner {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl ConnectClient {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(ClientInner {
                client: {
                    // Build a client that trusts corporate CA bundle if SSL_CERT_FILE or SSL_CERT_DIR are set
                    let mut builder = reqwest::Client::builder();

                    // Load single PEM file from SSL_CERT_FILE
                    if let Ok(path) = std::env::var("SSL_CERT_FILE") {
                        if let Ok(pem) = std::fs::read(&path) {
                            if let Ok(cert) = reqwest::Certificate::from_pem(&pem) {
                                builder = builder.add_root_certificate(cert);
                            }
                        }
                    }

                    // Load all *.pem files from SSL_CERT_DIR
                    if let Ok(dir) = std::env::var("SSL_CERT_DIR") {
                        if let Ok(entries) = std::fs::read_dir(&dir) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                if path.extension().and_then(|s| s.to_str()) == Some("pem") {
                                    if let Ok(pem) = std::fs::read(&path) {
                                        if let Ok(cert) = reqwest::Certificate::from_pem(&pem) {
                                            builder = builder.add_root_certificate(cert);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    builder.build().expect("failed to build reqwest client")
                },
                base_url: base_url.into().trim_end_matches('/').to_string(),
                api_key: api_key.into(),
            }),
        }
    }

    fn auth_header(&self) -> String {
        format!("Key {}", self.inner.api_key)
    }

    pub async fn list_content(&self) -> Result<Vec<ContentItem>, AppError> {
        let url = format!("{}/__api__/v1/content", self.inner.base_url);
        let resp = self
            .inner
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Api(format!(
                "HTTP {} — {}",
                status,
                &body[..body.len().min(200)]
            )));
        }

        let text = resp.text().await?;

        // Posit Connect returns a plain array
        if let Ok(items) = serde_json::from_str::<Vec<ContentItem>>(&text) {
            return Ok(items);
        }
        // Fallback: results wrapper
        #[derive(Deserialize)]
        struct Wrapper {
            results: Vec<ContentItem>,
        }
        if let Ok(w) = serde_json::from_str::<Wrapper>(&text) {
            return Ok(w.results);
        }

        Err(AppError::Api(format!(
            "Unexpected response format: {}",
            &text[..text.len().min(300)]
        )))
    }

    pub async fn get_env_vars(&self, guid: &str) -> Result<Vec<EnvVar>, AppError> {
        let url = format!(
            "{}/__api__/v1/content/{}/environment",
            self.inner.base_url, guid
        );
        let resp = self
            .inner
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(AppError::Api(format!(
                "HTTP {} fetching env vars for {}",
                resp.status(),
                guid
            )));
        }

        let text = resp.text().await?;

        // Connect returns a plain string array ["VAR_NAME", ...] (names only).
        // Older/alternate versions may return [{name, value}] or {"KEY": "VALUE"}.
        if let Ok(names) = serde_json::from_str::<Vec<String>>(&text) {
            return Ok(names
                .into_iter()
                .map(|name| EnvVar { name, value: None })
                .collect());
        }
        if let Ok(vars) = serde_json::from_str::<Vec<EnvVar>>(&text) {
            return Ok(vars);
        }
        if let Ok(map) = serde_json::from_str::<HashMap<String, Option<String>>>(&text) {
            return Ok(map
                .into_iter()
                .map(|(name, value)| EnvVar { name, value })
                .collect());
        }

        Err(AppError::Api(format!(
            "Unexpected env var response for {}: {}",
            guid,
            &text[..text.len().min(200)]
        )))
    }

    /// PATCH replaces the full env var set — we always send the safe-merged list.
    pub async fn set_env_vars(&self, guid: &str, vars: &[EnvVar]) -> Result<(), AppError> {
        let url = format!(
            "{}/__api__/v1/content/{}/environment",
            self.inner.base_url, guid
        );
        let resp = self
            .inner
            .client
            .patch(&url)
            .header("Authorization", self.auth_header())
            .json(vars)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Api(format!("PATCH failed: {}", body)));
        }

        Ok(())
    }
}
