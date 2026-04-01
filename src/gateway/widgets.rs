//! Widget update API for World Monitor dashboard.
//!
//! Allows agents to push data to dashboard widgets via SSE broadcast.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Sse as AxumSse},
    Json,
};
use serde_json::Value;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use super::AppState;

/// Widget update payload
#[derive(serde::Deserialize)]
pub struct WidgetUpdate {
    data: Value,
}

/// Swarm event types for A2A communication
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmEventType {
    /// Agent-to-agent communication
    A2ACommunication,
    /// Widget data update
    WidgetUpdate,
    /// Agent status change
    AgentStatus,
    /// Task progress update
    TaskProgress,
    /// Agent action (for Swarm Monitor visualization)
    AgentAction,
}

/// Swarm event payload
#[derive(Debug, Clone, serde::Serialize)]
pub struct SwarmEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub data: Value,
    pub timestamp: i64,
}

impl SwarmEvent {
    /// Create A2A communication event
    pub fn a2a(from: &str, to: &str, message: &str) -> Self {
        Self {
            event_type: "a2a_communication".to_string(),
            from: Some(from.to_string()),
            to: Some(to.to_string()),
            data: serde_json::json!({ "message": message }),
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Create widget update event
    pub fn widget_update(widget_id: &str, data: Value) -> Self {
        Self {
            event_type: "widget_update".to_string(),
            from: Some(widget_id.to_string()),
            to: None,
            data,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Create agent status event
    pub fn agent_status(agent_id: &str, status: &str) -> Self {
        Self {
            event_type: "agent_status".to_string(),
            from: Some(agent_id.to_string()),
            to: None,
            data: serde_json::json!({ "status": status }),
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Create agent action event (for Swarm Monitor)
    pub fn agent_action(from: &str, to: &str, action: &str) -> Self {
        Self {
            event_type: "agent_action".to_string(),
            from: Some(from.to_string()),
            to: Some(to.to_string()),
            data: serde_json::json!({ "action": action }),
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// POST /v1/widgets/:widget_id
///
/// Accepts arbitrary JSON data for a widget and broadcasts via SSE.
pub async fn handle_widget_update(
    State(state): State<AppState>,
    Path(widget_id): Path<String>,
    Json(payload): Json<WidgetUpdate>,
) -> impl IntoResponse {
    // Log the update
    tracing::info!(
        widget_id = %widget_id,
        data = %payload.data,
        "Widget update received"
    );

    // Broadcast to SSE clients
    let event = SwarmEvent::widget_update(&widget_id, payload.data);
    let _ = state.event_tx.send(serde_json::to_value(event).unwrap());

    Json(serde_json::json!({
        "status": "ok",
        "widget_id": widget_id,
        "broadcasted": true
    }))
    .into_response()
}

/// GET /v1/widgets/:widget_id
///
/// Returns current state of a widget (placeholder for now)
pub async fn handle_widget_get(
    State(_state): State<AppState>,
    Path(widget_id): Path<String>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "widget_id": widget_id,
        "data": null
    }))
    .into_response()
}

/// POST /v1/swarm/communicate
///
/// Send A2A communication event (for testing swarm graph)
#[derive(serde::Deserialize)]
pub struct A2APayload {
    pub from: String,
    pub to: String,
    pub message: String,
}

pub async fn handle_swarm_communicate(
    State(state): State<AppState>,
    Json(payload): Json<A2APayload>,
) -> impl IntoResponse {
    tracing::info!(
        from = %payload.from,
        to = %payload.to,
        message = %payload.message,
        "A2A communication"
    );

    let event = SwarmEvent::a2a(&payload.from, &payload.to, &payload.message);
    let _ = state.event_tx.send(serde_json::to_value(event).unwrap());

    Json(serde_json::json!({
        "status": "ok",
        "event_sent": true
    }))
    .into_response()
}

/// POST /v1/swarm/action
///
/// Send agent action event (for Swarm Monitor)
#[derive(serde::Deserialize)]
pub struct AgentActionPayload {
    pub from: String,
    pub to: String,
    pub action: String,
}

pub async fn handle_agent_action(
    State(state): State<AppState>,
    Json(payload): Json<AgentActionPayload>,
) -> impl IntoResponse {
    tracing::info!(
        from = %payload.from,
        to = %payload.to,
        action = %payload.action,
        "[ZeroClaw] Agent Action"
    );

    let event = SwarmEvent::agent_action(&payload.from, &payload.to, &payload.action);
    let _ = state.event_tx.send(serde_json::to_value(event).unwrap());

    Json(serde_json::json!({
        "status": "ok",
        "action_broadcasted": true
    }))
    .into_response()
}

/// GET /v1/swarm/stream
///
/// SSE stream for real-time swarm events
pub async fn handle_swarm_stream(
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
                    // Format as SSE event
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
