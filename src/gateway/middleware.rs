//! Gateway middleware: rate limiting, idempotency, and client IP parsing.
//!
//! This module provides:
//! - SlidingWindowRateLimiter: Time-window based rate limiting
//! - GatewayRateLimiter: Gateway-specific rate limiting (pair/webhook)
//! - IdempotencyStore: Idempotency key tracking for request deduplication
//! - Client IP parsing utilities (parse_client_ip, forwarded_client_ip)
//! - Webhook secret hashing (hash_webhook_secret)
//! - WhatsApp signature verification (verify_whatsapp_signature)

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::http::HeaderMap;

/// How often the rate limiter sweeps stale IP entries from its map.
const RATE_LIMITER_SWEEP_INTERVAL_SECS: u64 = 300; // 5 minutes

/// Sliding window rate limiter with periodic cleanup and eviction under cardinality pressure.
#[derive(Debug)]
pub struct SlidingWindowRateLimiter {
    limit_per_window: u32,
    window: Duration,
    max_keys: usize,
    requests: std::sync::Mutex<(HashMap<String, Vec<Instant>>, Instant)>,
}

impl SlidingWindowRateLimiter {
    fn new(limit_per_window: u32, window: Duration, max_keys: usize) -> Self {
        Self {
            limit_per_window,
            window,
            max_keys: max_keys.max(1),
            requests: std::sync::Mutex::new((HashMap::new(), Instant::now())),
        }
    }

    fn prune_stale(requests: &mut HashMap<String, Vec<Instant>>, cutoff: Instant) {
        requests.retain(|_, timestamps| {
            timestamps.retain(|t| *t > cutoff);
            !timestamps.is_empty()
        });
    }

    fn allow(&self, key: &str) -> bool {
        if self.limit_per_window == 0 {
            return true;
        }

        let now = Instant::now();
        let cutoff = now.checked_sub(self.window).unwrap_or_else(Instant::now);

        let mut guard = self.requests.lock().unwrap();
        let (requests, last_sweep) = &mut *guard;

        // Periodic sweep: remove keys with no recent requests
        if last_sweep.elapsed() >= Duration::from_secs(RATE_LIMITER_SWEEP_INTERVAL_SECS) {
            Self::prune_stale(requests, cutoff);
            *last_sweep = now;
        }

        if !requests.contains_key(key) && requests.len() >= self.max_keys {
            // Opportunistic stale cleanup before eviction under cardinality pressure.
            Self::prune_stale(requests, cutoff);
            *last_sweep = now;

            if requests.len() >= self.max_keys {
                let evict_key = requests
                    .iter()
                    .min_by_key(|(_, timestamps)| timestamps.last().copied().unwrap_or(cutoff))
                    .map(|(k, _)| k.clone());
                if let Some(evict_key) = evict_key {
                    requests.remove(&evict_key);
                }
            }
        }

        let entry = requests.entry(key.to_owned()).or_default();
        entry.retain(|instant| *instant > cutoff);

        if entry.len() >= self.limit_per_window as usize {
            return false;
        }

        entry.push(now);
        true
    }
}

/// Gateway-specific rate limiter with separate limits for pairing and webhooks.
#[derive(Debug)]
pub struct GatewayRateLimiter {
    pair: SlidingWindowRateLimiter,
    webhook: SlidingWindowRateLimiter,
}

impl GatewayRateLimiter {
    pub fn new(pair_per_minute: u32, webhook_per_minute: u32, max_keys: usize) -> Self {
        let window = Duration::from_secs(60);
        Self {
            pair: SlidingWindowRateLimiter::new(pair_per_minute, window, max_keys),
            webhook: SlidingWindowRateLimiter::new(webhook_per_minute, window, max_keys),
        }
    }

    pub fn allow_pair(&self, key: &str) -> bool {
        self.pair.allow(key)
    }

    pub fn allow_webhook(&self, key: &str) -> bool {
        self.webhook.allow(key)
    }
}

/// Idempotency store for tracking request deduplication keys with TTL.
#[derive(Debug)]
pub struct IdempotencyStore {
    ttl: Duration,
    max_keys: usize,
    keys: std::sync::Mutex<HashMap<String, Instant>>,
}

impl IdempotencyStore {
    pub fn new(ttl: Duration, max_keys: usize) -> Self {
        Self {
            ttl,
            max_keys: max_keys.max(1),
            keys: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Returns true if this key is new and is now recorded.
    pub fn record_if_new(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut keys = self.keys.lock().unwrap();

        keys.retain(|_, seen_at| now.duration_since(*seen_at) < self.ttl);

        if keys.contains_key(key) {
            return false;
        }

        if keys.len() >= self.max_keys {
            let evict_key = keys
                .iter()
                .min_by_key(|(_, seen_at)| *seen_at)
                .map(|(k, _)| k.clone());
            if let Some(evict_key) = evict_key {
                keys.remove(&evict_key);
            }
        }

        keys.insert(key.to_owned(), now);
        true
    }
}

/// Parse a client IP address from a header value.
/// Handles plain IPs, socket addresses, and IPv6 addresses.
pub fn parse_client_ip(value: &str) -> Option<IpAddr> {
    let value = value.trim().trim_matches('"').trim();
    if value.is_empty() {
        return None;
    }

    if let Ok(ip) = value.parse::<IpAddr>() {
        return Some(ip);
    }

    if let Ok(addr) = value.parse::<SocketAddr>() {
        return Some(addr.ip());
    }

    let value = value.trim_matches(['[', ']']);
    value.parse::<IpAddr>().ok()
}

/// Extract client IP from X-Forwarded-For or X-Real-IP headers.
pub fn forwarded_client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    if let Some(xff) = headers.get("X-Forwarded-For").and_then(|v| v.to_str().ok()) {
        for candidate in xff.split(',') {
            if let Some(ip) = parse_client_ip(candidate) {
                return Some(ip);
            }
        }
    }

    headers
        .get("X-Real-IP")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_client_ip)
}

/// Generate client key from request peer address and/or forwarded headers.
pub fn client_key_from_request(
    peer_addr: Option<SocketAddr>,
    headers: &HeaderMap,
    trust_forwarded_headers: bool,
) -> String {
    if trust_forwarded_headers {
        if let Some(ip) = forwarded_client_ip(headers) {
            return ip.to_string();
        }
    }

    peer_addr
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Hash a webhook secret using SHA-256.
pub fn hash_webhook_secret(value: &str) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(value.as_bytes());
    hex::encode(digest)
}

/// Verify WhatsApp webhook signature (X-Hub-Signature-256).
///
/// WhatsApp signs webhook payloads with HMAC-SHA256 using the app secret.
/// The signature is provided as `sha256=<hex_digest>` in the X-Hub-Signature-256 header.
pub fn verify_whatsapp_signature(app_secret: &str, body: &[u8], signature_header: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    // Extract signature from header (format: "sha256=<hex>")
    let signature = signature_header.strip_prefix("sha256=").unwrap_or(signature_header);

    // Decode hex signature
    let expected_sig = match hex::decode(signature) {
        Ok(sig) => sig,
        Err(_) => return false,
    };

    // Compute HMAC of body using app secret
    let mut mac = match HmacSha256::new_from_slice(app_secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(body);
    let computed_sig = mac.finalize().into_bytes();

    // Constant-time comparison to prevent timing attacks
    constant_time_eq(&expected_sig, &computed_sig)
}

/// Constant-time comparison for secrets to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }

    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_client_ip() {
        assert_eq!(
            parse_client_ip("127.0.0.1"),
            Some(IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)))
        );
        assert_eq!(
            parse_client_ip("::1"),
            Some(IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)))
        );
        assert_eq!(parse_client_ip(""), None);
        assert_eq!(parse_client_ip("invalid"), None);
    }

    #[test]
    fn test_hash_webhook_secret() {
        let secret = "test-secret";
        let hash1 = hash_webhook_secret(secret);
        let hash2 = hash_webhook_secret(secret);
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, secret);
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = SlidingWindowRateLimiter::new(2, Duration::from_secs(1), 100);
        assert!(limiter.allow("key1"));
        assert!(limiter.allow("key1"));
        assert!(!limiter.allow("key1"));
    }

    #[test]
    fn test_idempotency_store() {
        let store = IdempotencyStore::new(Duration::from_secs(1), 100);
        assert!(store.record_if_new("key1"));
        assert!(!store.record_if_new("key1"));
    }
}
