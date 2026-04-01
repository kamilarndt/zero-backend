// gateway/api_tasks.rs — Tasks API handlers

use super::{api::require_auth, AppState};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TasksQuery {
    pub status: Option<String>,
}

#[derive(Deserialize)]
pub struct TaskBody {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub parent_id: Option<String>,
    pub assigned_hand: Option<String>,
}

fn task_status_from_str(s: &str) -> Option<crate::memory::tasks::TaskStatus> {
    match s {
        "Todo" => Some(crate::memory::tasks::TaskStatus::Todo),
        "InProgress" => Some(crate::memory::tasks::TaskStatus::InProgress),
        "Review" => Some(crate::memory::tasks::TaskStatus::Review),
        "Done" => Some(crate::memory::tasks::TaskStatus::Done),
        _ => None,
    }
}

fn open_tasks_db(workspace_dir: &std::path::Path) -> Result<rusqlite::Connection, (StatusCode, Json<serde_json::Value>)> {
    let db_path = workspace_dir.join(".zeroclaw/agent_tasks.db");
    rusqlite::Connection::open(&db_path).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to open tasks database: {e}")})))
    })
}

pub async fn handle_api_tasks_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<TasksQuery>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let Some(workspace_dir) = &state.workspace_dir else {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Workspace directory not configured"}))).into_response();
    };
    let conn = match open_tasks_db(workspace_dir) { Ok(c) => c, Err((s, j)) => return (s, j).into_response() };
    let _ = crate::memory::tasks::init_tasks_table(&conn);
    let status_filter = params.status.as_ref().and_then(|s| task_status_from_str(s));
    match crate::memory::tasks::list_tasks(&conn, status_filter) {
        Ok(tasks) => Json(serde_json::json!({"success": true, "data": tasks})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to list tasks: {e}")}))).into_response(),
    }
}

pub async fn handle_api_tasks_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TaskBody>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let Some(title) = &body.title else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Missing required field: title"}))).into_response();
    };
    let Some(workspace_dir) = &state.workspace_dir else {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Workspace directory not configured"}))).into_response();
    };
    let conn = match open_tasks_db(workspace_dir) { Ok(c) => c, Err((s, j)) => return (s, j).into_response() };
    let _ = crate::memory::tasks::init_tasks_table(&conn);
    let status = body.status.as_ref().and_then(|s| task_status_from_str(s)).unwrap_or(crate::memory::tasks::TaskStatus::Todo);
    let task = crate::memory::tasks::AgentTask::new(title.clone(), status, body.parent_id, body.assigned_hand);
    match crate::memory::tasks::create_task(&conn, &task) {
        Ok(()) => (StatusCode::CREATED, Json(serde_json::json!({"success": true, "data": task}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to create task: {e}")}))).into_response(),
    }
}

pub async fn handle_api_tasks_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<TaskBody>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let Some(workspace_dir) = &state.workspace_dir else {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Workspace directory not configured"}))).into_response();
    };
    let conn = match open_tasks_db(workspace_dir) { Ok(c) => c, Err((s, j)) => return (s, j).into_response() };
    let _ = crate::memory::tasks::init_tasks_table(&conn);
    match crate::memory::tasks::get_task(&conn, &id) {
        Ok(Some(_)) => {}
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Task not found"}))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to get task: {e}")}))).into_response(),
    }
    if let Some(status_str) = &body.status {
        let Some(status) = task_status_from_str(status_str) else {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("Invalid status: {status_str}")}))).into_response();
        };
        if let Err(e) = crate::memory::tasks::update_task_status(&conn, &id, status, body.assigned_hand.as_deref()) {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to update task: {e}")}))).into_response();
        }
    }
    match crate::memory::tasks::get_task(&conn, &id) {
        Ok(Some(updated)) => Json(serde_json::json!({"success": true, "data": updated})).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Task not found after update"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to fetch updated task: {e}")}))).into_response(),
    }
}

pub async fn handle_api_tasks_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let Some(workspace_dir) = &state.workspace_dir else {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Workspace directory not configured"}))).into_response();
    };
    let conn = match open_tasks_db(workspace_dir) { Ok(c) => c, Err((s, j)) => return (s, j).into_response() };
    let _ = crate::memory::tasks::init_tasks_table(&conn);
    match crate::memory::tasks::delete_task(&conn, &id) {
        Ok(()) => Json(serde_json::json!({"success": true, "message": "Task deleted"})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to delete task: {e}")}))).into_response(),
    }
}

pub async fn handle_api_tasks_interrupt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let Some(workspace_dir) = &state.workspace_dir else {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Workspace directory not configured"}))).into_response();
    };
    let conn = match open_tasks_db(workspace_dir) { Ok(c) => c, Err((s, j)) => return (s, j).into_response() };
    let task = match crate::memory::tasks::get_task(&conn, &id) {
        Ok(Some(t)) => t,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Task not found"}))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to get task: {e}")}))).into_response(),
    };
    let Some(hand_id) = task.assigned_hand else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Task has no assigned hand"}))).into_response();
    };
    if let Err(e) = state.hands.interrupt_hand_killed(&hand_id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to interrupt hand: {e}")}))).into_response();
    }
    let _ = crate::memory::tasks::update_task_status(&conn, &id, crate::memory::tasks::TaskStatus::Review, None);
    Json(serde_json::json!({"success": true, "message": format!("Hand {hand_id} interrupted; task {id} moved to Review")})).into_response()
}
