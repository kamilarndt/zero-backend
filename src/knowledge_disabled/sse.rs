//! SSE event helpers for the knowledge base.
//!
//! Emits events through the existing `AppState::event_tx` broadcast channel.

use serde_json::Value;
use tokio::sync::broadcast::Sender;

/// Broadcast a knowledge-base event to all SSE subscribers.
pub fn emit(sender: &Sender<Value>, event_type: &str, data: Value) {
    let event = serde_json::json!({
        "type": event_type,
        "from": "knowledge",
        "to": null,
        "data": data,
        "timestamp": chrono::Utc::now().timestamp_millis()
    });
    let _ = sender.send(event);
}
