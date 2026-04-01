// gateway/api_tui.rs — TUI API handlers

use super::{api::require_auth, AppState};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};

pub async fn handle_tui_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let session_id = body.get("session_id").and_then(|v| v.as_str()).unwrap_or("default");
    let content = match body.get("content").and_then(|v| v.as_str()) {
        Some(msg) if !msg.is_empty() => msg,
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"response": serde_json::Value::Null, "error": "Missing or empty 'content' field"}))).into_response(),
    };
    let config = state.config.lock().clone();
    let response = match crate::agent::process_message(config, content).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("TUI chat error: {e:#}");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"response": serde_json::Value::Null, "error": format!("Processing error: {}", e)}))).into_response();
        }
    };
    if state.auto_save {
        let _ = state.mem.store(&format!("tui:{}:user", session_id), content, crate::memory::MemoryCategory::Conversation, Some(session_id)).await;
        let _ = state.mem.store(&format!("tui:{}:assistant", session_id), &response, crate::memory::MemoryCategory::Conversation, Some(session_id)).await;
    }
    Json(serde_json::json!({"response": response, "error": null})).into_response()
}

pub async fn handle_tui_agents_active(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let active_hands = state.hands.list_active_hands().await;
    let mut agents = Vec::new();
    for (hand_id, hand_state) in active_hands {
        agents.push(serde_json::json!({
            "id": hand_id,
            "name": format!("Hand: {}", hand_id),
            "model": "hand",
            "progress": if hand_state.token.is_cancelled() { 100 } else { 50 },
            "status": if hand_state.token.is_cancelled() { "done" } else { "running" },
            "current_task": hand_state.workspace_path.and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string())),
        }));
    }
    Json(agents).into_response()
}

pub async fn handle_tui_memory_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let backend_name = state.mem.name();
    let total_memories = state.mem.count().await.unwrap_or(0);
    Json(serde_json::json!({
        "backend": backend_name,
        "total_memories": total_memories,
        "storage_bytes": 0u64,
        "recent_operations": [],
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })).into_response()
}

pub async fn handle_tui_logs_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let now = chrono::Utc::now();
    let mock_logs = serde_json::json!([
        {"level": "INFO", "message": "ZeroClaw backend started", "module": "zeroclaw::main", "timestamp": (now - chrono::Duration::minutes(5)).to_rfc3339()},
        {"level": "DEBUG", "message": "Initializing memory backend", "module": "zeroclaw::memory", "timestamp": (now - chrono::Duration::minutes(4)).to_rfc3339()},
    ]);
    Json(serde_json::json!({"log_lines": mock_logs, "total_lines": 2, "timestamp": now.to_rfc3339()})).into_response()
}

pub async fn handle_tui_routing_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let config = state.config.lock();
    let response = serde_json::json!({
        "active_provider": config.default_provider.clone().unwrap_or_else(|| "unknown".to_string()),
        "model": state.model.clone(),
        "temperature": state.temperature,
        "quota_used_percent": 0.0,
        "fallback_active": false,
        "paired": state.pairing.is_paired(),
    });
    drop(config);
    Json(response).into_response()
}

pub async fn handle_diagnostic(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if state.pairing.require_pairing() {
        if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    }
    let config = state.config.lock().clone();
    let mut checks = Vec::new();
    // Config validation
    checks.push(serde_json::json!({"name": "Config", "status": "ok", "message": format!("Provider: {}", config.default_provider.as_deref().unwrap_or("unknown"))}));
    // API key
    if config.api_key.is_some() {
        checks.push(serde_json::json!({"name": "API Key", "status": "ok", "message": "API key is configured"}));
    } else {
        checks.push(serde_json::json!({"name": "API Key", "status": "warning", "message": "No API key configured"}));
    }
    // Memory backend
    let mem_name = state.mem.name();
    let mem_count = state.mem.count().await.unwrap_or(0);
    checks.push(serde_json::json!({"name": "Memory", "status": "ok", "message": format!("{mem_name} backend, {mem_count} entries")}));
    // Gateway
    checks.push(serde_json::json!({"name": "Gateway", "status": "ok", "message": format!("Paired: {}", state.pairing.is_paired())}));
    // Channels
    let channels = config.channels_config.channels_except_webhook();
    let active_channels: Vec<&str> = channels.iter().filter(|(_, ok)| *ok).map(|(handle, _)| handle.name()).collect();
    checks.push(serde_json::json!({"name": "Channels", "status": if active_channels.is_empty() { "warning" } else { "ok" }, "message": if active_channels.is_empty() { "No channels configured".to_string() } else { format!("Active: {}", active_channels.join(", ")) }}));
    Json(serde_json::json!({"checks": checks, "timestamp": chrono::Utc::now().to_rfc3339()})).into_response()
}
