//! Message processing logic for channel messages.
//!
//! This module handles the core message flow: receiving, processing,
//! tool execution, and response delivery.

pub mod context;
pub mod handler;

pub use context::{
    ChannelRuntimeContext,
    ChannelRouteSelection,
    InFlightTaskCompletion,
    InFlightSenderTaskState,
    conversation_memory_key,
    conversation_history_key,
    interruption_scope_key,
    // Type aliases
    ConversationHistoryMap,
    ProviderCacheMap,
    RouteSelectionMap,
};

pub use handler::{process_channel_message, CHANNEL_MESSAGE_TIMEOUT_SECS};
