//! # Routing Module
//!
//! This module provides intelligent routing for AI provider requests with:
//! - Rate-aware provider selection
//! - Usage monitoring and synchronization
//! - Request classification for optimal routing
//! - Subagent delegation capabilities

pub mod classifier;
pub mod rate_limiter;
pub mod router;
pub mod subagent;
pub mod usage_monitor;

// Re-export key types for convenience
pub use classifier::{ClassificationInput, ClassificationResult, Classifier};
pub use router::{
    ProviderEntry, RateAwareRouter,
};
pub use subagent::SubAgentManager;
pub use usage_monitor::{UsageMonitor, UsageSyncConfig};

use std::sync::Arc;
use tokio::sync::Mutex;

/// Routing module configuration
#[derive(Debug, Clone)]
pub struct RoutingConfig {
    /// Enable usage monitoring
    pub enable_monitoring: bool,
    /// Enable request classification
    pub enable_classification: bool,
    /// Enable subagent delegation
    pub enable_delegation: bool,
    /// Preemptive fallback threshold (0.0-1.0)
    pub fallback_threshold: f64,
    /// Usage sync interval in seconds
    pub sync_interval_secs: u64,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            enable_monitoring: true,
            enable_classification: true,
            enable_delegation: true,
            fallback_threshold: 0.9, // 90%
            sync_interval_secs: 300, // 5 minutes
        }
    }
}

/// Main routing manager that coordinates all routing components
pub struct RoutingManager {
    /// Rate-aware router
    router: Arc<Mutex<RateAwareRouter>>,
    /// Usage monitor
    monitor: Arc<Mutex<UsageMonitor>>,
    /// Subagent manager
    manager: Arc<SubAgentManager>,
    /// Configuration
    config: RoutingConfig,
}

impl RoutingManager {
    /// Create a new routing manager
    pub fn new(providers: Vec<ProviderEntry>, config: RoutingConfig) -> Self {
        let router = Arc::new(Mutex::new(RateAwareRouter::new(providers)));
        let monitor = Arc::new(Mutex::new(UsageMonitor::new(UsageSyncConfig {
            enabled: config.enable_monitoring,
            interval_secs: config.sync_interval_secs,
            api_key: std::env::var("ZAI_API_KEY").ok(),
            api_base_url: "https://api.z.ai/v1".to_string(),
        })));
        let manager = Arc::new(SubAgentManager::new(3, 3)); // max 3 concurrent, depth 3

        Self {
            router,
            monitor,
            manager,
            config,
        }
    }

    /// Get the rate-aware router
    pub fn router(&self) -> Arc<Mutex<RateAwareRouter>> {
        Arc::clone(&self.router)
    }

    /// Get the usage monitor
    pub fn monitor(&self) -> Arc<Mutex<UsageMonitor>> {
        Arc::clone(&self.monitor)
    }

    /// Get the subagent manager
    pub fn manager(&self) -> Arc<SubAgentManager> {
        Arc::clone(&self.manager)
    }

    /// Start background tasks for monitoring and sync
    pub async fn start_background_tasks(&self) {
        if self.config.enable_monitoring {
            let monitor = Arc::clone(&self.monitor);
            tokio::spawn(async move {
                let mon = monitor.lock().await;
                if let Err(e) = mon.start_sync_loop().await {
                    eprintln!("Usage monitor sync error: {}", e);
                }
            });
        }
    }

    /// Shutdown all background tasks
    pub async fn shutdown(&self) {
        let monitor = self.monitor.lock().await;
        monitor.shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_config_default() {
        let config = RoutingConfig::default();
        assert_eq!(config.fallback_threshold, 0.9);
        assert_eq!(config.sync_interval_secs, 300);
        assert!(config.enable_monitoring);
    }
}
