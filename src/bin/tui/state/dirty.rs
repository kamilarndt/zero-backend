//! Dirty tracking system for conditional UI rendering
//!
//! Tracks which UI panels need redrawing using atomic flags.
//! This prevents redundant terminal.draw() calls when nothing has changed.

use std::sync::atomic::{AtomicBool, Ordering};

/// Tracks which UI panels need redrawing
#[derive(Debug)]
pub struct DirtyFlags {
    swarm: AtomicBool,
    cost: AtomicBool,
    memory: AtomicBool,
    logs: AtomicBool,
    any_dirty: AtomicBool,
}

impl DirtyFlags {
    /// Create new dirty flags (all clean)
    pub fn new() -> Self {
        Self {
            swarm: AtomicBool::new(false),
            cost: AtomicBool::new(false),
            memory: AtomicBool::new(false),
            logs: AtomicBool::new(false),
            any_dirty: AtomicBool::new(false),
        }
    }

    /// Mark swarm panel as dirty
    pub fn mark_swarm(&self) {
        self.swarm.store(true, Ordering::Relaxed);
        self.any_dirty.store(true, Ordering::Relaxed);
    }

    /// Mark cost panel as dirty
    pub fn mark_cost(&self) {
        self.cost.store(true, Ordering::Relaxed);
        self.any_dirty.store(true, Ordering::Relaxed);
    }

    /// Mark memory panel as dirty
    pub fn mark_memory(&self) {
        self.memory.store(true, Ordering::Relaxed);
        self.any_dirty.store(true, Ordering::Relaxed);
    }

    /// Mark logs panel as dirty
    pub fn mark_logs(&self) {
        self.logs.store(true, Ordering::Relaxed);
        self.any_dirty.store(true, Ordering::Relaxed);
    }

    /// Mark all panels as dirty
    pub fn mark_all(&self) {
        self.mark_swarm();
        self.mark_cost();
        self.mark_memory();
        self.mark_logs();
    }

    /// Clear swarm dirty flag
    pub fn clear_swarm(&self) {
        self.swarm.store(false, Ordering::Relaxed);
    }

    /// Clear cost dirty flag
    pub fn clear_cost(&self) {
        self.cost.store(false, Ordering::Relaxed);
    }

    /// Clear memory dirty flag
    pub fn clear_memory(&self) {
        self.memory.store(false, Ordering::Relaxed);
    }

    /// Clear logs dirty flag
    pub fn clear_logs(&self) {
        self.logs.store(false, Ordering::Relaxed);
    }

    /// Clear all dirty flags
    pub fn clear_all(&self) {
        self.swarm.store(false, Ordering::Relaxed);
        self.cost.store(false, Ordering::Relaxed);
        self.memory.store(false, Ordering::Relaxed);
        self.logs.store(false, Ordering::Relaxed);
        self.any_dirty.store(false, Ordering::Relaxed);
    }

    /// Check if swarm panel is dirty
    pub fn is_swarm_dirty(&self) -> bool {
        self.swarm.load(Ordering::Relaxed)
    }

    /// Check if cost panel is dirty
    pub fn is_cost_dirty(&self) -> bool {
        self.cost.load(Ordering::Relaxed)
    }

    /// Check if memory panel is dirty
    pub fn is_memory_dirty(&self) -> bool {
        self.memory.load(Ordering::Relaxed)
    }

    /// Check if logs panel is dirty
    pub fn is_logs_dirty(&self) -> bool {
        self.logs.load(Ordering::Relaxed)
    }

    /// Check if any panel is dirty (fast path for conditional rendering)
    pub fn is_any_dirty(&self) -> bool {
        self.any_dirty.load(Ordering::Relaxed)
    }
}

impl Default for DirtyFlags {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_flags_initial_state() {
        let dirty = DirtyFlags::new();
        assert!(!dirty.is_any_dirty());
        assert!(!dirty.is_swarm_dirty());
        assert!(!dirty.is_cost_dirty());
        assert!(!dirty.is_memory_dirty());
        assert!(!dirty.is_logs_dirty());
    }

    #[test]
    fn test_mark_single_panel() {
        let dirty = DirtyFlags::new();
        dirty.mark_swarm();
        assert!(dirty.is_any_dirty());
        assert!(dirty.is_swarm_dirty());
        assert!(!dirty.is_cost_dirty());
    }

    #[test]
    fn test_mark_all_panels() {
        let dirty = DirtyFlags::new();
        dirty.mark_all();
        assert!(dirty.is_swarm_dirty());
        assert!(dirty.is_cost_dirty());
        assert!(dirty.is_memory_dirty());
        assert!(dirty.is_logs_dirty());
        assert!(dirty.is_any_dirty());
    }

    #[test]
    fn test_clear_single_panel() {
        let dirty = DirtyFlags::new();
        dirty.mark_swarm();
        dirty.mark_cost();
        dirty.clear_swarm();
        assert!(!dirty.is_swarm_dirty());
        assert!(dirty.is_cost_dirty());
        assert!(dirty.is_any_dirty()); // Still dirty due to cost
    }

    #[test]
    fn test_clear_all_panels() {
        let dirty = DirtyFlags::new();
        dirty.mark_all();
        dirty.clear_all();
        assert!(!dirty.is_any_dirty());
        assert!(!dirty.is_swarm_dirty());
        assert!(!dirty.is_cost_dirty());
    }

    #[test]
    fn test_default() {
        let dirty = DirtyFlags::default();
        assert!(!dirty.is_any_dirty());
    }
}
