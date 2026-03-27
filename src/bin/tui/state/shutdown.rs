//! Shutdown coordination for background tasks
//!
//! Provides graceful shutdown mechanism for async tasks
//! using tokio broadcast channels.

use tokio::sync::broadcast;

/// Shutdown coordination for background tasks
///
/// Allows main thread to signal all background tasks to shut down gracefully.
///
/// # Example
/// ```rust
/// use crate::state::shutdown::ShutdownCoordinator;
///
/// let shutdown = ShutdownCoordinator::new();
/// let mut rx = shutdown.subscribe();
///
/// // In background task
/// tokio::spawn(async move {
///     loop {
///         tokio::select! {
///             _ = interval.tick() => { /* work */ }
///             _ = rx.recv() => {
///                 println!("Shutting down");
///                 return Ok(());
///             }
///         }
///     }
/// });
///
/// // In main thread
/// shutdown.shutdown().await;
/// ```
#[derive(Clone)]
pub struct ShutdownCoordinator {
    tx: broadcast::Sender<()>,
}

impl ShutdownCoordinator {
    /// Create new shutdown coordinator
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1);
        Self { tx }
    }

    /// Subscribe to shutdown signal
    ///
    /// Each task should call this to get its own receiver
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }

    /// Signal all tasks to shut down
    pub async fn shutdown(&self) {
        let _ = self.tx.send(());
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[test]
    fn test_shutdown_coordinator_creates() {
        let shutdown = ShutdownCoordinator::new();
        let _rx = shutdown.subscribe();
        // Just verify it works
    }

    #[tokio::test]
    async fn test_shutdown_signal_receives() {
        let shutdown = ShutdownCoordinator::new();
        let mut rx = shutdown.subscribe();

        // Signal shutdown
        shutdown.shutdown().await;

        // Should receive signal
        let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_subscribers_all_receive() {
        let shutdown = ShutdownCoordinator::new();
        let mut rx1 = shutdown.subscribe();
        let mut rx2 = shutdown.subscribe();
        let mut rx3 = shutdown.subscribe();

        // Signal shutdown
        shutdown.shutdown().await;

        // All should receive
        let r1 = tokio::time::timeout(Duration::from_millis(100), rx1.recv()).await;
        let r2 = tokio::time::timeout(Duration::from_millis(100), rx2.recv()).await;
        let r3 = tokio::time::timeout(Duration::from_millis(100), rx3.recv()).await;

        assert!(r1.is_ok());
        assert!(r2.is_ok());
        assert!(r3.is_ok());
    }

    #[tokio::test]
    async fn test_task_responds_to_shutdown() {
        let shutdown = ShutdownCoordinator::new();
        let mut rx = shutdown.subscribe();

        // Spawn task that shuts down on signal
        let handle = tokio::spawn(async move {
            let mut count = 0;
            loop {
                tokio::select! {
                    _ = sleep(Duration::from_millis(10)) => {
                        count += 1;
                        if count >= 5 {
                            break;
                        }
                    }
                    _ = rx.recv() => {
                        return "shut_down";
                    }
                }
            }
            "completed"
        });

        // Send shutdown signal immediately
        shutdown.shutdown().await;

        let result = handle.await.unwrap();
        assert_eq!(result, "shut_down");
    }
}