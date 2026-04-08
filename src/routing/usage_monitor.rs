//! # Usage Monitor
//!
//! Monitors API usage and syncs with Z.AI API for rate limit awareness.
//! Runs background tasks to fetch usage statistics periodically.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

/// Configuration for usage monitoring
#[derive(Debug, Clone)]
pub struct UsageSyncConfig {
    /// Enable usage monitoring
    pub enabled: bool,
    /// Sync interval in seconds
    pub interval_secs: u64,
    /// Z.AI API key for authentication
    pub api_key: Option<String>,
    /// Z.AI API base URL
    pub api_base_url: String,
}

impl Default for UsageSyncConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 300, // 5 minutes
            api_key: std::env::var("ZAI_API_KEY").ok(),
            api_base_url: "https://api.z.ai/v1".to_string(),
        }
    }
}

/// Usage statistics from Z.AI API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    /// Current period usage
    pub current_period: UsagePeriod,
    /// Previous period usage
    pub previous_period: UsagePeriod,
    /// Rate limit information
    pub rate_limits: Vec<RateLimitInfo>,
    /// Timestamp of data
    pub timestamp: DateTime<Utc>,
}

/// Usage for a specific period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePeriod {
    /// Start of period
    pub start_date: DateTime<Utc>,
    /// End of period
    pub end_date: DateTime<Utc>,
    /// Total requests made
    pub total_requests: u64,
    /// Total tokens used
    pub total_tokens: u64,
    /// Total cost in USD
    pub total_cost: f64,
}

/// Rate limit information for a specific tier/type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    /// Limit tier name (e.g., "tier1", "tier2")
    pub tier: String,
    /// Requests per minute limit
    pub rpm_limit: u32,
    /// Tokens per minute limit
    pub tpm_limit: u32,
    /// Daily request limit
    pub daily_limit: u32,
    /// Current usage percentage
    pub usage_percentage: f64,
    /// Reset time for limits
    pub reset_at: DateTime<Utc>,
}

/// Provider-specific usage data
#[derive(Debug, Clone)]
pub struct ProviderUsage {
    /// Provider identifier
    pub provider_id: String,
    /// Requests in current minute
    pub rpm_current: u32,
    /// Tokens in current minute
    pub tpm_current: u32,
    /// Requests today
    pub daily_current: u32,
    /// Usage ratios (rpm, tpm, daily)
    pub ratios: (f64, f64, f64),
    /// Last update timestamp
    pub last_update: DateTime<Utc>,
}

/// Error type for usage monitor operations
#[derive(Debug, Clone)]
pub enum UsageMonitorError {
    /// API key not configured
    NoApiKey,
    /// Network request failed
    RequestFailed(String),
    /// Invalid response format
    InvalidResponse(String),
    /// Authentication failed
    AuthenticationFailed,
    /// Rate limit exceeded
    RateLimitExceeded,
}

impl std::fmt::Display for UsageMonitorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoApiKey => write!(f, "Z.AI API key not configured"),
            Self::RequestFailed(msg) => write!(f, "Request failed: {}", msg),
            Self::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            Self::AuthenticationFailed => write!(f, "Authentication failed"),
            Self::RateLimitExceeded => write!(f, "Rate limit exceeded"),
        }
    }
}

impl std::error::Error for UsageMonitorError {}

/// Usage monitor that syncs with Z.AI API
pub struct UsageMonitor {
    /// Configuration
    config: UsageSyncConfig,
    /// Latest usage statistics
    stats: Arc<Mutex<Option<UsageStats>>>,
    /// Provider-specific usage data
    provider_usage: Arc<Mutex<std::collections::HashMap<String, ProviderUsage>>>,
    /// Whether sync loop is running
    sync_running: Arc<Mutex<bool>>,
    /// Shutdown signal
    shutdown_signal: Arc<Mutex<bool>>,
}

impl UsageMonitor {
    /// Create a new usage monitor
    pub fn new(config: UsageSyncConfig) -> Self {
        Self {
            config,
            stats: Arc::new(Mutex::new(None)),
            provider_usage: Arc::new(Mutex::new(std::collections::HashMap::new())),
            sync_running: Arc::new(Mutex::new(false)),
            shutdown_signal: Arc::new(Mutex::new(false)),
        }
    }

    /// Fetch current usage from Z.AI API
    pub async fn fetch_usage(&self) -> Result<UsageStats, UsageMonitorError> {
        // Check if API key is configured
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or(UsageMonitorError::NoApiKey)?;

        // Build request URL
        let url = format!("{}/usage", self.config.api_base_url);

        // Create HTTP client
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| UsageMonitorError::RequestFailed(e.to_string()))?;

        // Make API request
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| UsageMonitorError::RequestFailed(e.to_string()))?;

        // Check response status
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(UsageMonitorError::AuthenticationFailed);
        }

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(UsageMonitorError::RateLimitExceeded);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(UsageMonitorError::RequestFailed(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        // Parse response
        let usage: UsageStats = response
            .json()
            .await
            .map_err(|e| UsageMonitorError::InvalidResponse(e.to_string()))?;

        Ok(usage)
    }

    /// Update usage statistics
    pub async fn update_stats(&self, stats: UsageStats) {
        let mut current_stats = self.stats.lock().await;
        *current_stats = Some(stats);
    }

    /// Get current usage statistics
    pub async fn get_stats(&self) -> Option<UsageStats> {
        let stats = self.stats.lock().await;
        stats.clone()
    }

    /// Update provider-specific usage
    pub async fn update_provider_usage(
        &self,
        provider_id: String,
        rpm_current: u32,
        tpm_current: u32,
        daily_current: u32,
        ratios: (f64, f64, f64),
    ) {
        let mut usage_map = self.provider_usage.lock().await;
        usage_map.insert(
            provider_id.clone(),
            ProviderUsage {
                provider_id,
                rpm_current,
                tpm_current,
                daily_current,
                ratios,
                last_update: Utc::now(),
            },
        );
    }

    /// Get usage for a specific provider
    pub async fn get_provider_usage(&self, provider_id: &str) -> Option<ProviderUsage> {
        let usage_map = self.provider_usage.lock().await;
        usage_map.get(provider_id).cloned()
    }

    /// Get all provider usage data
    pub async fn get_all_provider_usage(&self) -> std::collections::HashMap<String, ProviderUsage> {
        let usage_map = self.provider_usage.lock().await;
        usage_map.clone()
    }

    /// Check if sync loop is running
    pub async fn is_sync_running(&self) -> bool {
        let running = self.sync_running.lock().await;
        *running
    }

    /// Start the background sync loop
    pub async fn start_sync_loop(&self) -> Result<(), UsageMonitorError> {
        // Check if already running
        let mut running = self.sync_running.lock().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        // Clone Arc references for the task
        let stats = Arc::clone(&self.stats);
        let config = self.config.clone();
        let sync_running = Arc::clone(&self.sync_running);
        let shutdown_signal = Arc::clone(&self.shutdown_signal);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.interval_secs));
            interval.tick().await; // Skip first immediate tick

            loop {
                // Check for shutdown signal
                {
                    let shutdown = shutdown_signal.lock().await;
                    if *shutdown {
                        break;
                    }
                }

                interval.tick().await;

                // Fetch usage if enabled
                if config.enabled {
                    let monitor = Self {
                        config: config.clone(),
                        stats: Arc::clone(&stats),
                        provider_usage: Arc::new(Mutex::new(std::collections::HashMap::new())),
                        sync_running: Arc::clone(&sync_running),
                        shutdown_signal: Arc::clone(&shutdown_signal),
                    };

                    match monitor.fetch_usage().await {
                        Ok(usage) => {
                            let mut current_stats = stats.lock().await;
                            *current_stats = Some(usage);
                        }
                        Err(e) => {
                            eprintln!("Usage sync error: {}", e);
                        }
                    }
                }
            }

            // Mark as not running
            let mut running = sync_running.lock().await;
            *running = false;
        });

        Ok(())
    }

    /// Stop the background sync loop
    pub async fn shutdown(&self) {
        let mut shutdown = self.shutdown_signal.lock().await;
        *shutdown = true;

        // Wait for sync loop to stop
        loop {
            let running = self.sync_running.lock().await;
            if !*running {
                break;
            }
            drop(running);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &UsageSyncConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: UsageSyncConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_sync_config_default() {
        let config = UsageSyncConfig::default();
        assert_eq!(config.interval_secs, 300);
        assert!(config.enabled);
    }

    #[test]
    fn test_usage_stats_serialization() {
        let stats = UsageStats {
            current_period: UsagePeriod {
                start_date: Utc::now(),
                end_date: Utc::now(),
                total_requests: 1000,
                total_tokens: 1000000,
                total_cost: 10.50,
            },
            previous_period: UsagePeriod {
                start_date: Utc::now(),
                end_date: Utc::now(),
                total_requests: 500,
                total_tokens: 500000,
                total_cost: 5.25,
            },
            rate_limits: vec![],
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&stats);
        assert!(json.is_ok());
    }

    #[test]
    fn test_rate_limit_info() {
        let info = RateLimitInfo {
            tier: "tier1".to_string(),
            rpm_limit: 100,
            tpm_limit: 100000,
            daily_limit: 10000,
            usage_percentage: 0.5,
            reset_at: Utc::now(),
        };

        assert_eq!(info.tier, "tier1");
        assert_eq!(info.rpm_limit, 100);
    }

    #[tokio::test]
    async fn test_usage_monitor_creation() {
        let config = UsageSyncConfig::default();
        let monitor = UsageMonitor::new(config);

        assert!(!monitor.is_sync_running().await);
        assert!(monitor.get_stats().await.is_none());
    }

    #[tokio::test]
    async fn test_usage_monitor_update_stats() {
        let config = UsageSyncConfig::default();
        let monitor = UsageMonitor::new(config);

        let stats = UsageStats {
            current_period: UsagePeriod {
                start_date: Utc::now(),
                end_date: Utc::now(),
                total_requests: 1000,
                total_tokens: 1000000,
                total_cost: 10.50,
            },
            previous_period: UsagePeriod {
                start_date: Utc::now(),
                end_date: Utc::now(),
                total_requests: 500,
                total_tokens: 500000,
                total_cost: 5.25,
            },
            rate_limits: vec![],
            timestamp: Utc::now(),
        };

        monitor.update_stats(stats.clone()).await;

        let retrieved = monitor.get_stats().await;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.current_period.total_requests, 1000);
    }

    #[tokio::test]
    async fn test_provider_usage_update() {
        let config = UsageSyncConfig::default();
        let monitor = UsageMonitor::new(config);

        monitor
            .update_provider_usage("provider1".to_string(), 10, 1000, 100, (0.1, 0.2, 0.3))
            .await;

        let usage = monitor.get_provider_usage("provider1").await;
        assert!(usage.is_some());
        let usage = usage.unwrap();
        assert_eq!(usage.provider_id, "provider1");
        assert_eq!(usage.rpm_current, 10);
        assert_eq!(usage.tpm_current, 1000);
        assert_eq!(usage.daily_current, 100);
        assert_eq!(usage.ratios, (0.1, 0.2, 0.3));
    }

    #[tokio::test]
    async fn test_get_all_provider_usage() {
        let config = UsageSyncConfig::default();
        let monitor = UsageMonitor::new(config);

        monitor
            .update_provider_usage("provider1".to_string(), 10, 1000, 100, (0.1, 0.2, 0.3))
            .await;

        monitor
            .update_provider_usage("provider2".to_string(), 20, 2000, 200, (0.2, 0.4, 0.6))
            .await;

        let all_usage = monitor.get_all_provider_usage().await;
        assert_eq!(all_usage.len(), 2);
        assert!(all_usage.contains_key("provider1"));
        assert!(all_usage.contains_key("provider2"));
    }

    #[test]
    fn test_error_display() {
        let err = UsageMonitorError::NoApiKey;
        assert_eq!(format!("{}", err), "Z.AI API key not configured");

        let err = UsageMonitorError::RequestFailed("network error".to_string());
        assert_eq!(format!("{}", err), "Request failed: network error");

        let err = UsageMonitorError::AuthenticationFailed;
        assert_eq!(format!("{}", err), "Authentication failed");

        let err = UsageMonitorError::RateLimitExceeded;
        assert_eq!(format!("{}", err), "Rate limit exceeded");
    }
}
