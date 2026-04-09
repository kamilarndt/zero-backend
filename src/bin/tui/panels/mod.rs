//! TUI Panel Components
//!
//! Individual panel rendering modules for the 5-panel dashboard layout.
//! Each panel subscribes to relevant state channels and renders efficiently.

mod chat;
mod cost;
mod logs;
mod memory;
mod swarm;

pub use cost::{render_cost_panel, CostPanelState};
pub use logs::{render_logs_panel, LogsPanelState};
pub use memory::{render_memory_panel, MemoryPanelState};
pub use swarm::{render_swarm_panel, SwarmPanelState};

// Re-export types from state subsystems for convenience
