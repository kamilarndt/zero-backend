// gateway/api_sops.rs — SOPs API handlers

use super::{api::require_auth, AppState};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

pub async fn handle_api_sops_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let category = crate::memory::MemoryCategory::Sop;
    match state.mem.list(Some(&category), None).await {
        Ok(sops) => Json(serde_json::json!({"success": true, "data": sops})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to fetch SOPs: {e}")}))).into_response(),
    }
}

#[derive(Deserialize)]
pub struct SopCreateBody {
    pub name: String,
    pub yaml: String,
}

pub async fn handle_api_sops_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SopCreateBody>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let id = format!("sop_{}", uuid::Uuid::new_v4());
    let content = format!("---\nname: {}\n---\n{}", body.name, body.yaml);
    match state.mem.store(&id, &content, crate::memory::MemoryCategory::Sop, None).await {
        Ok(_) => Json(serde_json::json!({
            "success": true,
            "data": {
                "id": id,
                "name": body.name,
                "yaml": body.yaml,
                "createdAt": chrono::Utc::now().to_rfc3339(),
                "updatedAt": chrono::Utc::now().to_rfc3339()
            }
        })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to create SOP: {e}")}))).into_response(),
    }
}

pub async fn handle_api_sops_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<SopCreateBody>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let content = format!("---\nname: {}\n---\n{}", body.name, body.yaml);
    match state.mem.store(&id, &content, crate::memory::MemoryCategory::Sop, None).await {
        Ok(_) => Json(serde_json::json!({
            "success": true,
            "data": {
                "id": id,
                "name": body.name,
                "yaml": body.yaml,
                "updatedAt": chrono::Utc::now().to_rfc3339()
            }
        })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to update SOP: {e}")}))).into_response(),
    }
}
