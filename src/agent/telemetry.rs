//! Telemetry bridge for agent loop → SSE dashboard.
//!
//! Provides a global broadcast sender that the agent loop can use to push
//! real-time state events (thinking, tool execution, delegation) to the
//! web dashboard via the SSE channel.
//!
//! The gateway initializes this once at startup via [`init()`].
//! The agent loop calls [`emit()`] at key lifecycle points.

use serde_json::json;
use std::sync::OnceLock;
use tokio::sync::broadcast;

/// Global broadcast sender for telemetry events.
static TELEMETRY_TX: OnceLock<broadcast::Sender<serde_json::Value>> = OnceLock::new();

/// Initialize the telemetry channel. Must be called once at gateway startup.
/// Returns the sender so callers can also wire it into their own broadcast.
pub fn init(capacity: usize) -> broadcast::Sender<serde_json::Value> {
    let (tx, _) = broadcast::channel::<serde_json::Value>(capacity);
    let tx_clone = tx.clone();
    TELEMETRY_TX.set(tx_clone).expect("telemetry already initialized");
    tx
}

/// Get a reference to the global telemetry sender, if initialized.
pub fn sender() -> Option<&'static broadcast::Sender<serde_json::Value>> {
    TELEMETRY_TX.get()
}

/// Subscribe to telemetry events. Returns `None` if not initialized.
pub fn subscribe() -> Option<broadcast::Receiver<serde_json::Value>> {
    TELEMETRY_TX.get().map(|tx| tx.subscribe())
}

/// Emit a telemetry event to the SSE broadcast channel.
///
/// # Arguments
/// * `action` — The telemetry action: "reasoning", "tool_execution", "delegation", "success"
/// * `details` — Human-readable description of what the agent is doing
/// * `status` — "running", "completed", "failed"
/// * `extra` — Optional additional data merged into the event
pub fn emit(
    action: &str,
    details: &str,
    status: &str,
    extra: Option<serde_json::Value>,
) {
    if let Some(tx) = TELEMETRY_TX.get() {
        let mut event = json!({
            "type": "telemetry",
            "action": action,
            "details": details,
            "status": status,
            "timestamp": chrono::Utc::now().timestamp_millis(),
        });

        // Merge extra fields into the event
        if let Some(extra_obj) = extra {
            if let (Some(event_map), Some(extra_map)) =
                (event.as_object_mut(), extra_obj.as_object())
            {
                for (k, v) in extra_map {
                    event_map.insert(k.clone(), v.clone());
                }
            }
        }

        // Fire-and-forget: ignore if no subscribers
        let _ = tx.send(event);
    }
}

/// Convenience: emit a "reasoning" telemetry event.
pub fn emit_reasoning(details: &str) {
    emit("reasoning", details, "running", None);
}

/// Convenience: emit a "tool_execution" telemetry event.
pub fn emit_tool_start(tool_name: &str, iteration: usize) {
    emit(
        "tool_execution",
        &format!("executing tool: {tool_name}"),
        "running",
        Some(json!({
            "tool": tool_name,
            "iteration": iteration,
        })),
    );
}

/// Convenience: emit a "tool_execution" completion event.
pub fn emit_tool_done(tool_name: &str, success: bool, duration_ms: u128) {
    emit(
        "tool_execution",
        &format!(
            "tool {tool_name} {}",
            if success { "completed" } else { "failed" }
        ),
        if success { "completed" } else { "failed" },
        Some(json!({
            "tool": tool_name,
            "duration_ms": duration_ms,
        })),
    );
}

/// Convenience: emit a "delegation" telemetry event.
pub fn emit_delegation(agent_name: &str, status: &str) {
    emit(
        "delegation",
        &format!("delegating to agent: {agent_name}"),
        status,
        Some(json!({
            "agent": agent_name,
        })),
    );
}

/// Convenience: emit a "success" telemetry event (final response ready).
pub fn emit_success(details: &str) {
    emit("success", details, "completed", None);
}
