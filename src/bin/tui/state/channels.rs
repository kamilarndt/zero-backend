//! State broadcasting channels for TUI updates
//!
//! Uses tokio::sync::watch channels for non-blocking, multi-consumer state updates.
//! Each subsystem publishes updates to a watch channel, and panels subscribe to
//! receive automatic notifications when state changes.

use super::subsystems::{CostSnapshot, LogsSnapshot, MemorySnapshot, SwarmSnapshot};
use std::time::Instant;
use tokio::sync::watch;

/// Central state broadcaster for all TUI subsystems
///
/// This struct holds watch channel transmitters for each subsystem.
/// Created once at startup and passed to async update tasks.
pub struct TuiStateChannels {
    /// Agent swarm state transmitter
    pub swarm_tx: watch::Sender<SwarmSnapshot>,

    /// Cost tracking state transmitter
    pub cost_tx: watch::Sender<CostSnapshot>,

    /// Memory system state transmitter
    pub memory_tx: watch::Sender<MemorySnapshot>,

    /// System logs state transmitter
    pub logs_tx: watch::Sender<LogsSnapshot>,

    /// When state updates were last initiated
    last_update: Instant,
}

impl TuiStateChannels {
    /// Create new state channels with initial empty snapshots
    pub fn new() -> Self {
        let (swarm_tx, _swarm_rx) = watch::channel(SwarmSnapshot::default());
        let (cost_tx, _cost_rx) = watch::channel(CostSnapshot::default());
        let (memory_tx, _memory_rx) = watch::channel(MemorySnapshot::default());
        let (logs_tx, _logs_rx) = watch::channel(LogsSnapshot::default());

        Self {
            swarm_tx,
            cost_tx,
            memory_tx,
            logs_tx,
            last_update: Instant::now(),
        }
    }

    /// Subscribe to swarm state updates
    pub fn subscribe_swarm(&self) -> watch::Receiver<SwarmSnapshot> {
        self.swarm_tx.subscribe()
    }

    /// Subscribe to cost state updates
    pub fn subscribe_cost(&self) -> watch::Receiver<CostSnapshot> {
        self.cost_tx.subscribe()
    }

    /// Subscribe to memory state updates
    pub fn subscribe_memory(&self) -> watch::Receiver<MemorySnapshot> {
        self.memory_tx.subscribe()
    }

    /// Subscribe to logs state updates
    pub fn subscribe_logs(&self) -> watch::Receiver<LogsSnapshot> {
        self.logs_tx.subscribe()
    }

    /// Broadcast swarm state update to all subscribers
    pub fn broadcast_swarm(&self, snapshot: SwarmSnapshot) {
        // Only send if changed significantly (at least 100ms since last)
        if self.last_update.elapsed() >= std::time::Duration::from_millis(100) {
            let _ = self.swarm_tx.send(snapshot);
        }
    }

    /// Broadcast cost state update to all subscribers
    pub fn broadcast_cost(&self, snapshot: CostSnapshot) {
        let _ = self.cost_tx.send(snapshot);
    }

    /// Broadcast memory state update to all subscribers
    pub fn broadcast_memory(&self, snapshot: MemorySnapshot) {
        let _ = self.memory_tx.send(snapshot);
    }

    /// Broadcast logs state update to all subscribers
    pub fn broadcast_logs(&self, snapshot: LogsSnapshot) {
        let _ = self.logs_tx.send(snapshot);
    }

    /// Get time since last state update
    pub fn time_since_last_update(&self) -> std::time::Duration {
        self.last_update.elapsed()
    }
}

impl Default for TuiStateChannels {
    fn default() -> Self {
        Self::new()
    }
}

/// Combined snapshot of all TUI state
///
/// Used for testing and batch operations where you need
/// consistent state across all subsystems.
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub swarm: SwarmSnapshot,
    pub cost: CostSnapshot,
    pub memory: MemorySnapshot,
    pub logs: LogsSnapshot,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_channels_creation() {
        let channels = TuiStateChannels::new();
        // Subscribe to each channel
        let _swarm_rx = channels.subscribe_swarm();
        let _cost_rx = channels.subscribe_cost();
        let _memory_rx = channels.subscribe_memory();
        let _logs_rx = channels.subscribe_logs();

        // Should not panic
        assert!(channels.time_since_last_update() < std::time::Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_swarm_broadcast() {
        let channels = TuiStateChannels::new();
        let mut swarm_rx = channels.subscribe_swarm();

        // Create test snapshot
        let snapshot = SwarmSnapshot::default();

        // Broadcast
        channels.broadcast_swarm(snapshot.clone());

        // Allow time for propagation
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Receiver should have new value
        let received = swarm_rx.borrow_and_update();
        assert_eq!(received.active_agents, 0);
    }
}
