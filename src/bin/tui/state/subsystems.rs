//! Subsystem state snapshots
//!
//! Lightweight data structures representing the current state of each
//! ZeroClaw subsystem. These are designed to be cheap to clone and
//! transmit across watch channels.

use chrono::{DateTime, Utc};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

/// Agent swarm state snapshot
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SwarmSnapshot {
    /// Active agents in the swarm
    pub active_agents: Vec<AgentInfo>,

    /// Total tasks completed this session
    pub tasks_completed: usize,

    /// Current swarm throughput (tasks/minute)
    pub throughput: f64,

    /// When this snapshot was captured
    pub timestamp: DateTime<Utc>,
}

/// Information about a single agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Agent unique ID
    pub id: String,

    /// Agent display name
    pub name: String,

    /// Agent role (planner/executor/reviewer)
    pub role: String,

    /// Model being used
    pub model: String,

    /// Current task (if any)
    pub current_task: Option<String>,

    /// Task progress 0-100
    pub progress: u8,

    /// Agent state (running/idle/done/failed)
    pub status: AgentStatus,

    /// When this agent was created
    pub created_at: DateTime<Utc>,
}

/// Agent execution status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentStatus {
    Idle,
    Running,
    Done,
    Failed,
}

impl Default for AgentStatus {
    fn default() -> Self {
        Self::Idle
    }
}

/// Cost tracking state snapshot
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostSnapshot {
    /// Total cost this session (USD)
    pub session_cost_usd: f64,

    /// Cost today (USD)
    pub daily_cost_usd: f64,

    /// Cost this month (USD)
    pub monthly_cost_usd: f64,

    /// Daily budget limit (USD)
    pub daily_limit_usd: f64,

    /// Monthly budget limit (USD)
    pub monthly_limit_usd: f64,

    /// Cost percentage used (0-100)
    pub daily_percent_used: f64,

    /// Recent cost history (last 100 data points)
    pub cost_history: Vec<CostDataPoint>,

    /// When this snapshot was captured
    pub timestamp: DateTime<Utc>,
}

/// Single cost data point for sparkline rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostDataPoint {
    /// Timestamp of this reading
    pub timestamp: DateTime<Utc>,

    /// Cumulative cost at this point (USD)
    pub cumulative_cost_usd: f64,

    /// Request cost (USD)
    pub request_cost_usd: f64,
}

/// Memory system state snapshot
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemorySnapshot {
    /// Active memory backend
    pub backend: String,

    /// Total memories stored
    pub total_memories: usize,

    /// Memory storage size in bytes
    pub storage_bytes: u64,

    /// Recent memory operations (last 50)
    pub recent_operations: Vec<MemoryOperation>,

    /// When this snapshot was captured
    pub timestamp: DateTime<Utc>,
}

/// A single memory operation (for logs display)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOperation {
    /// Operation type
    pub op_type: MemoryOpType,

    /// Memory key (if applicable)
    pub key: Option<String>,

    /// Success or failure
    pub success: bool,

    /// When this operation occurred
    pub timestamp: DateTime<Utc>,
}

/// Type of memory operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryOpType {
    Store,
    Recall,
    Search,
    Delete,
    Clear,
}

impl MemoryOpType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Store => "STORE",
            Self::Recall => "RECALL",
            Self::Search => "SEARCH",
            Self::Delete => "DELETE",
            Self::Clear => "CLEAR",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Self::Store => Color::Green,
            Self::Recall => Color::Cyan,
            Self::Search => Color::Yellow,
            Self::Delete => Color::Red,
            Self::Clear => Color::Magenta,
        }
    }
}

/// System logs state snapshot
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LogsSnapshot {
    /// Recent log lines (last 200)
    pub log_lines: Vec<LogLine>,

    /// Current log level filter
    pub log_level: String,

    /// When this snapshot was captured
    pub timestamp: DateTime<Utc>,
}

/// Log level for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
            Self::Trace => "TRACE",
        }
    }

    pub fn color(self) -> ratatui::style::Color {
        match self {
            Self::Error => ratatui::style::Color::Red,
            Self::Warn => ratatui::style::Color::Yellow,
            Self::Info => ratatui::style::Color::Cyan,
            Self::Debug => ratatui::style::Color::DarkGray,
            Self::Trace => ratatui::style::Color::Gray,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "ERROR" => Some(Self::Error),
            "WARN" | "WARNING" => Some(Self::Warn),
            "INFO" => Some(Self::Info),
            "DEBUG" => Some(Self::Debug),
            "TRACE" => Some(Self::Trace),
            _ => None,
        }
    }
}

/// A single log line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    /// Log level
    pub level: LogLevel,

    /// Log message
    pub message: String,

    /// Optional module path
    pub module: Option<String>,

    /// When this log was emitted
    pub timestamp: DateTime<Utc>,
}

// ============================================================================
// Async Update Tasks
// ============================================================================

/// Task that continuously updates swarm state from A2A subsystem
///
/// This runs in the background, polling the SubAgentManager for state
/// and broadcasting updates to TUI panels.
pub async fn swarm_update_task(
    _channels: tokio::sync::watch::Sender<SwarmSnapshot>,
) -> anyhow::Result<()> {
    // TODO: Integrate with src/agent/a2a.rs SubAgentManager
    // For now, just tick periodically
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    loop {
        interval.tick().await;

        // In production, this would:
        // 1. Query SubAgentManager for active agents
        // 2. Get task progress from A2A message queue
        // 3. Calculate throughput metrics
        // 4. Broadcast snapshot via channels.send()
    }
}

/// Task that continuously updates cost state from CostTracker
///
/// Polls the cost tracker for current spending and broadcasts updates.
pub async fn cost_update_task(
    _channels: tokio::sync::watch::Sender<CostSnapshot>,
) -> anyhow::Result<()> {
    // TODO: Integrate with src/cost/tracker.rs
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        interval.tick().await;

        // In production, this would:
        // 1. Query CostTracker for current session costs
        // 2. Get daily/monthly aggregates
        // 3. Build cost history sparkline
        // 4. Broadcast snapshot via channels.send()
    }
}

/// Task that continuously updates memory state from Memory backend
///
/// Monitors memory operations and storage usage.
pub async fn memory_update_task(
    _channels: tokio::sync::watch::Sender<MemorySnapshot>,
) -> anyhow::Result<()> {
    // TODO: Integrate with src/memory/mod.rs
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
    loop {
        interval.tick().await;

        // In production, this would:
        // 1. Query Memory backend for total count
        // 2. Get storage size from filesystem
        // 3. Capture recent operations from operation log
        // 4. Broadcast snapshot via channels.send()
    }
}

/// Task that continuously tails system logs for TUI display
///
/// Subscribes to tracing events and filters for TUI consumption.
pub async fn logs_update_task(
    _channels: tokio::sync::watch::Sender<LogsSnapshot>,
) -> anyhow::Result<()> {
    // TODO: Integrate with tracing-subscriber
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
    loop {
        interval.tick().await;

        // In production, this would:
        // 1. Subscribe to tracing events via tracing-appender
        // 2. Filter by log level
        // 3. Maintain rolling buffer of last 200 lines
        // 4. Broadcast snapshot via channels.send()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swarm_snapshot_default() {
        let snapshot = SwarmSnapshot::default();
        assert_eq!(snapshot.active_agents.len(), 0);
        assert_eq!(snapshot.tasks_completed, 0);
    }

    #[test]
    fn test_agent_info_creation() {
        let agent = AgentInfo {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Test Agent".to_string(),
            role: "executor".to_string(),
            model: "gpt-4".to_string(),
            current_task: Some("Test task".to_string()),
            progress: 50,
            status: AgentStatus::Running,
            created_at: Utc::now(),
        };

        assert_eq!(agent.progress, 50);
        assert_eq!(agent.status, AgentStatus::Running);
        assert!(agent.current_task.is_some());
    }

    #[test]
    fn test_cost_snapshot_default() {
        let snapshot = CostSnapshot::default();
        assert_eq!(snapshot.session_cost_usd, 0.0);
        assert_eq!(snapshot.cost_history.len(), 0);
    }

    #[test]
    fn test_memory_operation_types() {
        let store_op = MemoryOpType::Store;
        let recall_op = MemoryOpType::Recall;
        assert_ne!(store_op, recall_op);
    }
}
