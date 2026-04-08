//! Reusable HTTP client with retry logic and timeout configuration for LLM providers.

use reqwest::Client;

/// Configuration for provider HTTP clients.
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// Service key for proxy configuration (e.g., "provider.openai")
    pub service_key: &'static str,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            service_key: "provider.unknown",
            timeout_secs: 120,
            connect_timeout_secs: 10,
        }
    }
}

/// Reusable HTTP client factory for LLM providers.
///
/// This struct wraps the common pattern of building reqwest clients with
/// runtime proxy support and timeout configuration.
///
/// # Example
/// ```rust
/// use crate::providers::common::http_client::ProviderHttpClient;
///
/// // Create client with default configuration
/// let client = ProviderHttpClient::new("provider.openai", 120, 10);
///
/// // Use client for requests
/// let response = client.client
///     .post("https://api.openai.com/v1/chat/completions")
///     .json(&request_body)
///     .send()
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct ProviderHttpClient {
    /// The underlying reqwest client
    pub client: Client,
    /// Configuration used to create this client
    pub config: HttpClientConfig,
}

impl ProviderHttpClient {
    /// Create a new HTTP client with the specified configuration.
    ///
    /// # Arguments
    /// * `service_key` - Service key for proxy configuration (e.g., "provider.openai")
    /// * `timeout_secs` - Request timeout in seconds (default: 120)
    /// * `connect_timeout_secs` - Connection timeout in seconds (default: 10)
    ///
    /// # Example
    /// ```rust
    /// let client = ProviderHttpClient::new("provider.anthropic", 120, 10);
    /// ```
    pub fn new(service_key: &'static str, timeout_secs: u64, connect_timeout_secs: u64) -> Self {
        let client = crate::config::build_runtime_proxy_client_with_timeouts(
            service_key,
            timeout_secs,
            connect_timeout_secs,
        );

        Self {
            client,
            config: HttpClientConfig {
                service_key,
                timeout_secs,
                connect_timeout_secs,
            },
        }
    }

    /// Create a client with default configuration (120s timeout, 10s connect timeout).
    ///
    /// # Example
    /// ```rust
    /// let client = ProviderHttpClient::with_defaults("provider.gemini");
    /// ```
    pub fn with_defaults(service_key: &'static str) -> Self {
        Self::new(service_key, 120, 10)
    }

    /// Create a client with long timeouts for slow providers (e.g., Ollama).
    ///
    /// # Example
    /// ```rust
    /// let client = ProviderHttpClient::with_long_timeout("provider.ollama");
    /// ```
    pub fn with_long_timeout(service_key: &'static str) -> Self {
        Self::new(service_key, 300, 10)
    }

    /// Get a reference to the underlying reqwest client.
    pub fn get(&self) -> &Client {
        &self.client
    }
}

/// Helper function to build HTTP clients with provider-specific configuration.
///
/// This is a convenience function that matches the common pattern used across providers.
///
/// # Arguments
/// * `provider_name` - Provider name (e.g., "openai", "anthropic", "gemini")
/// * `timeout_secs` - Optional timeout in seconds (defaults to 120)
/// * `connect_timeout_secs` - Optional connect timeout in seconds (defaults to 10)
///
/// # Example
/// ```rust
/// let client = build_provider_client("openai", None, None);
/// let client = build_provider_client("ollama", Some(300), Some(10));
/// ```
pub fn build_provider_client(
    provider_name: &str,
    timeout_secs: Option<u64>,
    connect_timeout_secs: Option<u64>,
) -> Client {
    let service_key = format!("provider.{}", provider_name);
    let timeout = timeout_secs.unwrap_or(120);
    let connect_timeout = connect_timeout_secs.unwrap_or(10);

    // Convert to static str for the config function
    let static_key: &'static str = Box::leak(service_key.into_boxed_str());

    crate::config::build_runtime_proxy_client_with_timeouts(static_key, timeout, connect_timeout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_config_default() {
        let config = HttpClientConfig::default();
        assert_eq!(config.service_key, "provider.unknown");
        assert_eq!(config.timeout_secs, 120);
        assert_eq!(config.connect_timeout_secs, 10);
    }

    #[test]
    fn test_provider_http_client_new() {
        let client = ProviderHttpClient::new("provider.test", 60, 5);
        assert_eq!(client.config.service_key, "provider.test");
        assert_eq!(client.config.timeout_secs, 60);
        assert_eq!(client.config.connect_timeout_secs, 5);
    }

    #[test]
    fn test_provider_http_client_with_defaults() {
        let client = ProviderHttpClient::with_defaults("provider.test2");
        assert_eq!(client.config.service_key, "provider.test2");
        assert_eq!(client.config.timeout_secs, 120);
        assert_eq!(client.config.connect_timeout_secs, 10);
    }

    #[test]
    fn test_provider_http_client_with_long_timeout() {
        let client = ProviderHttpClient::with_long_timeout("provider.test3");
        assert_eq!(client.config.service_key, "provider.test3");
        assert_eq!(client.config.timeout_secs, 300);
        assert_eq!(client.config.connect_timeout_secs, 10);
    }

    #[test]
    fn test_provider_http_client_get() {
        let client = ProviderHttpClient::with_defaults("provider.test4");
        let _client_ref = client.get();
        // Just ensure we can get a reference
        assert!(true);
    }
}
