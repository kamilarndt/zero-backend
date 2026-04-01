//! Event broadcasting API for World Monitor dashboard.
//!
//! Universal SSE stream that accepts and broadcasts arbitrary JSON events.
//! No specific widget or agent logic - just a dumb pipe for real-time data.

use axum::{
    extract::State,
    response::{IntoResponse, Sse as AxumSse},
    Json,
};
use serde::Deserialize;
use serde_json::Value;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use super::AppState;

/// Generic event payload - accepts any JSON
#[derive(Debug, Deserialize)]
pub struct EventPayload {
    /// Event type (e.g., "log_entry", "widget_update", "agent_action")
    #[serde(rename = "type")]
    pub event_type: String,

    /// Source identifier (optional)
    pub from: Option<String>,

    /// Target identifier (optional)
    pub to: Option<String>,

    /// Event data (any JSON)
    pub data: Option<Value>,
}

/// POST /v1/events
///
/// Universal event endpoint - accepts any event and broadcasts via SSE.
/// This is the single entry point for all dashboard updates.
pub async fn handle_emit_event(
    State(state): State<AppState>,
    Json(payload): Json<EventPayload>,
) -> impl IntoResponse {
    tracing::info!(
        event_type = %payload.event_type,
        from = ?payload.from,
        to = ?payload.to,
        "[ZeroClaw Events] Emitting"
    );

    // Construct event with timestamp
    let event = serde_json::json!({
        "type": payload.event_type,
        "from": payload.from,
        "to": payload.to,
        "data": payload.data,
        "timestamp": chrono::Utc::now().timestamp_millis()
    });

    // Broadcast to all SSE subscribers
    let _ = state.event_tx.send(event);

    Json(serde_json::json!({
        "status": "ok",
        "event_type": payload.event_type,
        "broadcasted": true
    }))
    .into_response()
}

/// GET /v1/events/stream
///
/// SSE stream - clients connect here to receive all events in real-time.
pub async fn handle_event_stream(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rx = state.event_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(
        |result: Result<
            Value,
            tokio_stream::wrappers::errors::BroadcastStreamRecvError,
        >| {
            match result {
                Ok(value) => {
                    let event_str = value.to_string();
                    Some(Ok::<_, Infallible>(
                        axum::response::sse::Event::default().data(event_str),
                    ))
                }
                Err(_) => None, // Skip lagged messages
            }
        },
    );

    AxumSse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new().interval(std::time::Duration::from_secs(10)),
    )
}
