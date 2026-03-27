//! Request deduplication cache
//!
//! Prevents duplicate simultaneous API calls for the same endpoint.
//! Uses a pending flag to synchronize concurrent requests.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Request deduplication cache
///
/// Prevents multiple simultaneous requests for the same endpoint.
/// NOT for reducing API call frequency (polling interval handles that).
///
/// # Purpose
/// - If multiple tasks request the same endpoint simultaneously
/// - Only one request is made, others wait for the result
/// - Results are cached for 5 seconds
///
/// # Example
/// ```rust
/// use crate::state::cache::RequestCache;
///
/// let cache = RequestCache::new(Duration::from_secs(5));
///
/// // Check if we have cached data or should fetch
/// if let Some(cached) = cache.try_get_or_mark_pending("swarm") {
///     // Use cached data
/// } else {
///     // Fetch from API, then cache result
///     cache.put_and_ready("swarm", data);
/// }
/// ```
pub struct RequestCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    ttl: Duration,
}

struct CacheEntry {
    data: serde_json::Value,
    timestamp: Instant,
    pending: bool, // true if request is in flight
}

impl RequestCache {
    /// Create new request cache
    ///
    /// # Arguments
    /// - `ttl`: Time-to-live for cached entries (recommend 5 seconds)
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    /// Try to get cached data, or mark as pending if fetching
    ///
    /// # Returns
    /// - `Some(data)`: Cached data is available, use it
    /// - `None`: No cached data, or request is in flight (caller should wait or fetch)
    ///
    /// # Behavior
    /// - If fresh cache exists: return cached data
    /// - If request is pending: return None (caller should wait and retry)
    /// - If cache is expired or new: mark as pending and return None (caller should fetch)
    pub fn try_get_or_mark_pending(&self, key: &str) -> Option<serde_json::Value> {
        let mut entries = self.entries.write().unwrap();

        if let Some(entry) = entries.get_mut(key) {
            if entry.timestamp.elapsed() < self.ttl {
                if entry.pending {
                    // Request is in flight, caller should wait
                    None
                } else {
                    // Return cached data
                    Some(entry.data.clone())
                }
            } else {
                // Expired, mark as pending
                entry.pending = true;
                entry.timestamp = Instant::now();
                None
            }
        } else {
            // New entry, mark as pending
            entries.insert(key.to_string(), CacheEntry {
                data: serde_json::Value::Null,
                timestamp: Instant::now(),
                pending: true,
            });
            None
        }
    }

    /// Store fetched data and mark as ready
    ///
    /// # Arguments
    /// - `key`: Cache key (e.g., "swarm", "logs")
    /// - `data`: Fetched data to cache
    pub fn put_and_ready(&self, key: &str, data: serde_json::Value) {
        let mut entries = self.entries.write().unwrap();
        if let Some(entry) = entries.get_mut(key) {
            entry.data = data;
            entry.pending = false;
        }
    }

    /// Mark request as failed (clear pending flag)
    ///
    /// Call this if API request fails so other tasks can retry
    ///
    /// # Arguments
    /// - `key`: Cache key to mark as failed
    pub fn mark_failed(&self, key: &str) {
        let mut entries = self.entries.write().unwrap();
        if let Some(entry) = entries.get_mut(key) {
            entry.pending = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_miss_returns_none() {
        let cache = RequestCache::new(Duration::from_secs(5));
        assert!(cache.try_get_or_mark_pending("test").is_none());
    }

    #[test]
    fn test_cache_hit_returns_data() {
        let cache = RequestCache::new(Duration::from_secs(5));
        let data = serde_json::json!({"test": true});

        // Mark as pending and store data
        cache.try_get_or_mark_pending("test");
        cache.put_and_ready("test", data.clone());

        // Should return cached data
        let cached = cache.try_get_or_mark_pending("test");
        assert_eq!(cached, Some(data));
    }

    #[test]
    fn test_cache_pending_blocks_concurrent() {
        let cache = RequestCache::new(Duration::from_secs(5));

        // Mark as pending
        cache.try_get_or_mark_pending("test");

        // Concurrent request should return None (wait)
        assert!(cache.try_get_or_mark_pending("test").is_none());

        // Mark as ready
        cache.put_and_ready("test", serde_json::json!({"data": 1}));

        // Now should return data
        assert!(cache.try_get_or_mark_pending("test").is_some());
    }

    #[test]
    fn test_cache_ttl_expiration() {
        let cache = RequestCache::new(Duration::from_millis(100));
        let data = serde_json::json!({"test": true});

        // Store data
        cache.try_get_or_mark_pending("test");
        cache.put_and_ready("test", data.clone());

        // Should be available immediately
        assert!(cache.try_get_or_mark_pending("test").is_some());

        // Wait for expiration (use 200ms for reliability on slow systems)
        std::thread::sleep(Duration::from_millis(200));

        // Should be expired now, returns None (marks as pending)
        assert!(cache.try_get_or_mark_pending("test").is_none());
    }

    #[test]
    fn test_mark_failed_clears_pending() {
        let cache = RequestCache::new(Duration::from_secs(5));

        // Mark as pending
        cache.try_get_or_mark_pending("test");

        // Mark as failed
        cache.mark_failed("test");

        // Should not be pending anymore
        let cached = cache.try_get_or_mark_pending("test");
        // Returns None because no data, but doesn't indicate pending
        assert!(cached.is_none());
    }
}