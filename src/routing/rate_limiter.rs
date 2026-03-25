//! Rate limiting with sliding window tracking for API request management.
//!
//! This module provides preemptive rate limiting using a sliding window algorithm
//! to track requests within time intervals and prevent hitting API limits.

use std::collections::VecDeque;
use std::time::Instant;

const SLIDING_WINDOW_SECONDS: u64 = 60;
const MAX_REQUESTS_PER_MINUTE: usize = 60; // Conservative default
const MAX_DAILY_REQUESTS: usize = 1000; // Conservative default
const USAGE_THRESHOLD: f64 = 0.9; // Preemptive threshold at 90%

/// Tracks API requests using a sliding window algorithm for rate limiting.
///
/// Maintains both short-term (per-minute) and long-term (daily) request counts
/// to provide comprehensive rate limiting with preemptive throttling.
pub struct RateLimitTracker {
    /// Sliding window of request timestamps within the last 60 seconds
    sliding_window: VecDeque<Instant>,

    /// Total requests made in the current daily period
    daily_requests: usize,

    /// Time when the daily counter should reset (24 hours from start)
    daily_reset_time: Instant,

    /// Maximum requests allowed per minute (configurable via API sync)
    max_rpm: usize,

    /// Maximum requests allowed per day (configurable via API sync)
    max_daily: usize,
}

impl Default for RateLimitTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimitTracker {
    /// Creates a new rate limit tracker with default thresholds.
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            sliding_window: VecDeque::new(),
            daily_requests: 0,
            daily_reset_time: now + std::time::Duration::from_secs(86400), // 24 hours
            max_rpm: MAX_REQUESTS_PER_MINUTE,
            max_daily: MAX_DAILY_REQUESTS,
        }
    }

    /// Creates a new tracker with custom limits.
    pub fn with_limits(max_rpm: usize, max_daily: usize) -> Self {
        let mut tracker = Self::new();
        tracker.max_rpm = max_rpm;
        tracker.max_daily = max_daily;
        tracker
    }

    /// Checks if a new request can be accepted based on current usage.
    ///
    /// Returns `false` if:
    /// - Current minute usage exceeds 90% of RPM limit
    /// - Daily usage exceeds 90% of daily limit
    /// - Daily period has expired
    ///
    /// Also automatically cleans up stale entries from the sliding window.
    pub fn can_accept(&mut self) -> bool {
        self.cleanup_window();

        // Check if daily period needs reset
        let now = Instant::now();
        if now >= self.daily_reset_time {
            return false; // Force reset via record_request
        }

        // Check RPM threshold (preemptive at 90%)
        let rpm_usage = self.sliding_window.len() as f64 / self.max_rpm as f64;
        if rpm_usage > USAGE_THRESHOLD {
            return false;
        }

        // Check daily threshold (preemptive at 90%)
        let daily_usage = self.daily_requests as f64 / self.max_daily as f64;
        if daily_usage > USAGE_THRESHOLD {
            return false;
        }

        true
    }

    /// Records a new request in the tracking system.
    ///
    /// Adds the current timestamp to the sliding window and increments
    /// the daily counter. Handles automatic daily reset if needed.
    pub fn record_request(&mut self) {
        let now = Instant::now();

        // Check for daily reset
        if now >= self.daily_reset_time {
            self.daily_requests = 0;
            self.daily_reset_time = now + std::time::Duration::from_secs(86400);
        }

        // Add to sliding window
        self.sliding_window.push_back(now);
        self.daily_requests += 1;

        // Cleanup after adding to maintain window size
        self.cleanup_window();
    }

    /// Calculates the current usage percentage (0.0 to 1.0).
    ///
    /// Returns the higher of RPM or daily usage to provide a conservative
    /// estimate of overall quota consumption.
    pub fn usage_percentage(&mut self) -> f64 {
        self.cleanup_window();

        let rpm_usage = self.sliding_window.len() as f64 / self.max_rpm as f64;
        let daily_usage = self.daily_requests as f64 / self.max_daily as f64;

        rpm_usage.max(daily_usage)
    }

    /// Synchronizes limits from API usage data.
    ///
    /// Updates the RPM and daily limits based on actual API quota information
    /// retrieved from the UsageMonitor. Resets counters if a new quota period
    /// has begun.
    pub fn sync_from_api(&mut self, api_rpm: Option<usize>, api_daily: Option<usize>) {
        if let Some(rpm) = api_rpm {
            self.max_rpm = rpm;
        }

        if let Some(daily) = api_daily {
            // Check if we're in a new quota period (significantly lower daily count)
            if daily < self.daily_requests {
                self.daily_requests = daily;
            }
            self.max_daily = daily;
        }
    }

    /// Removes entries older than 60 seconds from the sliding window.
    ///
    /// This maintains the accuracy of the sliding window and prevents
    /// unbounded memory growth.
    fn cleanup_window(&mut self) {
        let cutoff = Instant::now() - std::time::Duration::from_secs(SLIDING_WINDOW_SECONDS);

        while let Some(&front) = self.sliding_window.front() {
            if front < cutoff {
                self.sliding_window.pop_front();
            } else {
                break;
            }
        }
    }

    /// Returns the current number of requests in the sliding window.
    pub fn current_rpm(&mut self) -> usize {
        self.cleanup_window();
        self.sliding_window.len()
    }

    /// Returns the current daily request count.
    pub fn current_daily(&self) -> usize {
        self.daily_requests
    }

    /// Returns the time until the next daily reset.
    pub fn time_until_reset(&self) -> Option<std::time::Duration> {
        self.daily_reset_time.checked_duration_since(Instant::now())
    }

    /// Resets all counters (for testing or manual intervention).
    pub fn reset(&mut self) {
        self.sliding_window.clear();
        self.daily_requests = 0;
        self.daily_reset_time = Instant::now() + std::time::Duration::from_secs(86400);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sliding_window_cleanup() {
        let mut tracker = RateLimitTracker::new();

        // Simulate 10 requests spread over time
        for _ in 0..10 {
            tracker.record_request();
        }

        assert_eq!(tracker.current_rpm(), 10);

        // After cleanup time passes, window should be empty
        // (Note: this would require sleeping or mocking time in real tests)
    }

    #[test]
    fn test_usage_threshold() {
        let mut tracker = RateLimitTracker::with_limits(10, 100);

        // Should accept requests up to 90% of limit (9 out of 10 = 90%)
        for i in 0..9 {
            assert!(tracker.can_accept(), "Should accept request {}", i + 1);
            tracker.record_request();
        }

        // After 9 requests, usage is exactly 90%, should still accept (threshold is > 90%)
        assert!(tracker.can_accept(), "Should accept at exactly 90% (9/10)");

        // Record 10th request, now at 100% which exceeds 90% threshold
        tracker.record_request();

        // Now at 10/10 = 100%, should reject
        assert!(!tracker.can_accept(), "Should reject at 100% (10/10)");
    }

    #[test]
    fn test_daily_reset() {
        let mut tracker = RateLimitTracker::new();
        tracker.record_request();
        assert_eq!(tracker.current_daily(), 1);

        // Reset functionality tested via manual reset or time manipulation
        tracker.reset();
        assert_eq!(tracker.current_daily(), 0);
    }
}
