use tokio_util::sync::CancellationToken;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct InterruptionHandler {
    pub active_hands: Arc<RwLock<HashMap<String, CancellationToken>>>,
}

impl InterruptionHandler {
    /// Kill a process group using the process group ID (PGID).
    ///
    /// This uses libc::kill with a negative PGID to kill all processes in the group.
    /// If the process group has already exited (ESRCH error), this is treated as success.
    ///
    /// # Arguments
    /// * `pgid` - Process group ID to kill
    ///
    /// # Returns
    /// * `Ok(())` if the process group was killed or already gone
    /// * `Err(anyhow::Error)` if killing failed for a reason other than ESRCH
    pub fn kill_process_group(pgid: u32) -> Result<(), anyhow::Error> {
        // Use negative PGID to kill entire process group
        let ret = unsafe { libc::kill(-(pgid as i32), libc::SIGKILL) };

        if ret == 0 {
            // Successfully killed the process group
            return Ok(());
        }

        let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);

        if errno == libc::ESRCH {
            // Process group doesn't exist - already exited, treat as success
            return Ok(());
        }

        // Other errors are actual failures
        Err(anyhow::anyhow!(
            "Failed to kill process group {}: errno {}",
            pgid,
            errno
        ))
    }

    pub fn new() -> Self {
        Self {
            active_hands: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn interrupt(&self, hand_id: &str) -> Result<(), anyhow::Error> {
        let mut hands = self.active_hands.write().await;
        if let Some(token) = hands.remove(hand_id) {
            token.cancel();
        }
        Ok(())
    }
}

impl Default for InterruptionHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_interruption_handler_creation() {
        let handler = InterruptionHandler::new();
        let hands = handler.active_hands.read().await;
        assert!(hands.is_empty());
    }

    #[tokio::test]
    async fn test_interrupt_nonexistent_hand() {
        let handler = InterruptionHandler::new();
        let result = handler.interrupt("nonexistent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_interrupt_active_hand() {
        let handler = InterruptionHandler::new();
        let token = CancellationToken::new();

        // Register a hand
        {
            let mut hands = handler.active_hands.write().await;
            hands.insert("test_hand".to_string(), token.clone());
        }

        // Verify it's not cancelled yet
        assert!(!token.is_cancelled());

        // Interrupt it
        handler.interrupt("test_hand").await.unwrap();

        // Verify it's cancelled
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_default_impl() {
        let handler = InterruptionHandler::default();
        let hands = handler.active_hands.read().await;
        assert!(hands.is_empty());
    }

    #[test]
    fn test_kill_process_group_esrch_ok() {
        // Try to kill a non-existent process group
        // This should return Ok() because ESRCH is treated as success
        let result = InterruptionHandler::kill_process_group(99999);
        assert!(result.is_ok(), "kill_process_group should handle ESRCH gracefully");
    }
}
