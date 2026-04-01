// gateway/api_skills.rs — Skills Management API handlers

use super::{api::require_auth, AppState};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};

pub async fn handle_list_skills(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let skills = if let Some(engine) = state.skill_engine.as_ref() {
        engine.list_skills(true).await.unwrap_or_default()
    } else {
        Vec::new()
    };
    let response: Vec<serde_json::Value> = skills.into_iter().map(|s| {
        serde_json::json!({
            "id": s.id, "name": s.name, "description": s.description,
            "version": s.version, "author": s.author, "tags": s.tags,
            "is_active": s.is_active, "created_at": s.created_at,
        })
    }).collect();
    Json(serde_json::json!({"skills": response, "count": response.len()})).into_response()
}

pub async fn skill_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(_skill): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let Some(_engine) = &state.skill_engine else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": "Skills engine not available"}))).into_response();
    };
    (StatusCode::NOT_IMPLEMENTED, Json(serde_json::json!({"error": "Skill creation not yet implemented"}))).into_response()
}

pub async fn handle_get_skill(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let Some(engine) = &state.skill_engine else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": "Skills engine not available"}))).into_response();
    };
    match engine.get_skill(id).await {
        Ok(Some(skill)) => Json(serde_json::json!({
            "id": skill.id, "name": skill.name, "description": skill.description,
            "content": skill.content, "version": skill.version, "author": skill.author,
            "tags": skill.tags, "is_active": skill.is_active,
            "created_at": skill.created_at, "updated_at": skill.updated_at,
        })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Skill not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("{}", e)}))).into_response(),
    }
}

pub async fn skill_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(_id): Path<i64>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await { return e.into_response(); }
    let Some(_engine) = &state.skill_engine else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": "Skills engine not available"}))).into_response();
    };
    (StatusCode::NOT_IMPLEMENTED, Json(serde_json::json!({"error": "Skill deletion not yet implemented"}))).into_response()
}
