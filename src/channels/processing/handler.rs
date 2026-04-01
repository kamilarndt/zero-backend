//! Message handler implementation.
//!
//! Processes incoming channel messages through the agent loop.

use super::context::ChannelRuntimeContext;
use crate::channels::traits::ChannelMessage;
use tokio_util::sync::CancellationToken;

/// Default message timeout in seconds
pub const CHANNEL_MESSAGE_TIMEOUT_SECS: u64 = 120;

/// Process a channel message through the agent loop.
///
/// This function handles the complete message flow:
/// 1. Message validation and preprocessing
/// 2. Memory retrieval (if enabled)
/// 3. Agent loop execution with tool calls
/// 4. Response delivery
/// 5. History persistence
pub async fn process_channel_message(
    ctx: std::sync::Arc<ChannelRuntimeContext>,
    msg: ChannelMessage,
    cancellation_token: CancellationToken,
) {
    // Delegate to the implementation in mod.rs
    // TODO: Move implementation here in follow-up commit
    crate::channels::process_channel_message_impl(ctx, msg, cancellation_token).await
}
