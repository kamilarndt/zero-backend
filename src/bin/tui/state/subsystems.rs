//! Subsystem state snapshots
//!
//! Lightweight data structures representing the current state of each
//! ZeroClaw subsystem. These are designed to be cheap to clone and
//! transmit across watch channels.

use crate::state::cache::RequestCache;
use crate::state::http::get_http_client;
use chrono::{DateTime, Utc};
use ratatui::style::Color;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

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

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
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
    channels: tokio::sync::watch::Sender<SwarmSnapshot>,
    cache: Arc<RequestCache>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    let client = get_http_client();
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    interval.tick().await; // Skip first immediate tick

    loop {
        tokio::select! {
            _ = interval.tick() => {
                const CACHE_KEY: &str = "swarm";

                // Check cache for deduplication
                if let Some(cached_json) = cache.try_get_or_mark_pending(CACHE_KEY) {
                    if let Ok(snapshot) = serde_json::from_value(cached_json) {
                        let _ = channels.send(snapshot);
                        continue;
                    }
                }

                // Fetch from API
                match client
                    .get("http://127.0.0.1:42617/api/agents/active")
                    .send()
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            if let Ok(agents) = response.json::<Vec<AgentInfo>>().await {
                                let snapshot = SwarmSnapshot {
                                    active_agents: agents,
                                    tasks_completed: 0,
                                    throughput: 0.0,
                                    timestamp: chrono::Utc::now(),
                                };

                                if let Ok(json) = serde_json::to_value(&snapshot) {
                                    cache.put_and_ready(CACHE_KEY, json);
                                }

                                let _ = channels.send(snapshot);
                            } else {
                                cache.mark_failed(CACHE_KEY);
                            }
                        } else {
                            tracing::debug!("API returned {}", response.status());
                            cache.mark_failed(CACHE_KEY);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch swarm state: {}", e);
                        cache.mark_failed(CACHE_KEY);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                tracing::info!("Swarm update task shutting down");
                return Ok(());
            }
        }
    }
}

/// Task that continuously updates cost state from CostTracker
///
/// Polls the cost tracker for current spending and broadcasts updates.
pub async fn cost_update_task(
    channels: tokio::sync::watch::Sender<CostSnapshot>,
    cache: Arc<RequestCache>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    let client = get_http_client();
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    interval.tick().await; // Skip first immediate tick

    loop {
        tokio::select! {
            _ = interval.tick() => {
                const CACHE_KEY: &str = "cost";

                // Check cache for deduplication
                if let Some(cached_json) = cache.try_get_or_mark_pending(CACHE_KEY) {
                    if let Ok(snapshot) = serde_json::from_value(cached_json) {
                        let _ = channels.send(snapshot);
                        continue;
                    }
                }

                // Fetch from API
                match client.get("http://127.0.0.1:42617/api/cost").send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            if let Ok(cost_response) = response.json::<serde_json::Value>().await {
                                if let Some(cost_data) = cost_response.get("cost") {
                                    let session_cost = cost_data
                                        .get("session_cost_usd")
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0);
                                    let daily_cost = cost_data
                                        .get("daily_cost_usd")
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0);
                                    let monthly_cost = cost_data
                                        .get("monthly_cost_usd")
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0);

                                    let daily_limit = 10.0;
                                    let monthly_limit = 100.0;

                                    let now = chrono::Utc::now();
                                    let cost_history = vec![CostDataPoint {
                                        timestamp: now,
                                        cumulative_cost_usd: daily_cost,
                                        request_cost_usd: 0.01,
                                    }];

                                    let daily_percent_used = if daily_limit > 0.0 {
                                        (daily_cost / daily_limit * 100.0).min(100.0)
                                    } else {
                                        0.0
                                    };

                                    let snapshot = CostSnapshot {
                                        session_cost_usd: session_cost,
                                        daily_cost_usd: daily_cost,
                                        monthly_cost_usd: monthly_cost,
                                        daily_limit_usd: daily_limit,
                                        monthly_limit_usd: monthly_limit,
                                        daily_percent_used,
                                        cost_history,
                                        timestamp: now,
                                    };

                                    if let Ok(json) = serde_json::to_value(&snapshot) {
                                        cache.put_and_ready(CACHE_KEY, json);
                                    }

                                    let _ = channels.send(snapshot);
                                }
                            } else {
                                cache.mark_failed(CACHE_KEY);
                            }
                        } else {
                            cache.mark_failed(CACHE_KEY);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch cost state: {}", e);
                        cache.mark_failed(CACHE_KEY);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                tracing::info!("Cost update task shutting down");
                return Ok(());
            }
        }
    }
}

/// Task that continuously updates memory state from Memory backend
///
/// Monitors memory operations and storage usage.
pub async fn memory_update_task(
    channels: tokio::sync::watch::Sender<MemorySnapshot>,
    cache: Arc<RequestCache>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    let client = get_http_client();
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    interval.tick().await; // Skip first immediate tick

    loop {
        tokio::select! {
            _ = interval.tick() => {
                const CACHE_KEY: &str = "memory";

                // Check cache for deduplication
                if let Some(cached_json) = cache.try_get_or_mark_pending(CACHE_KEY) {
                    if let Ok(snapshot) = serde_json::from_value(cached_json) {
                        let _ = channels.send(snapshot);
                        continue;
                    }
                }

                // Fetch from API
                match client
                    .get("http://127.0.0.1:42617/api/memory/status")
                    .send()
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            if let Ok(memory_response) = response.json::<serde_json::Value>().await {
                                let backend = memory_response
                                    .get("backend")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                let total_memories = memory_response
                                    .get("total_memories")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0) as usize;
                                let storage_bytes = memory_response
                                    .get("storage_bytes")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0);

                                let recent_operations = memory_response
                                    .get("recent_operations")
                                    .and_then(|v| v.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|op| serde_json::from_value(op.clone()).ok())
                                            .collect()
                                    })
                                    .unwrap_or_else(Vec::new);

                                let snapshot = MemorySnapshot {
                                    backend,
                                    total_memories,
                                    storage_bytes,
                                    recent_operations,
                                    timestamp: chrono::Utc::now(),
                                };

                                if let Ok(json) = serde_json::to_value(&snapshot) {
                                    cache.put_and_ready(CACHE_KEY, json);
                                }

                                let _ = channels.send(snapshot);
                            } else {
                                cache.mark_failed(CACHE_KEY);
                            }
                        } else {
                            cache.mark_failed(CACHE_KEY);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch memory state: {}", e);
                        cache.mark_failed(CACHE_KEY);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                tracing::info!("Memory update task shutting down");
                return Ok(());
            }
        }
    }
}

/// Task that continuously tails system logs for TUI display
///
/// Subscribes to tracing events and filters for TUI consumption.
pub async fn logs_update_task(
    channels: tokio::sync::watch::Sender<LogsSnapshot>,
    cache: Arc<RequestCache>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    let client = get_http_client();
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    interval.tick().await; // Skip first immediate tick

    loop {
        tokio::select! {
            _ = interval.tick() => {
                const CACHE_KEY: &str = "logs";

                // Check cache for deduplication
                if let Some(cached_json) = cache.try_get_or_mark_pending(CACHE_KEY) {
                    if let Ok(snapshot) = serde_json::from_value(cached_json) {
                        let _ = channels.send(snapshot);
                        continue;
                    }
                }

                // Fetch from API
                match client
                    .get("http://127.0.0.1:42617/api/logs/status")
                    .send()
                    .await
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            if let Ok(logs_response) = response.json::<serde_json::Value>().await {
                                let log_lines = logs_response
                                    .get("log_lines")
                                    .and_then(|v| v.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|line| serde_json::from_value(line.clone()).ok())
                                            .collect()
                                    })
                                    .unwrap_or_else(Vec::new);

                                let snapshot = LogsSnapshot {
                                    log_lines,
                                    log_level: "INFO".to_string(),
                                    timestamp: chrono::Utc::now(),
                                };

                                if let Ok(json) = serde_json::to_value(&snapshot) {
                                    cache.put_and_ready(CACHE_KEY, json);
                                }

                                let _ = channels.send(snapshot);
                            } else {
                                cache.mark_failed(CACHE_KEY);
                            }
                        } else {
                            cache.mark_failed(CACHE_KEY);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch logs state: {}", e);
                        cache.mark_failed(CACHE_KEY);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                tracing::info!("Logs update task shutting down");
                return Ok(());
            }
        }
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
