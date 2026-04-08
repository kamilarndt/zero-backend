//! HTTP connection pool manager
//!
//! Provides a shared reqwest::Client with connection pooling
//! to reuse TCP connections across all API requests.

use once_cell::sync::Lazy;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;

/// Shared HTTP client with connection pooling
///
/// Configuration:
/// - pool_idle_timeout: 90s (keep connections open for reuse)
/// - pool_max_idle_per_host: 10 (maintain up to 10 idle connections)
/// - timeout: 10s (prevent hanging requests)
///
/// # Example
/// ```rust
/// use crate::state::http::get_http_client;
///
/// let client = get_http_client();
/// let response = client.get("http://example.com").send().await?;
/// ```
pub fn get_http_client() -> &'static Arc<Client> {
    static CLIENT: Lazy<Arc<Client>> = Lazy::new(|| {
        Arc::new(
            Client::builder()
                .pool_idle_timeout(Duration::from_secs(90))
                .pool_max_idle_per_host(10)
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
        )
    });

    &CLIENT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_http_client_returns_client() {
        let client = get_http_client();
        // Just verify it returns without panic and is not null
        assert!(std::sync::Arc::strong_count(client) >= 1);
    }

    #[test]
    fn test_get_http_client_is_singleton() {
        let client1 = get_http_client();
        let client2 = get_http_client();
        // Same address = same singleton
        assert!(std::ptr::eq(client1.as_ref(), client2.as_ref()));
    }
}
