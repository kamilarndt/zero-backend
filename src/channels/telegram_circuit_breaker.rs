// Zero-Bloat Circuit Breaker for Telegram API
// Protects against cascading failures

use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::time::Duration;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub max_failures: u32,
    pub timeout_ms: u64,
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            max_failures: 5,
            timeout_ms: 60_000, // 60 seconds
            half_open_max_calls: 3,
        }
    }
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    state: AtomicU8, // Stores CircuitState as u8
    failure_count: AtomicU32,
    last_failure_time: AtomicU64,
    half_open_calls: AtomicU32,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default config
    pub fn new() -> Self {
        Self::with_config(CircuitBreakerConfig::default())
    }

    /// Create a new circuit breaker with custom config
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            state: AtomicU8::new(CircuitState::Closed as u8),
            failure_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
            half_open_calls: AtomicU32::new(0),
            config,
        }
    }

    /// Check if request is allowed
    pub fn allow_request(&self) -> bool {
        let current_state = self.get_state();

        match current_state {
            CircuitState::Closed => {
                self.record_attempt();
                true
            }
            CircuitState::Open => {
                // Check if timeout has elapsed
                let last_fail = self.last_failure_time.load(Ordering::Relaxed);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let elapsed = Duration::from_millis(now.saturating_sub(last_fail));

                if elapsed >= Duration::from_millis(self.config.timeout_ms) {
                    // Transition to HalfOpen
                    self.set_state(CircuitState::HalfOpen);
                    self.half_open_calls.store(0, Ordering::Relaxed);
                    tracing::info!("Circuit breaker transitioning from Open to HalfOpen");
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                let calls = self.half_open_calls.load(Ordering::Relaxed);
                if calls >= self.config.half_open_max_calls {
                    // Too many calls, go back to Open
                    self.set_state(CircuitState::Open);
                    false
                } else {
                    self.record_attempt();
                    true
                }
            }
        }
    }

    /// Record a successful request
    pub fn record_success(&self) {
        let current_state = self.get_state();

        match current_state {
            CircuitState::Closed => {
                // Reset failure count
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::HalfOpen => {
                // If enough successes, go to Closed
                let calls = self.half_open_calls.fetch_add(1, Ordering::Relaxed);
                if calls + 1 >= self.config.half_open_max_calls {
                    self.set_state(CircuitState::Closed);
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.half_open_calls.store(0, Ordering::Relaxed);
                    tracing::info!("Circuit breaker closed after successful HalfOpen period");
                }
            }
            CircuitState::Open => {
                // Should not happen, but reset anyway
                self.set_state(CircuitState::Closed);
                self.failure_count.store(0, Ordering::Relaxed);
            }
        }
    }

    /// Record a failed request
    pub fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed);

        let last_fail = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.last_failure_time.store(last_fail, Ordering::Relaxed);

        if failures >= self.config.max_failures {
            self.set_state(CircuitState::Open);
            tracing::warn!("Circuit breaker opened after {} failures", failures);
        }

        // If in HalfOpen and failed, go back to Open
        let current_state = self.get_state();
        if current_state == CircuitState::HalfOpen {
            self.set_state(CircuitState::Open);
        }
    }

    /// Get current circuit state
    pub fn get_state(&self) -> CircuitState {
        match self.state.load(Ordering::Relaxed) {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }

    /// Reset circuit breaker to closed state
    pub fn reset(&self) {
        self.set_state(CircuitState::Closed);
        self.failure_count.store(0, Ordering::Relaxed);
        self.half_open_calls.store(0, Ordering::Relaxed);
        tracing::info!("Circuit breaker manually reset");
    }

    fn set_state(&self, state: CircuitState) {
        self.state.store(state as u8, Ordering::Relaxed);
    }

    fn record_attempt(&self) {
        // For HalfOpen tracking
        let current_state = self.get_state();
        if current_state == CircuitState::HalfOpen {
            self.half_open_calls.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_initially_closed() {
        let cb = CircuitBreaker::new();
        assert_eq!(cb.get_state(), CircuitState::Closed);
        assert!(cb.allow_request());
    }

    #[test]
    fn test_circuit_opens_after_failures() {
        let config = CircuitBreakerConfig {
            max_failures: 2,
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config(config);

        assert!(cb.allow_request());
        cb.record_failure();

        assert!(cb.allow_request());
        cb.record_failure();

        // Should be open now
        assert_eq!(cb.get_state(), CircuitState::Open);
        assert!(!cb.allow_request());
    }

    #[test]
    fn test_circuit_resets_on_success() {
        let config = CircuitBreakerConfig {
            max_failures: 2,
            ..Default::default()
        };
        let cb = CircuitBreaker::with_config(config);

        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);

        // Wait for timeout (simulate by resetting)
        cb.reset();
        assert_eq!(cb.get_state(), CircuitState::Closed);
        assert!(cb.allow_request());

        // Success should keep it closed
        cb.record_success();
        assert_eq!(cb.get_state(), CircuitState::Closed);
    }
}
