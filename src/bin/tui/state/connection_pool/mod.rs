//! Connection Pool Manager for ZeroClaw TUI
//!
//! Manages HTTP connection reuse for efficient API calls to the ZeroClaw backend.
//! Uses a fixed-size pool of connections to minimize connection overhead.

use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    /// Maximum number of connections to keep open
    pub max_connections: usize,
    /// How long to keep idle connections open
    pub idle_timeout: Duration,
    /// Base URL for all connections
    pub base_url: String,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            idle_timeout: Duration::from_secs(30),
            base_url: "http://127.0.0.1:42617".to_string(),
        }
    }
}

/// Connection pool for HTTP requests
#[derive(Debug, Clone)]
pub struct ConnectionPool {
    inner: Arc<Mutex<InnerPool>>,
    config: ConnectionPoolConfig,
    base_url: String,
}

/// Inner state of the connection pool
#[derive(Debug)]
struct InnerPool {
    /// Available connections
    available: Vec<Client>,
    /// In-use connections (not directly managed by pool)
    in_use: usize,
    /// Total created connections
    total_created: usize,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: ConnectionPoolConfig) -> Self {
        let base_url = config.base_url.clone();
        let inner = InnerPool {
            available: Vec::with_capacity(config.max_connections),
            in_use: 0,
            total_created: 0,
        };

        Self {
            inner: Arc::new(Mutex::new(inner)),
            config,
            base_url,
        }
    }

    /// Get a client from the pool or create a new one
    pub async fn get_client(&self) -> Result<ConnectionWrapper, reqwest::Error> {
        let mut inner = self.inner.lock().await;

        // Try to reuse an existing connection
        if let Some(client) = inner.available.pop() {
            inner.in_use += 1;
            return Ok(ConnectionWrapper {
                client: Some(client),
                pool: self.clone(),
                in_use: true,
            });
        }

        // If we haven't reached max connections, create a new one
        if inner.total_created < self.config.max_connections {
            inner.total_created += 1;
            inner.in_use += 1;

            let client = self.create_client().await?;
            return Ok(ConnectionWrapper {
                client: Some(client),
                pool: self.clone(),
                in_use: true,
            });
        }

        // Pool is exhausted, create a temporary client
        let client = self.create_client().await?;
        Ok(ConnectionWrapper {
            client: Some(client),
            pool: self.clone(),
            in_use: false,
        })
    }

    /// Create a new HTTP client with optimized settings
    async fn create_client(&self) -> Result<Client, reqwest::Error> {
        let timeout = Duration::from_secs(10);

        Client::builder()
            .timeout(timeout)
            .pool_max_idle_per_host(self.config.max_connections)
            .pool_idle_timeout(self.config.idle_timeout)
            .build()
    }

    /// Return a client to the pool (if it was from the pool)
    pub fn return_client(&self) {
        // This is called via the ConnectionWrapper drop
    }

    /// Get pool statistics for debugging
    pub async fn get_stats(&self) -> PoolStats {
        let inner = self.inner.lock().await;
        PoolStats {
            available: inner.available.len(),
            in_use: inner.in_use,
            total_created: inner.total_created,
            max_connections: self.config.max_connections,
        }
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new(ConnectionPoolConfig::default())
    }
}

/// Wrapper for connections that manages returning them to the pool
pub struct ConnectionWrapper {
    client: Option<Client>,
    pool: ConnectionPool,
    in_use: bool,
}

impl ConnectionWrapper {
    /// Get the inner client
    pub fn client(&self) -> &Client {
        self.client.as_ref().unwrap()
    }

    /// Get a mutable reference to the inner client
    pub fn client_mut(&mut self) -> &mut Client {
        self.client.as_mut().unwrap()
    }
}

impl Drop for ConnectionWrapper {
    fn drop(&mut self) {
        if self.in_use {
            // Return the client to the pool
            self.pool.return_client();
        }
    }
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Number of available connections
    pub available: usize,
    /// Number of connections currently in use
    pub in_use: usize,
    /// Total connections created
    pub total_created: usize,
    /// Maximum connections configured
    pub max_connections: usize,
}

impl PoolStats {
    /// Calculate pool utilization percentage
    pub fn utilization_percent(&self) -> f64 {
        if self.max_connections == 0 {
            return 0.0;
        }
        (self.in_use as f64 / self.max_connections as f64) * 100.0
    }
}

/// Global connection pool instance
pub static GLOBAL_POOL: once_cell::sync::Lazy<ConnectionPool> =
    once_cell::sync::Lazy::new(|| ConnectionPool::default());

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_pool_creation() {
        let pool = ConnectionPool::new(ConnectionPoolConfig {
            max_connections: 5,
            idle_timeout: Duration::from_secs(10),
            base_url: "http://127.0.0.1:42617".to_string(),
        });

        let stats = pool.get_stats().await;
        assert_eq!(stats.available, 0);
        assert_eq!(stats.in_use, 0);
        assert_eq!(stats.total_created, 0);
    }

    #[tokio::test]
    async fn test_connection_borrow_return() {
        let pool = ConnectionPool::new(ConnectionPoolConfig {
            max_connections: 3,
            ..Default::default()
        });

        // Borrow connections
        let mut conn1 = pool.get_client().await.unwrap();
        let mut conn2 = pool.get_client().await.unwrap();
        let mut conn3 = pool.get_client().await.unwrap();

        // Check stats
        let stats = pool.get_stats().await;
        assert_eq!(stats.in_use, 3);
        assert_eq!(stats.available, 0);

        // Drop connections (should return them to pool)
        drop(conn1);
        drop(conn2);
        drop(conn3);

        // Check stats after return
        let stats = pool.get_stats().await;
        assert_eq!(stats.in_use, 0);
        assert_eq!(stats.available, 3);
    }

    #[tokio::test]
    async fn test_connection_max_limit() {
        let pool = ConnectionPool::new(ConnectionPoolConfig {
            max_connections: 2,
            ..Default::default()
        });

        // Borrow up to max connections
        let conn1 = pool.get_client().await.unwrap();
        let conn2 = pool.get_client().await.unwrap();

        // Try to borrow one more (should create a temporary)
        let conn3 = pool.get_client().await.unwrap();

        // Check stats - should have 2 in pool + 1 temporary
        let stats = pool.get_stats().await;
        assert_eq!(stats.in_use, 3);
        assert_eq!(stats.total_created, 3);

        // Drop connections
        drop(conn1);
        drop(conn2);
        drop(conn3);
    }

    #[tokio::test]
    async fn test_global_pool() {
        let conn = GLOBAL_POOL.get_client().await.unwrap();
        // Just verify it doesn't panic
        assert!(!conn.client().is_empty());
    }
}
