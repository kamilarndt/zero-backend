//! SiYuan API HTTP client with authentication support.

use super::types::SiyuanResponse;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;

/// Default SiYuan API endpoint.
const DEFAULT_BASE_URL: &str = "http://localhost:6806";

/// HTTP client for communicating with the SiYuan REST API.
#[derive(Debug, Clone)]
pub struct SiyuanClient {
    /// HTTP client with connection pooling.
    client: Client,
    /// API token for authentication (optional for local without auth).
    api_token: String,
    /// Base URL of the SiYuan instance.
    base_url: String,
}

impl SiyuanClient {
    /// Create a new SiYuan client with explicit API token.
    pub fn new(api_token: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_token,
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// Create a new SiYuan client from environment variables.
    ///
    /// Environment variables:
    /// - `SIYUAN_API_TOKEN`: API token for authentication (optional)
    /// - `SIYUAN_BASE_URL`: Base URL (defaults to http://localhost:6806)
    pub fn from_env() -> Result<Self> {
        let api_token = std::env::var("SIYUAN_API_TOKEN").unwrap_or_default();
        let base_url = std::env::var("SIYUAN_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            api_token,
            base_url,
        })
    }

    /// Get the API token being used.
    pub fn api_token(&self) -> &str {
        &self.api_token
    }

    /// Get the base URL being used.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Send a POST request to the SiYuan API.
    ///
    /// # Arguments
    /// * `endpoint` - API endpoint path (e.g., "/api/query/sql")
    /// * `payload` - Request body to serialize as JSON
    ///
    /// # Returns
    /// Deserialized response or an error.
    async fn post<T: Serialize, R: for<'de> serde::Deserialize<'de>>(
        &self,
        endpoint: &str,
        payload: &T,
    ) -> Result<R> {
        let url = format!("{}{}", self.base_url, endpoint);

        let mut request = self.client.post(&url).json(payload);

        // Add Authorization header if token is present
        if !self.api_token.is_empty() {
            request = request.header("Authorization", format!("Token {}", self.api_token));
        }

        let response = request
            .send()
            .await
            .context("Failed to send request to SiYuan API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("SiYuan API returned error: {} - {}", status, error_text);
        }

        let result = response
            .json()
            .await
            .context("Failed to parse SiYuan API response")?;

        Ok(result)
    }

    /// Send a POST request and extract the data field from SiYuanResponse wrapper.
    async fn post_data<T: Serialize, R: for<'de> serde::Deserialize<'de>>(
        &self,
        endpoint: &str,
        payload: &T,
    ) -> Result<R> {
        let response: SiyuanResponse<R> = self.post(endpoint, payload).await?;
        response.into_result()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creates_with_token() {
        let client = SiyuanClient::new("test-token".into());
        assert_eq!(client.api_token(), "test-token");
        assert_eq!(client.base_url(), "http://localhost:6806");
    }

    #[test]
    fn test_client_from_env_with_custom_url() {
        std::env::set_var("SIYUAN_API_TOKEN", "env-token");
        std::env::set_var("SIYUAN_BASE_URL", "http://localhost:6807");

        let client = SiyuanClient::from_env().unwrap();
        assert_eq!(client.api_token(), "env-token");
        assert_eq!(client.base_url(), "http://localhost:6807");

        // Clean up
        std::env::remove_var("SIYUAN_API_TOKEN");
        std::env::remove_var("SIYUAN_BASE_URL");
    }
}