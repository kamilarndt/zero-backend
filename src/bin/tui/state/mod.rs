//! TUI State Management Layer
//!
//! This module provides thread-safe state channels for broadcasting core subsystem state
//! to TUI panels without blocking the main agent loop. Uses tokio::sync::watch for
//! non-blocking updates with automatic receiver synchronization.

pub mod channels;
pub mod subsystems;

pub use channels::{TuiStateChannels, StateSnapshot};
pub use subsystems::{
    SwarmSnapshot, CostSnapshot, MemorySnapshot, LogsSnapshot,
    swarm_update_task, cost_update_task, memory_update_task, logs_update_task
};

use std::sync::Arc;
use parking_lot::Mutex;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::state::subsystems::{AgentInfo, AgentStatus as SubAgentStatus};

/// Available agent types for routing
pub const DEFAULT_AGENTS: &[&str] = &["coder", "planner", "vision", "fast", "siyuan-master"];

/// Main application state container
///
/// Holds all panel states with thread-safe access via Arc<Mutex<T>>
#[derive(Clone)]
pub struct AppState {
    // UI state (main thread only)
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub should_quit: bool,
    pub active_session: usize,

    // Agent selection
    pub available_agents: Vec<String>,
    pub selected_agent_index: usize,

    // Panel visibility (main thread only)
    pub panels: PanelVisibility,

    // Legacy session management (integrating with existing app.rs)
    pub sessions: Vec<Session>,
    pub router_status: RouterStatus,
    pub chat_scroll: usize,
    pub active_agents: Vec<AgentStatus>,

    // Panel states (updated by async tasks, read by render thread)
    pub chat_panel: Arc<Mutex<ChatPanel>>,
    pub swarm_panel: Arc<Mutex<SwarmPanel>>,
    pub cost_panel: Arc<Mutex<CostPanel>>,
    pub memory_panel: Arc<Mutex<MemoryPanel>>,
    pub log_panel: Arc<Mutex<LogPanel>>,
}

/// Input modes for the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,    // Command mode
    Insert,    // Typing messages
    Command,   // Command mode (:q, :help, etc.)
}

/// Controls which panels are visible
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelVisibility {
    pub swarm: bool,
    pub cost: bool,
    pub memory: bool,
    pub logs: bool,
}

impl Default for PanelVisibility {
    fn default() -> Self {
        Self {
            swarm: true,
            cost: true,
            memory: true,
            logs: true,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        let mut sessions = Vec::new();
        // Create initial session
        sessions.push(Session {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Session 1".to_string(),
            messages: Vec::new(),
            created_at: Utc::now(),
        });

        Self {
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            should_quit: false,
            active_session: 0,
            available_agents: DEFAULT_AGENTS.iter().map(|s| s.to_string()).collect(),
            selected_agent_index: 0,
            panels: PanelVisibility::default(),
            sessions,
            router_status: RouterStatus::default(),
            chat_scroll: 0,
            active_agents: Vec::new(),
            chat_panel: Arc::new(Mutex::new(ChatPanel::new())),
            swarm_panel: Arc::new(Mutex::new(SwarmPanel::new())),
            cost_panel: Arc::new(Mutex::new(CostPanel::new())),
            memory_panel: Arc::new(Mutex::new(MemoryPanel::new())),
            log_panel: Arc::new(Mutex::new(LogPanel::new())),
        }
    }
}

impl AppState {
    /// Create new AppState with demo data
    pub fn new_demo() -> Self {
        let mut state = Self::default();

        // Initialize with demo data
        {
            let mut chat = state.chat_panel.lock();
            chat.add_demo_messages();
        }

        {
            let mut swarm = state.swarm_panel.lock();
            swarm.add_demo_agents();
        }

        {
            let mut cost = state.cost_panel.lock();
            cost.add_demo_costs();
        }

        {
            let mut memory = state.memory_panel.lock();
            memory.add_demo_operations();
        }

        {
            let mut log = state.log_panel.lock();
            log.add_demo_logs();
        }

        state
    }

    /// Toggle panel visibility
    pub fn toggle_panel(&mut self, panel: PanelType) {
        match panel {
            PanelType::Swarm => self.panels.swarm = !self.panels.swarm,
            PanelType::Cost => self.panels.cost = !self.panels.cost,
            PanelType::Memory => self.panels.memory = !self.panels.memory,
            PanelType::Logs => self.panels.logs = !self.panels.logs,
        }
    }

    /// Get the currently selected agent ID
    pub fn selected_agent(&self) -> String {
        self.available_agents
            .get(self.selected_agent_index)
            .cloned()
            .unwrap_or_else(|| "coder".to_string())
    }

    /// Set the selected agent by index
    pub fn set_selected_agent(&mut self, index: usize) {
        if !self.available_agents.is_empty() {
            self.selected_agent_index = index % self.available_agents.len();
        }
    }

    /// Cycle to next agent
    pub fn next_agent(&mut self) {
        if !self.available_agents.is_empty() {
            self.selected_agent_index = (self.selected_agent_index + 1) % self.available_agents.len();
        }
    }

    /// Cycle to previous agent
    pub fn prev_agent(&mut self) {
        if !self.available_agents.is_empty() {
            self.selected_agent_index = if self.selected_agent_index == 0 {
                self.available_agents.len() - 1
            } else {
                self.selected_agent_index - 1
            };
        }
    }

    /// Check if panel is visible
    pub fn is_panel_visible(&self, panel: PanelType) -> bool {
        match panel {
            PanelType::Swarm => self.panels.swarm,
            PanelType::Cost => self.panels.cost,
            PanelType::Memory => self.panels.memory,
            PanelType::Logs => self.panels.logs,
        }
    }

    /// Create a new session and switch to it
    pub fn new_session(&mut self) {
        let session = Session {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("Session {}", self.sessions.len() + 1),
            messages: Vec::new(),
            created_at: Utc::now(),
        };
        self.sessions.push(session);
        self.active_session = self.sessions.len() - 1;
        self.chat_scroll = 0;
    }

    /// Close the current session
    pub fn close_session(&mut self) {
        if self.sessions.len() > 1 {
            self.sessions.remove(self.active_session);
            if self.active_session >= self.sessions.len() {
                self.active_session = self.sessions.len() - 1;
            }
            self.chat_scroll = 0;
        } else {
            // If last session, just clear messages
            if let Some(session) = self.sessions.get_mut(self.active_session) {
                session.messages.clear();
            }
            self.chat_scroll = 0;
        }
    }

    /// Switch to next session (Tab)
    pub fn next_session(&mut self) {
        if !self.sessions.is_empty() {
            self.active_session = (self.active_session + 1) % self.sessions.len();
            self.chat_scroll = 0;
        }
    }

    /// Switch to previous session (Shift+Tab)
    pub fn prev_session(&mut self) {
        if !self.sessions.is_empty() {
            if self.active_session == 0 {
                self.active_session = self.sessions.len() - 1;
            } else {
                self.active_session -= 1;
            }
            self.chat_scroll = 0;
        }
    }

    /// Get the current session
    pub fn current_session(&self) -> Option<&Session> {
        self.sessions.get(self.active_session)
    }

    /// Get mutable reference to current session
    pub fn current_session_mut(&mut self) -> Option<&mut Session> {
        self.sessions.get_mut(self.active_session)
    }

    /// Add a user message to the current session
    pub fn add_user_message(&mut self, content: String) {
        if let Some(session) = self.current_session_mut() {
            session.messages.push(Message {
                id: uuid::Uuid::new_v4().to_string(),
                role: MessageRole::User,
                content,
                model_used: None,
                timestamp: Utc::now(),
                token_count: None,
            });
        }
    }

    /// Add an assistant message to the current session
    pub fn add_assistant_message(&mut self, content: String, model: Option<String>) {
        if let Some(session) = self.current_session_mut() {
            session.messages.push(Message {
                id: uuid::Uuid::new_v4().to_string(),
                role: MessageRole::Assistant,
                content,
                model_used: model,
                timestamp: Utc::now(),
                token_count: None,
            });
        }
    }

    /// Scroll chat up
    pub fn scroll_up(&mut self) {
        if self.chat_scroll < self.current_session().map(|s| s.messages.len()).unwrap_or(0) {
            self.chat_scroll = self.chat_scroll.saturating_add(1);
        }
    }

    /// Scroll chat down
    pub fn scroll_down(&mut self) {
        self.chat_scroll = self.chat_scroll.saturating_sub(1);
    }

    /// Execute a command from command mode
    /// Returns (output_message, should_clear_input)
    pub fn execute_command(&mut self, command: &str) -> (String, bool) {
        let cmd = command.trim().to_lowercase();
        let parts: Vec<&str> = cmd.split_whitespace().collect();

        match parts.first() {
            Some(&"q") | Some(&"quit") => {
                self.should_quit = true;
                ("Quitting...".to_string(), true)
            }
            Some(&"h") | Some(&"help") => {
                ("Showing help. Press '?' to see key bindings.".to_string(), false)
            }
            Some(&"new") => {
                self.new_session();
                ("New session created.".to_string(), true)
            }
            Some(&"clear") => {
                if let Some(session) = self.current_session_mut() {
                    session.messages.clear();
                }
                ("Session cleared.".to_string(), true)
            }
            Some(&"test") => {
                let output = run_tui_diagnostic();
                (output, true)
            }
            _ => {
                (format!("Unknown command: {}", command), false)
            }
        }
    }
}

/// Type of panel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelType {
    Swarm,
    Cost,
    Memory,
    Logs,
}

/// Chat session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session ID
    pub id: String,

    /// Session name (editable)
    pub name: String,

    /// Messages in this session
    pub messages: Vec<Message>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message ID
    pub id: String,

    /// Message role
    pub role: MessageRole,

    /// Message content
    pub content: String,

    /// Model used for generation (if applicable)
    pub model_used: Option<String>,

    /// Message timestamp
    pub timestamp: DateTime<Utc>,

    /// Token count for this message
    pub token_count: Option<usize>,
}

/// Message role (user/assistant/system)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Router status information
#[derive(Debug, Clone)]
pub struct RouterStatus {
    /// Currently active provider
    pub active_provider: String,

    /// Quota used percentage (0-100)
    pub quota_used_percent: f32,

    /// Whether fallback is active
    pub fallback_active: bool,
}

impl Default for RouterStatus {
    fn default() -> Self {
        Self {
            active_provider: "None".to_string(),
            quota_used_percent: 0.0,
            fallback_active: false,
        }
    }
}

/// Active agent status (for main app state)
#[derive(Debug, Clone)]
pub struct AgentStatus {
    /// Agent ID
    pub id: String,

    /// Agent name
    pub name: String,

    /// Model being used
    pub model: String,

    /// Progress 0-100
    pub progress: u8,

    /// Current state
    pub status: crate::state::subsystems::AgentStatus,
}

/// Agent execution state
pub type AgentState = crate::state::subsystems::AgentStatus;

/// Run TUI diagnostic test
fn run_tui_diagnostic() -> String {
    use std::time::Duration;

    // Try to connect to local gateway
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default();

    let mut results = Vec::new();
    results.push("🔍 ZeroClaw TUI Diagnostics".to_string());
    results.push("─".repeat(40));

    // Test 1: Gateway connection
    match client.get("http://127.0.0.1:42617/health").send() {
        Ok(response) => {
            if response.status().is_success() {
                results.push("✓ Gateway: CONNECTED".to_string());
            } else {
                results.push(format!("✗ Gateway: HTTP {}", response.status()));
            }
        }
        Err(e) => {
            results.push(format!("✗ Gateway: {}", e));
        }
    }

    // Test 2: Diagnostic endpoint
    match client.get("http://127.0.0.1:42617/api/diagnostic").send() {
        Ok(response) => {
            if response.status().is_success() {
                results.push("✓ Diagnostic API: WORKING".to_string());
            } else {
                results.push(format!("! Diagnostic API: HTTP {}", response.status()));
            }
        }
        Err(e) => {
            results.push(format!("✗ Diagnostic API: {}", e));
        }
    }

    results.push("─".repeat(40));
    results.push("Commands:".to_string());
    results.push("  :test    Run diagnostics".to_string());
    results.push("  :new     Create new session".to_string());
    results.push("  :clear   Clear current session".to_string());
    results.push("  :q       Quit".to_string());

    results.join("\n")
}

/// Main chat panel state
pub struct ChatPanel {
    pub messages: Vec<Message>,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
}

impl ChatPanel {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
        }
    }

    pub fn add_demo_messages(&mut self) {
        let demo_messages = vec![
            Message {
                id: "1".to_string(),
                role: MessageRole::User,
                content: "Analyze the performance data from the last 24 hours".to_string(),
                model_used: None,
                timestamp: chrono::Utc::now() - chrono::Duration::minutes(5),
                token_count: Some(150),
            },
            Message {
                id: "2".to_string(),
                role: MessageRole::Assistant,
                content: "I'll analyze the performance metrics. Looking at the data from the last 24 hours, I can see several key trends:\n\n1. API response times have improved by 15%\n2. Error rates decreased from 2.1% to 0.8%\n3. Throughput increased to 120 requests/minute\n\nWould you like me to dive deeper into any specific metric?".to_string(),
                model_used: Some("gpt-4".to_string()),
                timestamp: chrono::Utc::now() - chrono::Duration::minutes(4),
                token_count: Some(342),
            },
        ];

        self.messages = demo_messages;
    }
}

/// Swarm panel state
pub struct SwarmPanel {
    pub agents: Vec<AgentInfo>,
    pub tasks_completed: usize,
    pub throughput: f64,
}

impl SwarmPanel {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            tasks_completed: 0,
            throughput: 0.0,
        }
    }

    pub fn add_demo_agents(&mut self) {
        use crate::state::subsystems::{AgentInfo, AgentStatus as SubAgentStatus};
        self.agents = vec![
            AgentInfo {
                id: "agent-001".to_string(),
                name: "Data Analyzer".to_string(),
                role: "executor".to_string(),
                model: "gpt-4".to_string(),
                current_task: Some("Performance analysis".to_string()),
                progress: 75,
                status: SubAgentStatus::Running,
                created_at: chrono::Utc::now() - chrono::Duration::minutes(10),
            },
            AgentInfo {
                id: "agent-002".to_string(),
                name: "Code Reviewer".to_string(),
                role: "reviewer".to_string(),
                model: "claude-3".to_string(),
                current_task: None,
                progress: 0,
                status: SubAgentStatus::Idle,
                created_at: chrono::Utc::now() - chrono::Duration::minutes(15),
            },
        ];
        self.tasks_completed = 12;
        self.throughput = 3.5; // tasks per minute
    }
}

/// Cost panel state
pub struct CostPanel {
    pub session_cost: f64,
    pub daily_cost: f64,
    pub daily_limit: f64,
    pub monthly_cost: f64,
    pub monthly_limit: f64,
    pub cost_history: Vec<CostDataPoint>,
}

impl CostPanel {
    pub fn new() -> Self {
        Self {
            session_cost: 0.0,
            daily_cost: 0.0,
            daily_limit: 5.0,
            monthly_cost: 0.0,
            monthly_limit: 50.0,
            cost_history: Vec::new(),
        }
    }

    pub fn add_demo_costs(&mut self) {
        self.session_cost = 0.234;
        self.daily_cost = 1.42;
        self.monthly_cost = 12.80;

        // Generate demo cost history
        let now = chrono::Utc::now();
        self.cost_history = (0..20)
            .map(|i| CostDataPoint {
                timestamp: now - chrono::Duration::minutes(20 - i),
                cumulative_cost_usd: 1.42 - (i as f64 * 0.07),
                request_cost_usd: 0.07,
            })
            .collect();
    }
}

/// Memory panel state
pub struct MemoryPanel {
    pub backend: String,
    pub total_memories: usize,
    pub storage_bytes: u64,
    pub recent_operations: Vec<MemoryOperation>,
}

impl MemoryPanel {
    pub fn new() -> Self {
        Self {
            backend: "Qdrant".to_string(),
            total_memories: 0,
            storage_bytes: 0,
            recent_operations: Vec::new(),
        }
    }

    pub fn add_demo_operations(&mut self) {
        self.total_memories = 1247;
        self.storage_bytes = 5_242_880; // 5MB

        let now = chrono::Utc::now();
        self.recent_operations = vec![
            MemoryOperation {
                op_type: MemoryOpType::Store,
                key: Some("user_context_123".to_string()),
                success: true,
                timestamp: now - chrono::Duration::minutes(1),
            },
            MemoryOperation {
                op_type: MemoryOpType::Recall,
                key: Some("performance_data".to_string()),
                success: true,
                timestamp: now - chrono::Duration::minutes(2),
            },
            MemoryOperation {
                op_type: MemoryOpType::Delete,
                key: Some("cache_old".to_string()),
                success: true,
                timestamp: now - chrono::Duration::minutes(3),
            },
        ];
    }
}

/// Log panel state
pub struct LogPanel {
    pub log_lines: std::collections::VecDeque<LogLine>,
    pub log_level_filter: String,
}

impl LogPanel {
    pub fn new() -> Self {
        Self {
            log_lines: std::collections::VecDeque::with_capacity(200),
            log_level_filter: "INFO".to_string(),
        }
    }

    pub fn add_demo_logs(&mut self) {
        let now = chrono::Utc::now();
        self.log_lines = vec![
            LogLine {
                level: "INFO".to_string(),
                message: "Starting agent iteration".to_string(),
                module: Some("agent::loop".to_string()),
                timestamp: now - chrono::Duration::minutes(1),
            },
            LogLine {
                level: "DEBUG".to_string(),
                message: "Received TaskAssignment from planner".to_string(),
                module: Some("agent::a2a".to_string()),
                timestamp: now - chrono::Duration::minutes(1),
            },
            LogLine {
                level: "WARN".to_string(),
                message: "Rate limit approaching (85%)".to_string(),
                module: Some("provider::openai".to_string()),
                timestamp: now - chrono::Duration::seconds(30),
            },
        ].into();
    }
}

// Re-export types from subsystems for convenience
pub use subsystems::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        let state = AppState::new_demo();
        assert_eq!(state.input_mode, InputMode::Normal);
        assert!(!state.should_quit);
        assert_eq!(state.active_session, 0);
        assert!(state.panels.swarm);
        assert!(state.panels.cost);
        assert!(state.panels.memory);
        assert!(state.panels.logs);
    }

    #[test]
    fn test_panel_toggling() {
        let mut state = AppState::new_demo();

        // Toggle swarm off
        state.toggle_panel(PanelType::Swarm);
        assert!(!state.panels.swarm);

        // Toggle swarm back on
        state.toggle_panel(PanelType::Swarm);
        assert!(state.panels.swarm);
    }
}
