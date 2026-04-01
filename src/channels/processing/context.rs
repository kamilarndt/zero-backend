//! Runtime context for channel message processing.
//!
//! Provides shared state and dependencies for message handlers.

use crate::channels::traits::ChannelMessage;
use crate::config::{MultimodalConfig, ReliabilityConfig};
use crate::memory::Memory;
use crate::observability::Observer;
use crate::providers::{Provider, ProviderRuntimeOptions};
use crate::tools::Tool;
use crate::hooks::HookRunner;
use crate::skills::Skill;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

/// Channel route selection (provider + model)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelRouteSelection {
    pub provider: String,
    pub model: String,
}

/// Provider cache mapping provider name to provider instance
pub type ProviderCacheMap = Arc<Mutex<HashMap<String, Arc<dyn Provider>>>>;

/// Route selection overrides
pub type RouteSelectionMap = Arc<Mutex<HashMap<String, ChannelRouteSelection>>>;

/// Per-sender conversation history
pub type ConversationHistoryMap = Arc<Mutex<HashMap<String, Vec<crate::providers::ChatMessage>>>>;

/// Shared runtime context for all channel operations.
#[derive(Clone)]
pub struct ChannelRuntimeContext {
    /// Map of channel name to channel instance
    pub channels_by_name: Arc<HashMap<String, Arc<dyn crate::channels::traits::Channel>>>,
    /// LLM provider for generating responses
    pub provider: Arc<dyn Provider>,
    /// Default provider identifier
    pub default_provider: Arc<String>,
    /// Memory backend for persistence
    pub memory: Arc<dyn Memory>,
    /// Available tools
    pub tools_registry: Arc<Vec<Box<dyn Tool>>>,
    /// Observability observer
    pub observer: Arc<dyn Observer>,
    /// System prompt for agent
    pub system_prompt: Arc<String>,
    /// Model identifier
    pub model: Arc<String>,
    /// Temperature for generation
    pub temperature: f64,
    /// Auto-save conversations to memory
    pub auto_save_memory: bool,
    /// Maximum tool iterations per message
    pub max_tool_iterations: usize,
    /// Minimum relevance score for memory retrieval
    pub min_relevance_score: f64,
    /// Per-sender conversation history
    pub conversation_histories: ConversationHistoryMap,
    /// Provider cache for routing hints
    pub provider_cache: ProviderCacheMap,
    /// Model route overrides
    pub route_overrides: RouteSelectionMap,
    /// API key override
    pub api_key: Option<String>,
    /// API URL override
    pub api_url: Option<String>,
    /// Reliability configuration
    pub reliability: Arc<ReliabilityConfig>,
    /// Provider runtime options
    pub provider_runtime_options: ProviderRuntimeOptions,
    /// Workspace directory
    pub workspace_dir: Arc<PathBuf>,
    /// Message timeout in seconds
    pub message_timeout_secs: u64,
    /// Interrupt on new message
    pub interrupt_on_new_message: bool,
    /// Multimodal configuration
    pub multimodal: MultimodalConfig,
    /// Optional hooks
    pub hooks: Option<Arc<HookRunner>>,
    /// Tools excluded from non-CLI channels
    pub non_cli_excluded_tools: Arc<Vec<String>>,
    /// All available skills
    pub all_skills: Arc<Vec<Skill>>,
}

impl ChannelRuntimeContext {
    /// Message timeout duration
    pub fn message_timeout(&self) -> Duration {
        Duration::from_secs(self.message_timeout_secs)
    }

    /// Check if interrupt on new message is enabled
    pub fn is_interrupt_enabled(&self) -> bool {
        self.interrupt_on_new_message
    }
}

/// In-flight task completion tracking
pub struct InFlightTaskCompletion {
    done: AtomicBool,
    notify: Notify,
}

impl InFlightTaskCompletion {
    pub fn new() -> Self {
        Self {
            done: AtomicBool::new(false),
            notify: Notify::new(),
        }
    }

    pub fn mark_done(&self) {
        self.done.store(true, Ordering::Release);
        self.notify.notify_waiters();
    }

    pub async fn wait(&self) {
        if self.done.load(Ordering::Acquire) {
            return;
        }
        self.notify.notified().await;
    }
}

/// In-flight sender task state
#[derive(Clone)]
pub struct InFlightSenderTaskState {
    pub task_id: u64,
    pub cancellation: CancellationToken,
    pub completion: Arc<InFlightTaskCompletion>,
}

/// Generate conversation memory key for a message
pub fn conversation_memory_key(msg: &ChannelMessage) -> String {
    // Include thread_ts for per-topic memory isolation in forum groups
    match &msg.thread_ts {
        Some(tid) => format!("{}_{}_{}_{}", msg.channel, tid, msg.sender, msg.id),
        None => format!("{}_{}_{}", msg.channel, msg.sender, msg.id),
    }
}

/// Generate conversation history key for a message
pub fn conversation_history_key(msg: &ChannelMessage) -> String {
    // Include thread_ts for per-topic session isolation in forum groups
    match &msg.thread_ts {
        Some(tid) => format!("{}_{}_{}", msg.channel, tid, msg.sender),
        None => format!("{}_{}", msg.channel, msg.sender),
    }
}

/// Generate interruption scope key for a message
pub fn interruption_scope_key(msg: &ChannelMessage) -> String {
    format!("{}_{}_{}", msg.channel, msg.reply_target, msg.sender)
}
