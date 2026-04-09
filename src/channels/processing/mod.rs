//! Message processing logic for channel messages.
//!
//! This module handles the core message flow: receiving, processing,
//! tool execution, and response delivery.

pub mod context;
pub mod handler;

pub use context::{
    conversation_history_key,
    conversation_memory_key,
    interruption_scope_key,
    ChannelRouteSelection,
    ChannelRuntimeContext,
    InFlightSenderTaskState,
    InFlightTaskCompletion,
};

pub use handler::process_channel_message;
