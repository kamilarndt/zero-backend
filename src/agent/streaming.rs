//! Streaming agent implementation for OpenAI compatibility layer.

use crate::providers::ChatMessage;

/// Request for streaming agent turn
#[derive(Debug, Clone)]
pub struct AgentTurnRequest {
    pub messages: Vec<ChatMessage>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub system_prompt: Option<String>,
    pub enable_tools: bool,
    pub enable_memory: bool,
}

/// Streaming chunk from agent
#[derive(Debug, Clone)]
pub enum AgentStreamChunk {
    /// Regular text content from LLM
    Text(String),

    /// Tool execution started
    ToolStart {
        tool_id: String,
        tool_name: String,
        args: serde_json::Value,
    },

    /// Tool execution result
    ToolResult { tool_id: String, result: String },

    /// All tool executions complete
    ToolEnd,

    /// Stream complete
    Done,
}

/// Tool loop state for streaming state machine
enum ToolLoopState {
    Start,
    Streaming,
    ExecutingTools,
    Done,
}
