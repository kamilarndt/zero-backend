use tokio_util::sync::CancellationToken;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};

use super::workspace::HandWorkspace;
use super::interruption::InterruptionHandler;

/// Represents the state of an active hand with its cancellation token,
/// process group ID, and workspace path.
pub struct HandState {
    /// Token for graceful cancellation of the hand's work
    pub token: CancellationToken,
    /// Process group ID (pgid) for the hand's process group
    pub pgid: Option<u32>,
    /// Workspace path where the hand is operating
    pub workspace_path: Option<PathBuf>,
}

impl HandState {
    /// Create a new HandState with a cancellation token
    pub fn new(token: CancellationToken) -> Self {
        Self {
            token,
            pgid: None,
            workspace_path: None,
        }
    }
}

/// Manages background worker tasks ("hands") with concurrency limits
/// and graceful interruption capabilities.
pub struct HandsDispatcher {
    /// Base directory for hand workspaces
    pub base_dir: String,
    /// Limits concurrent hand executions to prevent resource exhaustion
    pub hands_semaphore: Arc<Semaphore>,
    /// Tracks active hands with their state for graceful interruption
    pub active_hands: Arc<RwLock<HashMap<String, HandState>>>,
}

impl HandsDispatcher {
    /// Create a new HandsDispatcher with specified concurrency limit
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            base_dir: "/tmp/zeroclaw".to_string(),
            hands_semaphore: Arc::new(Semaphore::new(max_concurrent)),
            active_hands: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for HandsDispatcher {
    /// Create with default limit of 10 concurrent hands
    fn default() -> Self {
        Self::new(10)
    }
}

impl HandsDispatcher {
    /// Register a new hand and acquire semaphore permit
    pub async fn register_hand(&self, hand_id: String) -> Result<PathBuf, anyhow::Error> {
        // Create workspace for this hand
        let workspace = HandWorkspace::create(&self.base_dir, &hand_id)?;

        let token = CancellationToken::new();
        let mut hand_state = HandState::new(token.clone());
        hand_state.workspace_path = Some(workspace.path.clone());

        {
            let mut hands = self.active_hands.write().await;
            hands.insert(hand_id.clone(), hand_state);
        }

        Ok(workspace.path)
    }

    /// Unregister a completed hand and release semaphore permit
    pub async fn unregister_hand(&self, hand_id: &str) -> Result<(), anyhow::Error> {
        // Remove hand state and get workspace path for cleanup
        let workspace_path = {
            let mut hands = self.active_hands.write().await;
            hands.remove(hand_id).and_then(|state| state.workspace_path)
        };

        // Cleanup workspace directory
        if let Some(path) = workspace_path {
            if path.exists() {
                std::fs::remove_dir_all(&path)?;
            }
        }

        Ok(())
    }

    /// Interrupt a specific hand by its ID
    pub async fn interrupt_hand(&self, hand_id: &str) -> Result<(), anyhow::Error> {
        let hands = self.active_hands.read().await;
        if let Some(hand_state) = hands.get(hand_id) {
            hand_state.token.cancel();
        }
        Ok(())
    }

    /// Interrupt a specific hand by killing its process group and cancelling its token.
    ///
    /// This method first attempts to kill the entire process group using the PGID,
    /// then cancels the token for cooperative cancellation. This ensures that even
    /// if cooperative cancellation fails, the processes will be terminated.
    ///
    /// # Arguments
    /// * `hand_id` - The ID of the hand to interrupt
    ///
    /// # Returns
    /// * `Ok(())` if the hand was interrupted successfully or if the hand doesn't exist
    /// * `Err(anyhow::Error)` if killing the process group fails
    pub async fn interrupt_hand_killed(&self, hand_id: &str) -> Result<(), anyhow::Error> {
        // First, get the hand state to check for pgid
        let (pgid, token) = {
            let hands = self.active_hands.read().await;
            if let Some(hand_state) = hands.get(hand_id) {
                (hand_state.pgid, hand_state.token.clone())
            } else {
                // Hand doesn't exist, treat as success
                return Ok(());
            }
        };

        // Kill the process group if we have a pgid
        if let Some(pgid) = pgid {
            InterruptionHandler::kill_process_group(pgid)?;
        }

        // Then cancel the token for cooperative cancellation
        token.cancel();

        Ok(())
    }

    /// Get count of currently active hands
    pub async fn active_count(&self) -> usize {
        let hands = self.active_hands.read().await;
        hands.len()
    }

    /// Check if a specific hand is currently active
    pub async fn is_active(&self, hand_id: &str) -> bool {
        let hands = self.active_hands.read().await;
        hands.contains_key(hand_id)
    }

    /// Set the process group ID (pgid) for a specific hand
    pub async fn set_hand_pgid(&self, hand_id: &str, pgid: u32) -> Result<(), anyhow::Error> {
        let mut hands = self.active_hands.write().await;
        if let Some(hand_state) = hands.get_mut(hand_id) {
            hand_state.pgid = Some(pgid);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Hand {} not found", hand_id))
        }
    }

    /// Set the workspace path for a specific hand
    pub async fn set_hand_workspace(&self, hand_id: &str, workspace_path: PathBuf) -> Result<(), anyhow::Error> {
        let mut hands = self.active_hands.write().await;
        if let Some(hand_state) = hands.get_mut(hand_id) {
            hand_state.workspace_path = Some(workspace_path);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Hand {} not found", hand_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hands_dispatcher_creation() {
        let dispatcher = HandsDispatcher::new(5);
        assert_eq!(dispatcher.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_register_and_unregister_hand() {
        let dispatcher = HandsDispatcher::new(5);

        // Register a hand
        dispatcher.register_hand("hand1".to_string()).await.unwrap();
        assert_eq!(dispatcher.active_count().await, 1);
        assert!(dispatcher.is_active("hand1").await);

        // Unregister the hand
        dispatcher.unregister_hand("hand1").await.unwrap();
        assert_eq!(dispatcher.active_count().await, 0);
        assert!(!dispatcher.is_active("hand1").await);
    }

    #[tokio::test]
    async fn test_interrupt_hand() {
        let dispatcher = HandsDispatcher::new(5);

        // Register a hand with a token
        let token = CancellationToken::new();
        let hand_state = HandState::new(token.clone());
        {
            let mut hands = dispatcher.active_hands.write().await;
            hands.insert("hand1".to_string(), hand_state);
        }

        assert!(!token.is_cancelled());

        // Interrupt the hand
        dispatcher.interrupt_hand("hand1").await.unwrap();

        // Token should be cancelled
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_interrupt_hand_killed_no_pgid() {
        let dispatcher = HandsDispatcher::new(5);

        // Register a hand with a token but no pgid
        let token = CancellationToken::new();
        let hand_state = HandState::new(token.clone());
        {
            let mut hands = dispatcher.active_hands.write().await;
            hands.insert("hand1".to_string(), hand_state);
        }

        assert!(!token.is_cancelled());

        // Interrupt the hand using interrupt_hand_killed
        dispatcher.interrupt_hand_killed("hand1").await.unwrap();

        // Token should be cancelled even without pgid
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_interrupt_hand_killed_with_pgid() {
        let dispatcher = HandsDispatcher::new(5);

        // Register a hand with a token and pgid
        let token = CancellationToken::new();
        let mut hand_state = HandState::new(token.clone());
        hand_state.pgid = Some(99999); // Non-existent pgid
        {
            let mut hands = dispatcher.active_hands.write().await;
            hands.insert("hand1".to_string(), hand_state);
        }

        assert!(!token.is_cancelled());

        // Interrupt the hand using interrupt_hand_killed
        let result = dispatcher.interrupt_hand_killed("hand1").await;
        assert!(result.is_ok(), "interrupt_hand_killed should handle non-existent pgid gracefully");

        // Token should be cancelled
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_interrupt_hand_killed_nonexistent_hand() {
        let dispatcher = HandsDispatcher::new(5);

        // Try to interrupt a non-existent hand
        let result = dispatcher.interrupt_hand_killed("nonexistent").await;
        assert!(result.is_ok(), "interrupt_hand_killed should return Ok for non-existent hand");
    }

    #[tokio::test]
    async fn test_set_hand_pgid() {
        let dispatcher = HandsDispatcher::new(5);

        // Register a hand
        dispatcher.register_hand("hand1".to_string()).await.unwrap();

        // Set pgid
        dispatcher.set_hand_pgid("hand1", 12345).await.unwrap();

        // Verify pgid was set
        let hands = dispatcher.active_hands.read().await;
        let hand_state = hands.get("hand1").unwrap();
        assert_eq!(hand_state.pgid, Some(12345));
    }

    #[tokio::test]
    async fn test_set_hand_workspace() {
        let dispatcher = HandsDispatcher::new(5);

        // Register a hand
        dispatcher.register_hand("hand1".to_string()).await.unwrap();

        // Set workspace
        let workspace_path = std::path::PathBuf::from("/tmp/workspace");
        dispatcher.set_hand_workspace("hand1", workspace_path.clone()).await.unwrap();

        // Verify workspace was set
        let hands = dispatcher.active_hands.read().await;
        let hand_state = hands.get("hand1").unwrap();
        assert_eq!(hand_state.workspace_path, Some(workspace_path));
    }

    #[tokio::test]
    async fn test_set_hand_pgid_not_found() {
        let dispatcher = HandsDispatcher::new(5);

        // Try to set pgid on non-existent hand
        let result = dispatcher.set_hand_pgid("hand1", 12345).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_hand_state_new() {
        let token = CancellationToken::new();
        let hand_state = HandState::new(token.clone());

        assert!(!hand_state.token.is_cancelled());
        assert!(hand_state.pgid.is_none());
        assert!(hand_state.workspace_path.is_none());
    }

    #[tokio::test]
    async fn test_default_dispatcher() {
        let dispatcher = HandsDispatcher::default();
        assert_eq!(dispatcher.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_workspace_lifecycle() {
        use tempfile::TempDir;

        // Create temporary base directory for testing
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().to_str().unwrap();

        let dispatcher = HandsDispatcher {
            base_dir: base_dir.to_string(),
            hands_semaphore: Arc::new(Semaphore::new(5)),
            active_hands: Arc::new(RwLock::new(HashMap::new())),
        };

        let hand_id = "test-hand-workspace";

        // Register hand - workspace should be created
        let workspace_path = dispatcher.register_hand(hand_id.to_string()).await.unwrap();

        // Verify workspace was created
        assert!(workspace_path.exists());
        assert!(workspace_path.is_dir());

        // Verify workspace path is stored in hand state
        let hands = dispatcher.active_hands.read().await;
        let hand_state = hands.get(hand_id).unwrap();
        assert_eq!(hand_state.workspace_path, Some(workspace_path.clone()));
        drop(hands);

        // Unregister hand - workspace should be cleaned up
        dispatcher.unregister_hand(hand_id).await.unwrap();

        // Verify workspace was removed
        assert!(!workspace_path.exists());

        // Verify hand state was removed
        assert!(!dispatcher.is_active(hand_id).await);
    }
}
