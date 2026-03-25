//! # Rate-Aware Router
//!
//! Intelligent routing that selects providers based on:
//! - Priority order
//! - Rate limit status
//! - Preemptive fallback thresholds

use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Provider configuration entry
#[derive(Debug, Clone)]
pub struct ProviderEntry {
    /// Unique provider identifier
    pub id: String,
    /// Provider name (e.g., "anthropic", "openai")
    pub name: String,
    /// Priority for routing (lower = higher priority)
    pub priority: u32,
    /// Base URL for API requests
    pub base_url: String,
    /// Models supported by this provider
    pub models: Vec<String>,
    /// Rate limit configuration
    pub rate_limit: RateLimitConfig,
    /// Whether provider is currently active
    pub active: bool,
}

impl ProviderEntry {
    /// Create a new provider entry
    pub fn new(
        id: String,
        name: String,
        priority: u32,
        base_url: String,
        models: Vec<String>,
        rate_limit: RateLimitConfig,
    ) -> Self {
        Self {
            id,
            name,
            priority,
            base_url,
            models,
            rate_limit,
            active: true,
        }
    }
}

/// Rate limit configuration for a provider
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Requests per minute limit
    pub rpm_limit: Option<u32>,
    /// Tokens per minute limit
    pub tpm_limit: Option<u32>,
    /// Daily request limit
    pub daily_limit: Option<u32>,
}

impl RateLimitConfig {
    /// Create a new rate limit configuration
    pub fn new(
        rpm_limit: Option<u32>,
        tpm_limit: Option<u32>,
        daily_limit: Option<u32>,
    ) -> Self {
        Self {
            rpm_limit,
            tpm_limit,
            daily_limit,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            rpm_limit: None,
            tpm_limit: None,
            daily_limit: None,
        }
    }
}

/// Tracks rate limit usage for a specific provider
#[derive(Debug, Clone)]
pub struct RateLimitTracker {
    /// Provider identifier
    pub provider_id: String,
    /// Requests made in current minute
    pub rpm_current: u32,
    /// Tokens used in current minute
    pub tpm_current: u32,
    /// Requests made today
    pub daily_current: u32,
    /// Last update timestamp
    pub last_update: DateTime<Utc>,
    /// Minute window start
    pub minute_window_start: DateTime<Utc>,
    /// Daily window start
    pub daily_window_start: DateTime<Utc>,
}

impl RateLimitTracker {
    /// Create a new rate limit tracker
    pub fn new(provider_id: String) -> Self {
        let now = Utc::now();
        Self {
            provider_id,
            rpm_current: 0,
            tpm_current: 0,
            daily_current: 0,
            last_update: now,
            minute_window_start: now,
            daily_window_start: now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc(),
        }
    }

    /// Reset minute counters if window has expired
    pub fn reset_minute_if_expired(&mut self) {
        let now = Utc::now();
        if now.signed_duration_since(self.minute_window_start).num_seconds() >= 60 {
            self.rpm_current = 0;
            self.tpm_current = 0;
            self.minute_window_start = now;
            self.last_update = now;
        }
    }

    /// Reset daily counters if window has expired
    pub fn reset_daily_if_expired(&mut self) {
        let now = Utc::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        if self.daily_window_start < today_start {
            self.daily_current = 0;
            self.daily_window_start = today_start;
            self.last_update = now;
        }
    }

    /// Check if provider is at rate limit
    pub fn is_rate_limited(&mut self, config: &RateLimitConfig) -> bool {
        self.reset_minute_if_expired();
        self.reset_daily_if_expired();

        if let Some(rpm_limit) = config.rpm_limit {
            if self.rpm_current >= rpm_limit {
                return true;
            }
        }

        if let Some(tpm_limit) = config.tpm_limit {
            if self.tpm_current >= tpm_limit {
                return true;
            }
        }

        if let Some(daily_limit) = config.daily_limit {
            if self.daily_current >= daily_limit {
                return true;
            }
        }

        false
    }

    /// Check if provider is approaching rate limit (preemptive fallback)
    pub fn is_approaching_limit(&mut self, config: &RateLimitConfig, threshold: f64) -> bool {
        self.reset_minute_if_expired();
        self.reset_daily_if_expired();

        if let Some(rpm_limit) = config.rpm_limit {
            if (self.rpm_current as f64 / rpm_limit as f64) >= threshold {
                return true;
            }
        }

        if let Some(tpm_limit) = config.tpm_limit {
            if (self.tpm_current as f64 / tpm_limit as f64) >= threshold {
                return true;
            }
        }

        if let Some(daily_limit) = config.daily_limit {
            if (self.daily_current as f64 / daily_limit as f64) >= threshold {
                return true;
            }
        }

        false
    }

    /// Record a request
    pub fn record_request(&mut self, tokens: u32) {
        self.reset_minute_if_expired();
        self.reset_daily_if_expired();

        self.rpm_current += 1;
        self.tpm_current += tokens;
        self.daily_current += 1;
        self.last_update = Utc::now();
    }

    /// Get current usage ratios
    pub fn usage_ratios(&mut self, config: &RateLimitConfig) -> (f64, f64, f64) {
        self.reset_minute_if_expired();
        self.reset_daily_if_expired();

        let rpm_ratio = config.rpm_limit
            .map(|limit| self.rpm_current as f64 / limit as f64)
            .unwrap_or(0.0);

        let tpm_ratio = config.tpm_limit
            .map(|limit| self.tpm_current as f64 / limit as f64)
            .unwrap_or(0.0);

        let daily_ratio = config.daily_limit
            .map(|limit| self.daily_current as f64 / limit as f64)
            .unwrap_or(0.0);

        (rpm_ratio, tpm_ratio, daily_ratio)
    }
}

/// Result of routing decision
#[derive(Debug, Clone)]
pub enum RouteDecision {
    /// Successful routing to a provider
    Success {
        provider_id: String,
        provider_name: String,
        base_url: String,
    },
    /// No available providers (all rate limited or inactive)
    NoProviders,
    /// All providers exhausted (fallback chain failed)
    AllProvidersExhausted {
        attempted: Vec<String>,
        last_error: String,
    },
}

/// Rate-aware router that selects providers based on priority and rate limits
pub struct RateAwareRouter {
    /// Available providers (sorted by priority)
    providers: Vec<ProviderEntry>,
    /// Rate limit trackers for each provider
    trackers: HashMap<String, RateLimitTracker>,
    /// Preemptive fallback threshold (0.0-1.0)
    fallback_threshold: f64,
}

impl RateAwareRouter {
    /// Create a new rate-aware router
    pub fn new(providers: Vec<ProviderEntry>) -> Self {
        let mut router = Self {
            providers: Vec::new(),
            trackers: HashMap::new(),
            fallback_threshold: 0.9, // 90% by default
        };

        // Add providers and initialize trackers
        for provider in providers {
            let provider_id = provider.id.clone();
            router.trackers.insert(
                provider_id.clone(),
                RateLimitTracker::new(provider_id),
            );
            router.providers.push(provider);
        }

        // Sort by priority (lower number = higher priority)
        router.providers.sort_by_key(|p| p.priority);

        router
    }

    /// Set the fallback threshold
    pub fn set_fallback_threshold(&mut self, threshold: f64) {
        self.fallback_threshold = threshold.clamp(0.0, 1.0);
    }

    /// Get the fallback threshold
    pub fn fallback_threshold(&self) -> f64 {
        self.fallback_threshold
    }

    /// Add a new provider
    pub fn add_provider(&mut self, provider: ProviderEntry) {
        let provider_id = provider.id.clone();
        self.trackers.insert(
            provider_id.clone(),
            RateLimitTracker::new(provider_id),
        );
        self.providers.push(provider);
        self.providers.sort_by_key(|p| p.priority);
    }

    /// Remove a provider
    pub fn remove_provider(&mut self, provider_id: &str) -> bool {
        if self.trackers.remove(provider_id).is_some() {
            self.providers.retain(|p| p.id != provider_id);
            true
        } else {
            false
        }
    }

    /// Get all providers
    pub fn providers(&self) -> &[ProviderEntry] {
        &self.providers
    }

    /// Get tracker for a specific provider
    pub fn tracker(&self, provider_id: &str) -> Option<&RateLimitTracker> {
        self.trackers.get(provider_id)
    }

    /// Get mutable tracker for a specific provider
    pub fn tracker_mut(&mut self, provider_id: &str) -> Option<&mut RateLimitTracker> {
        self.trackers.get_mut(provider_id)
    }

    /// Route a request to the best available provider
    ///
    /// This method implements the fallback chain:
    /// 1. Try providers in priority order
    /// 2. Skip providers at rate limit or approaching limit
    /// 3. Skip inactive providers
    /// 4. Return first available provider
    pub fn route(&self, model: &str) -> RouteDecision {
        let mut attempted = Vec::new();

        for provider in &self.providers {
            // Check if provider supports the requested model
            if !provider.models.contains(&model.to_string()) {
                continue;
            }

            // Skip inactive providers
            if !provider.active {
                continue;
            }

            // Get tracker for this provider
            let tracker = match self.trackers.get(&provider.id) {
                Some(t) => t,
                None => continue,
            };

            // Check if provider is at rate limit using immutable checks
            if self.check_rate_limited(tracker, &provider.rate_limit) {
                attempted.push(provider.id.clone());
                continue;
            }

            // Check if provider is approaching rate limit (preemptive fallback)
            if self.check_approaching_limit(tracker, &provider.rate_limit, self.fallback_threshold) {
                attempted.push(provider.id.clone());
                continue;
            }

            // This provider is available!
            return RouteDecision::Success {
                provider_id: provider.id.clone(),
                provider_name: provider.name.clone(),
                base_url: provider.base_url.clone(),
            };
        }

        // No providers available
        if self.providers.is_empty() {
            RouteDecision::NoProviders
        } else {
            RouteDecision::AllProvidersExhausted {
                attempted,
                last_error: "All providers are rate limited, inactive, or do not support the requested model".to_string(),
            }
        }
    }

    /// Check if tracker is rate limited (immutable helper)
    fn check_rate_limited(&self, tracker: &RateLimitTracker, config: &RateLimitConfig) -> bool {
        // Check current values without mutating
        if let Some(rpm_limit) = config.rpm_limit {
            if tracker.rpm_current >= rpm_limit {
                return true;
            }
        }

        if let Some(tpm_limit) = config.tpm_limit {
            if tracker.tpm_current >= tpm_limit {
                return true;
            }
        }

        if let Some(daily_limit) = config.daily_limit {
            if tracker.daily_current >= daily_limit {
                return true;
            }
        }

        false
    }

    /// Check if tracker is approaching rate limit (immutable helper)
    fn check_approaching_limit(&self, tracker: &RateLimitTracker, config: &RateLimitConfig, threshold: f64) -> bool {
        if let Some(rpm_limit) = config.rpm_limit {
            if rpm_limit > 0 && (tracker.rpm_current as f64 / rpm_limit as f64) >= threshold {
                return true;
            }
        }

        if let Some(tpm_limit) = config.tpm_limit {
            if tpm_limit > 0 && (tracker.tpm_current as f64 / tpm_limit as f64) >= threshold {
                return true;
            }
        }

        if let Some(daily_limit) = config.daily_limit {
            if daily_limit > 0 && (tracker.daily_current as f64 / daily_limit as f64) >= threshold {
                return true;
            }
        }

        false
    }

    /// Record a request for rate limit tracking
    pub fn record_request(&mut self, provider_id: &str, tokens: u32) -> Result<(), String> {
        let tracker = self.trackers.get_mut(provider_id)
            .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

        tracker.record_request(tokens);
        Ok(())
    }

    /// Get current usage statistics for all providers
    pub fn usage_stats(&self) -> HashMap<String, (f64, f64, f64)> {
        let mut stats = HashMap::new();

        for provider in &self.providers {
            if let Some(tracker) = self.trackers.get(&provider.id) {
                let ratios = self.get_usage_ratios(tracker, &provider.rate_limit);
                stats.insert(provider.id.clone(), ratios);
            }
        }

        stats
    }

    /// Get usage ratios without mutation (immutable helper)
    fn get_usage_ratios(&self, tracker: &RateLimitTracker, config: &RateLimitConfig) -> (f64, f64, f64) {
        let rpm_ratio = config.rpm_limit
            .map(|limit| if limit > 0 { tracker.rpm_current as f64 / limit as f64 } else { 0.0 })
            .unwrap_or(0.0);

        let tpm_ratio = config.tpm_limit
            .map(|limit| if limit > 0 { tracker.tpm_current as f64 / limit as f64 } else { 0.0 })
            .unwrap_or(0.0);

        let daily_ratio = config.daily_limit
            .map(|limit| if limit > 0 { tracker.daily_current as f64 / limit as f64 } else { 0.0 })
            .unwrap_or(0.0);

        (rpm_ratio, tpm_ratio, daily_ratio)
    }

    /// Reset all rate limit trackers
    pub fn reset_all_trackers(&mut self) {
        let provider_ids: Vec<String> = self.trackers.keys().cloned().collect();
        for provider_id in provider_ids {
            self.trackers.insert(
                provider_id.clone(),
                RateLimitTracker::new(provider_id),
            );
        }
    }

    /// Reset tracker for a specific provider
    pub fn reset_tracker(&mut self, provider_id: &str) -> Result<(), String> {
        if self.trackers.contains_key(provider_id) {
            self.trackers.insert(
                provider_id.to_string(),
                RateLimitTracker::new(provider_id.to_string()),
            );
            Ok(())
        } else {
            Err(format!("Provider not found: {}", provider_id))
        }
    }

    /// Update provider active status
    pub fn set_provider_active(&mut self, provider_id: &str, active: bool) -> Result<(), String> {
        let provider = self.providers.iter_mut()
            .find(|p| p.id == provider_id)
            .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

        provider.active = active;
        Ok(())
    }

    /// Update provider priority
    pub fn set_provider_priority(&mut self, provider_id: &str, priority: u32) -> Result<(), String> {
        {
            let provider = self.providers.iter_mut()
                .find(|p| p.id == provider_id)
                .ok_or_else(|| format!("Provider not found: {}", provider_id))?;

            provider.priority = priority;
        }

        // Re-sort by priority
        self.providers.sort_by_key(|p| p.priority);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_provider(id: &str, priority: u32) -> ProviderEntry {
        ProviderEntry::new(
            id.to_string(),
            format!("Provider {}", id),
            priority,
            format!("https://api{}.example.com", id),
            vec!["claude-3-5-sonnet".to_string()],
            RateLimitConfig::new(Some(100), Some(100000), Some(10000)),
        )
    }

    #[test]
    fn test_router_initialization() {
        let providers = vec![
            create_test_provider("provider1", 1),
            create_test_provider("provider2", 2),
        ];

        let router = RateAwareRouter::new(providers);
        assert_eq!(router.providers().len(), 2);
        assert_eq!(router.providers()[0].id, "provider1");
        assert_eq!(router.providers()[1].id, "provider2");
    }

    #[test]
    fn test_route_success() {
        let providers = vec![
            create_test_provider("provider1", 1),
        ];

        let router = RateAwareRouter::new(providers);
        let decision = router.route("claude-3-5-sonnet");

        match decision {
            RouteDecision::Success { provider_id, .. } => {
                assert_eq!(provider_id, "provider1");
            },
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn test_tracker_initialization() {
        let tracker = RateLimitTracker::new("test-provider".to_string());
        assert_eq!(tracker.provider_id, "test-provider");
        assert_eq!(tracker.rpm_current, 0);
        assert_eq!(tracker.tpm_current, 0);
        assert_eq!(tracker.daily_current, 0);
    }

    #[test]
    fn test_tracker_record_request() {
        let mut tracker = RateLimitTracker::new("test-provider".to_string());
        tracker.record_request(1000);

        assert_eq!(tracker.rpm_current, 1);
        assert_eq!(tracker.tpm_current, 1000);
        assert_eq!(tracker.daily_current, 1);
    }

    #[test]
    fn test_tracker_rate_limit_check() {
        let config = RateLimitConfig::new(Some(5), None, None);
        let mut tracker = RateLimitTracker::new("test-provider".to_string());

        assert!(!tracker.is_rate_limited(&config));

        for _ in 0..5 {
            tracker.record_request(0);
        }

        assert!(tracker.is_rate_limited(&config));
    }

    #[test]
    fn test_tracker_preemptive_fallback() {
        let config = RateLimitConfig::new(Some(100), None, None);
        let mut tracker = RateLimitTracker::new("test-provider".to_string());

        // Use 90% of limit
        for _ in 0..90 {
            tracker.record_request(0);
        }

        assert!(tracker.is_approaching_limit(&config, 0.9));
        assert!(!tracker.is_approaching_limit(&config, 0.95));
    }

    #[test]
    fn test_fallback_threshold() {
        let mut router = RateAwareRouter::new(vec![]);
        router.set_fallback_threshold(0.85);
        assert_eq!(router.fallback_threshold(), 0.85);
    }

    #[test]
    fn test_add_provider() {
        let mut router = RateAwareRouter::new(vec![]);
        router.add_provider(create_test_provider("provider1", 1));
        assert_eq!(router.providers().len(), 1);
    }

    #[test]
    fn test_remove_provider() {
        let providers = vec![create_test_provider("provider1", 1)];
        let mut router = RateAwareRouter::new(providers);

        let removed = router.remove_provider("provider1");
        assert!(removed);
        assert_eq!(router.providers().len(), 0);

        let removed_again = router.remove_provider("provider1");
        assert!(!removed_again);
    }

    #[test]
    fn test_set_provider_active() {
        let providers = vec![create_test_provider("provider1", 1)];
        let mut router = RateAwareRouter::new(providers);

        router.set_provider_active("provider1", false).unwrap();
        assert!(!router.providers()[0].active);

        let result = router.set_provider_active("nonexistent", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_record_request() {
        let providers = vec![create_test_provider("provider1", 1)];
        let mut router = RateAwareRouter::new(providers);

        let result = router.record_request("provider1", 1000);
        assert!(result.is_ok());

        let tracker = router.tracker("provider1").unwrap();
        assert_eq!(tracker.tpm_current, 1000);
    }
}
