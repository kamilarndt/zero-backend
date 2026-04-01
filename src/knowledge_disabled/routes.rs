//! REST API endpoints for the knowledge base.
//!
//! All handlers expect `AppState` with a `KnowledgeStore` injected.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::gateway::AppState;
use super::schema::*;
use super::sse;

/// GET /api/v1/kb/documents
pub async fn list_documents(State(state): State<AppState>) -> impl IntoResponse {
    let store = match state.knowledge_store() {
        Some(s) => s,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "knowledge store not initialized"}))).into_response(),
    };

    match store.list_documents() {
        Ok(docs) => (StatusCode::OK, Json(serde_json::to_value(docs).unwrap())).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// POST /api/v1/kb/documents
pub async fn create_document(
    State(state): State<AppState>,
    Json(input): Json<CreateDocument>,
) -> impl IntoResponse {
    let store = match state.knowledge_store() {
        Some(s) => s,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "knowledge store not initialized"}))).into_response(),
    };

    match store.create_document(&input) {
        Ok(doc) => {
            sse::emit(&state.event_tx, "document_created", serde_json::to_value(&doc).unwrap());
            (StatusCode::CREATED, Json(serde_json::to_value(&doc).unwrap())).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /api/v1/kb/documents/:id
pub async fn get_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let store = match state.knowledge_store() {
        Some(s) => s,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "knowledge store not initialized"}))).into_response(),
    };

    match store.get_document(&id) {
        Ok(Some(doc)) => {
            let blocks = store.get_blocks(&id).unwrap_or_default();
            let resp = DocumentWithBlocks {
                document: doc,
                blocks,
            };
            (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "document not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// PATCH /api/v1/kb/documents/:id
pub async fn update_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateDocument>,
) -> impl IntoResponse {
    let store = match state.knowledge_store() {
        Some(s) => s,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "knowledge store not initialized"}))).into_response(),
    };

    match store.update_document(&id, &input) {
        Ok(Some(doc)) => {
            sse::emit(&state.event_tx, "document_updated", serde_json::to_value(&doc).unwrap());
            (StatusCode::OK, Json(serde_json::to_value(&doc).unwrap())).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "document not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// DELETE /api/v1/kb/documents/:id
pub async fn delete_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let store = match state.knowledge_store() {
        Some(s) => s,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "knowledge store not initialized"}))).into_response(),
    };

    match store.delete_document(&id) {
        Ok(true) => {
            sse::emit(&state.event_tx, "document_deleted", serde_json::json!({"id": id}));
            (StatusCode::OK, Json(serde_json::json!({"deleted": true}))).into_response()
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "document not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

// ── Block endpoints ──────────────────────────────────────────────────────────

/// POST /api/v1/kb/documents/:id/blocks
pub async fn create_block(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    Json(input): Json<CreateBlock>,
) -> impl IntoResponse {
    let store = match state.knowledge_store() {
        Some(s) => s,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "knowledge store not initialized"}))).into_response(),
    };

    // Verify document exists
    match store.get_document(&doc_id) {
        Ok(Some(_)) => {}
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "document not found"})),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    }

    match store.create_block(&doc_id, &input) {
        Ok(block) => {
            sse::emit(&state.event_tx, "block_inserted", serde_json::to_value(&block).unwrap());
            (StatusCode::CREATED, Json(serde_json::to_value(&block).unwrap())).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// PATCH /api/v1/kb/documents/:doc_id/blocks/:block_id
pub async fn update_block(
    State(state): State<AppState>,
    Path((doc_id, block_id)): Path<(String, String)>,
    Json(input): Json<UpdateBlock>,
) -> impl IntoResponse {
    let store = match state.knowledge_store() {
        Some(s) => s,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "knowledge store not initialized"}))).into_response(),
    };

    match store.update_block(&doc_id, &block_id, &input) {
        Ok(Some(block)) => {
            sse::emit(&state.event_tx, "block_updated", serde_json::to_value(&block).unwrap());
            (StatusCode::OK, Json(serde_json::to_value(&block).unwrap())).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "block not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// DELETE /api/v1/kb/documents/:doc_id/blocks/:block_id
pub async fn delete_block(
    State(state): State<AppState>,
    Path((doc_id, block_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let store = match state.knowledge_store() {
        Some(s) => s,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "knowledge store not initialized"}))).into_response(),
    };

    match store.delete_block(&doc_id, &block_id) {
        Ok(true) => {
            sse::emit(
                &state.event_tx,
                "block_deleted",
                serde_json::json!({"id": block_id, "document_id": doc_id}),
            );
            (StatusCode::OK, Json(serde_json::json!({"deleted": true}))).into_response()
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "block not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}
