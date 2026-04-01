//! Dirty Manager for coordinating UI updates
//!
//! Manages dirty flags for different UI panels and provides a centralized
//! way to mark panels as dirty when data changes.

use std::sync::Arc;
use tokio::sync::Mutex;

/// Dirty manager for coordinating UI updates
#[derive(Clone)]
pub struct DirtyManager {
    inner: Arc<Mutex<InnerManager>>,
}

/// Inner state of the dirty manager
struct InnerManager {
    swarm_dirty: bool,
    cost_dirty: bool,
    memory_dirty: bool,
    logs_dirty: bool,
    any_dirty: bool,
}

impl DirtyManager {
    /// Create a new dirty manager
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerManager {
                swarm_dirty: false,
                cost_dirty: false,
                memory_dirty: false,
                logs_dirty: false,
                any_dirty: false,
            })),
        }
    }

    /// Mark swarm panel as dirty
    pub async fn mark_swarm(&self) {
        let mut inner = self.inner.lock().await;
        inner.swarm_dirty = true;
        inner.any_dirty = true;
    }

    /// Mark cost panel as dirty
    pub async fn mark_cost(&self) {
        let mut inner = self.inner.lock().await;
        inner.cost_dirty = true;
        inner.any_dirty = true;
    }

    /// Mark memory panel as dirty
    pub async fn mark_memory(&self) {
        let mut inner = self.inner.lock().await;
        inner.memory_dirty = true;
        inner.any_dirty = true;
    }

    /// Mark logs panel as dirty
    pub async fn mark_logs(&self) {
        let mut inner = self.inner.lock().await;
        inner.logs_dirty = true;
        inner.any_dirty = true;
    }

    /// Mark all panels as dirty
    pub async fn mark_all(&self) {
        let mut inner = self.inner.lock().await;
        inner.swarm_dirty = true;
        inner.cost_dirty = true;
        inner.memory_dirty = true;
        inner.logs_dirty = true;
        inner.any_dirty = true;
    }

    /// Check if swarm panel is dirty
    pub async fn is_swarm_dirty(&self) -> bool {
        let inner = self.inner.lock().await;
        inner.swarm_dirty
    }

    /// Check if cost panel is dirty
    pub async fn is_cost_dirty(&self) -> bool {
        let inner = self.inner.lock().await;
        inner.cost_dirty
    }

    /// Check if memory panel is dirty
    pub async fn is_memory_dirty(&self) -> bool {
        let inner = self.inner.lock().await;
        inner.memory_dirty
    }

    /// Check if logs panel is dirty
    pub async fn is_logs_dirty(&self) -> bool {
        let inner = self.inner.lock().await;
        inner.logs_dirty
    }

    /// Check if any panel is dirty
    pub async fn is_any_dirty(&self) -> bool {
        let inner = self.inner.lock().await;
        inner.any_dirty
    }

    /// Clear swarm dirty flag
    pub async fn clear_swarm(&self) {
        let mut inner = self.inner.lock().await;
        inner.swarm_dirty = false;
    }

    /// Clear cost dirty flag
    pub async fn clear_cost(&self) {
        let mut inner = self.inner.lock().await;
        inner.cost_dirty = false;
    }

    /// Clear memory dirty flag
    pub async fn clear_memory(&self) {
        let mut inner = self.inner.lock().await;
        inner.memory_dirty = false;
    }

    /// Clear logs dirty flag
    pub async fn clear_logs(&self) {
        let mut inner = self.inner.lock().await;
        inner.logs_dirty = false;
    }

    /// Clear all dirty flags
    pub async fn clear_all(&self) {
        let mut inner = self.inner.lock().await;
        inner.swarm_dirty = false;
        inner.cost_dirty = false;
        inner.memory_dirty = false;
        inner.logs_dirty = false;
        inner.any_dirty = false;
    }
}

impl Default for DirtyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[tokio::test]
    async fn test_dirty_manager_creation() {
        let manager = DirtyManager::new();
        assert!(!manager.is_any_dirty().await);
    }

    #[tokio::test]
    async fn test_mark_swarm() {
        let manager = DirtyManager::new();
        manager.mark_swarm().await;
        assert!(manager.is_swarm_dirty().await);
        assert!(manager.is_any_dirty().await);
    }

    #[tokio::test]
    async fn test_mark_all() {
        let manager = DirtyManager::new();
        manager.mark_all().await;
        assert!(manager.is_swarm_dirty().await);
        assert!(manager.is_cost_dirty().await);
        assert!(manager.is_memory_dirty().await);
        assert!(manager.is_logs_dirty().await);
        assert!(manager.is_any_dirty().await);
    }

    #[tokio::test]
    async fn test_clear_panel() {
        let manager = DirtyManager::new();
        manager.mark_all().await;
        manager.clear_swarm().await;
        assert!(!manager.is_swarm_dirty().await);
        assert!(manager.is_any_dirty().await); // Still dirty due to other panels
    }

    #[tokio::test]
    async fn test_clear_all() {
        let manager = DirtyManager::new();
        manager.mark_all().await;
        manager.clear_all().await;
        assert!(!manager.is_any_dirty().await);
    }
}
