//! CPU monitoring for CPU-aware scheduling.

use sysinfo::System;
use tokio::time::{sleep, Duration};

/// CPU monitor for CPU-aware scheduling.
///
/// CPU is considered "idle enough" when usage is below `(100 - threshold)` percent.
#[derive(Debug)]
pub struct CpuMonitor {
    /// Idle threshold (0-100). CPU is idle when usage < (100 - threshold).
    threshold: u8,
    /// System information collector.
    system: System,
}

impl CpuMonitor {
    /// Creates a new CPU monitor with the given idle threshold.
    ///
    /// # Panics
    ///
    /// Panics if threshold is not in the range 0-100.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Idle threshold (0-100). CPU is idle when usage < (100 - threshold).
    ///
    /// # Examples
    ///
    /// ```
    /// use zeroclaw::monitoring::CpuMonitor;
    ///
    /// let monitor = CpuMonitor::new(80); // CPU is idle when usage < 20%
    /// ```
    #[must_use]
    pub fn new(threshold: u8) -> Self {
        assert!(threshold <= 100, "threshold must be between 0 and 100");
        Self {
            threshold,
            system: System::new(),
        }
    }

    /// Returns `true` if CPU usage is below the idle threshold.
    ///
    /// Refreshes system info and checks if global CPU usage is less than `(100 - threshold)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use zeroclaw::monitoring::CpuMonitor;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let monitor = CpuMonitor::new(80); // CPU is idle when usage < 20%
    /// let idle = monitor.is_idle_enough();
    /// # }
    /// ```
    #[must_use]
    pub fn is_idle_enough(&mut self) -> bool {
        self.system.refresh_cpu_usage();
        let usage = self.system.global_cpu_usage();
        usage < (100 - self.threshold) as f32
    }

    /// Waits until CPU is idle enough or the maximum duration is reached.
    ///
    /// Checks CPU usage every 5 minutes. Returns `true` if CPU became idle enough,
    /// `false` if the maximum duration was reached.
    ///
    /// # Arguments
    ///
    /// * `max_duration` - Maximum time to wait for CPU to become idle.
    ///
    /// # Returns
    ///
    /// * `bool` - `true` if CPU became idle enough, `false` if timeout reached.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use zeroclaw::monitoring::CpuMonitor;
    /// use tokio::time::Duration;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut monitor = CpuMonitor::new(80);
    /// let became_idle = monitor.wait_until_idle(Duration::from_secs(3600)).await;
    /// # }
    /// ```
    pub async fn wait_until_idle(&mut self, max_duration: Duration) -> bool {
        let start = tokio::time::Instant::now();
        let check_interval = Duration::from_secs(300); // 5 minutes

        loop {
            if self.is_idle_enough() {
                return true;
            }

            let elapsed = start.elapsed();
            if elapsed >= max_duration {
                return false;
            }

            let remaining = max_duration.saturating_sub(elapsed);
            let sleep_duration = std::cmp::min(check_interval, remaining);
            sleep(sleep_duration).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation() {
        let monitor = CpuMonitor::new(80);
        assert_eq!(monitor.threshold, 80);
    }

    #[test]
    #[should_panic(expected = "threshold must be between 0 and 100")]
    fn test_invalid_threshold_panics() {
        CpuMonitor::new(101);
    }

    #[test]
    fn test_zero_threshold_valid() {
        let monitor = CpuMonitor::new(0);
        assert_eq!(monitor.threshold, 0);
    }

    #[test]
    fn test_is_idle_enough() {
        let mut monitor = CpuMonitor::new(100); // Always idle (usage < 0% is impossible, but this tests the logic)
        // With threshold 100, CPU is idle when usage < 0%, which is never true
        // So this should return false
        assert!(!monitor.is_idle_enough());
    }

    #[tokio::test]
    async fn test_wait_until_idle_immediate_return() {
        let mut monitor = CpuMonitor::new(100); // Always idle (usage < 0%)
        // With threshold 100, CPU is never idle (usage < 0% is impossible)
        // So this should wait the full duration and return false
        let became_idle = monitor.wait_until_idle(Duration::from_millis(100)).await;
        assert!(!became_idle);
    }
}
