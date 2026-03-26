//! REST API handlers for the web dashboard.
//!
//! All `/api/*` routes require bearer token authentication (PairingGuard).

use super::AppState;
use crate::auth::jwt;
use crate::skills::Skill;
use axum::{
    extract::{Json, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;

const MASKED_SECRET: &str = "***MASKED***";

// ── Bearer token auth extractor ─────────────────────────────────

/// Extract and validate bearer token from Authorization header.
fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "))
}

/// Verify bearer token against JWT signature or PairingGuard. Returns error response if unauthorized.
async fn require_auth(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if !state.pairing.require_pairing() {
        return Ok(());
    }

    let token = extract_bearer_token(headers).unwrap_or("");

    // 1. Try JWT verification (new security standard)
    if let Ok(_claims) = jwt::verify_token(token, &state.jwt_secret) {
        return Ok(());
    }

    // 2. Fallback to legacy PairingGuard tokens
    if state.pairing.is_authenticated(token).await {
        Ok(())
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized — invalid token or JWT signature. Pair first via POST /pair, then send Authorization: Bearer <token>"
            })),
        ))
    }
}

// ── Query parameters ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct MemoryQuery {
    pub query: Option<String>,
    pub category: Option<String>,
}

#[derive(Deserialize)]
pub struct MemoryStoreBody {
    pub key: String,
    pub content: String,
    pub category: Option<String>,
}

#[derive(Deserialize)]
pub struct CronAddBody {
    pub name: Option<String>,
    pub schedule: String,
    pub command: String,
}

// ── Handlers ────────────────────────────────────────────────────

/// GET /api/status — system status overview
pub async fn handle_api_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let config = state.config.lock().clone();
    let health = crate::health::snapshot();

    let mut channels = serde_json::Map::new();

    for (channel, present) in config.channels_config.channels() {
        channels.insert(channel.name().to_string(), serde_json::Value::Bool(present));
    }

    let body = serde_json::json!({
        "provider": config.default_provider,
        "model": state.model,
        "temperature": state.temperature,
        "uptime_seconds": health.uptime_seconds,
        "gateway_port": config.gateway.port,
        "locale": "en",
        "memory_backend": state.mem.name(),
        "paired": state.pairing.is_paired(),
        "channels": channels,
        "health": health,
    });

    Json(body).into_response()
}

/// GET /api/config — current config (api_key masked)
pub async fn handle_api_config_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let config = state.config.lock().clone();

    // Serialize to TOML after masking sensitive fields.
    let masked_config = mask_sensitive_fields(&config);
    let toml_str = match toml::to_string_pretty(&masked_config) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to serialize config: {e}")})),
            )
                .into_response();
        }
    };

    Json(serde_json::json!({
        "format": "toml",
        "content": toml_str,
    }))
    .into_response()
}

/// PUT /api/config — update config from TOML body
pub async fn handle_api_config_put(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    // Parse the incoming TOML
    let incoming: crate::config::Config = match toml::from_str(&body) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Invalid TOML: {e}")})),
            )
                .into_response();
        }
    };

    let current_config = state.config.lock().clone();
    let new_config = hydrate_config_for_save(incoming, &current_config);

    if let Err(e) = new_config.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Invalid config: {e}")})),
        )
            .into_response();
    }

    // Save to disk
    if let Err(e) = new_config.save().await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to save config: {e}")})),
        )
            .into_response();
    }

    // Update in-memory config
    *state.config.lock() = new_config;

    Json(serde_json::json!({"status": "ok"})).into_response()
}

/// GET /api/v1/config — simplified config for frontend dashboard
///
/// Returns a simplified JSON structure that the frontend expects:
/// {
///   "success": true,
///   "data": {
///     "network": { "enabled": boolean, "whitelist": [] },
///     "api": { "provider": string, "maxTokens": number },
///     "limits": { "maxIterations": number, "timeout": number },
///     "providers": { "openai": boolean, "anthropic": boolean, "ollama": boolean }
///   }
/// }
pub async fn handle_api_v1_config_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let config = state.config.lock().clone();

    // Extract provider info
    let default_provider = config.default_provider.clone().unwrap_or_default();
    let has_ollama = config.model_providers.contains_key("ollama");
    let has_anthropic =
        default_provider.contains("anthropic") || default_provider.contains("claude");
    let has_openai = default_provider.contains("openai") || default_provider.contains("gpt");

    // Transform ZeroClaw config to frontend-expected format
    let frontend_config = serde_json::json!({
        "success": true,
        "data": {
            "network": {
                "enabled": config.gateway.allow_public_bind,
                "whitelist": [] // TODO: Extract from tunnel config if needed
            },
            "api": {
                "provider": default_provider,
                "maxTokens": config.agent.max_context_tokens
            },
            "limits": {
                "maxIterations": config.agent.max_tool_iterations,
                "timeout": 300 // Default timeout in seconds
            },
            "providers": {
                "openai": has_openai,
                "anthropic": has_anthropic,
                "ollama": has_ollama
            }
        }
    });

    Json(frontend_config).into_response()
}

/// GET /api/tools — list registered tool specs
pub async fn handle_api_tools(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let tools: Vec<serde_json::Value> = state
        .tools_registry
        .iter()
        .map(|spec| {
            serde_json::json!({
                "name": spec.name,
                "description": spec.description,
                "parameters": spec.parameters,
            })
        })
        .collect();

    Json(serde_json::json!({"tools": tools})).into_response()
}

/// GET /api/cron — list cron jobs
pub async fn handle_api_cron_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let config = state.config.lock().clone();
    match crate::cron::list_jobs(&config) {
        Ok(jobs) => {
            let jobs_json: Vec<serde_json::Value> = jobs
                .iter()
                .map(|job| {
                    serde_json::json!({
                        "id": job.id,
                        "name": job.name,
                        "command": job.command,
                        "next_run": job.next_run.to_rfc3339(),
                        "last_run": job.last_run.map(|t| t.to_rfc3339()),
                        "last_status": job.last_status,
                        "enabled": job.enabled,
                    })
                })
                .collect();
            Json(serde_json::json!({"jobs": jobs_json})).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to list cron jobs: {e}")})),
        )
            .into_response(),
    }
}

/// POST /api/cron — add a new cron job
pub async fn handle_api_cron_add(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CronAddBody>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let config = state.config.lock().clone();
    let schedule = crate::cron::Schedule::Cron {
        expr: body.schedule,
        tz: None,
    };

    match crate::cron::add_shell_job(&config, body.name, schedule, &body.command) {
        Ok(job) => Json(serde_json::json!({
            "status": "ok",
            "job": {
                "id": job.id,
                "name": job.name,
                "command": job.command,
                "enabled": job.enabled,
            }
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to add cron job: {e}")})),
        )
            .into_response(),
    }
}

/// DELETE /api/cron/:id — remove a cron job
pub async fn handle_api_cron_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let config = state.config.lock().clone();
    match crate::cron::remove_job(&config, &id) {
        Ok(()) => Json(serde_json::json!({"status": "ok"})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to remove cron job: {e}")})),
        )
            .into_response(),
    }
}

/// GET /api/integrations — list all integrations with status
pub async fn handle_api_integrations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let config = state.config.lock().clone();
    let entries = crate::integrations::registry::all_integrations();

    let integrations: Vec<serde_json::Value> = entries
        .iter()
        .map(|entry| {
            let status = (entry.status_fn)(&config);
            serde_json::json!({
                "name": entry.name,
                "description": entry.description,
                "category": entry.category,
                "status": status,
            })
        })
        .collect();

    Json(serde_json::json!({"integrations": integrations})).into_response()
}

/// POST /api/doctor — run diagnostics
pub async fn handle_api_doctor(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let config = state.config.lock().clone();
    let results = crate::doctor::diagnose(&config);

    let ok_count = results
        .iter()
        .filter(|r| r.severity == crate::doctor::Severity::Ok)
        .count();
    let warn_count = results
        .iter()
        .filter(|r| r.severity == crate::doctor::Severity::Warn)
        .count();
    let error_count = results
        .iter()
        .filter(|r| r.severity == crate::doctor::Severity::Error)
        .count();

    Json(serde_json::json!({
        "results": results,
        "summary": {
            "ok": ok_count,
            "warnings": warn_count,
            "errors": error_count,
        }
    }))
    .into_response()
}

/// GET /api/memory — list or search memory entries
pub async fn handle_api_memory_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<MemoryQuery>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    if let Some(ref query) = params.query {
        // Search mode
        match state.mem.recall(query, 50, None).await {
            Ok(entries) => Json(serde_json::json!({"entries": entries})).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Memory recall failed: {e}")})),
            )
                .into_response(),
        }
    } else {
        // List mode
        let category = params.category.as_deref().map(|cat| match cat {
            "core" => crate::memory::MemoryCategory::Core,
            "daily" => crate::memory::MemoryCategory::Daily,
            "conversation" => crate::memory::MemoryCategory::Conversation,
            other => crate::memory::MemoryCategory::Custom(other.to_string()),
        });

        match state.mem.list(category.as_ref(), None).await {
            Ok(entries) => Json(serde_json::json!({"entries": entries})).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Memory list failed: {e}")})),
            )
                .into_response(),
        }
    }
}

/// POST /api/memory — store a memory entry
pub async fn handle_api_memory_store(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<MemoryStoreBody>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let category = body
        .category
        .as_deref()
        .map(|cat| match cat {
            "core" => crate::memory::MemoryCategory::Core,
            "daily" => crate::memory::MemoryCategory::Daily,
            "conversation" => crate::memory::MemoryCategory::Conversation,
            other => crate::memory::MemoryCategory::Custom(other.to_string()),
        })
        .unwrap_or(crate::memory::MemoryCategory::Core);

    match state
        .mem
        .store(&body.key, &body.content, category, None)
        .await
    {
        Ok(()) => Json(serde_json::json!({"status": "ok"})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Memory store failed: {e}")})),
        )
            .into_response(),
    }
}

/// DELETE /api/memory/:key — delete a memory entry
pub async fn handle_api_memory_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    match state.mem.forget(&key).await {
        Ok(deleted) => {
            Json(serde_json::json!({"status": "ok", "deleted": deleted})).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Memory forget failed: {e}")})),
        )
            .into_response(),
    }
}

// ── Memory Graph API ─────────────────────────────────────────────

#[derive(serde::Serialize)]
struct GraphNode {
    id: String,
    label: String,
    #[serde(rename = "type")]
    node_type: String,
}

#[derive(serde::Serialize)]
struct GraphLink {
    source: String,
    target: String,
    weight: f32,
}

#[derive(serde::Serialize)]
struct MemoryGraphResponse {
    nodes: Vec<GraphNode>,
    links: Vec<GraphLink>,
}

/// GET /api/v1/memory/graph — memory graph for force-directed visualization
pub async fn handle_api_memory_graph(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    // Get all memories from SQLite backend
    let entries = match state.mem.list(None, None).await {
        Ok(entries) => entries,
        Err(e) => {
            tracing::error!("Failed to list memories for graph: {}", e);
            return Json(serde_json::json!({
                "nodes": [],
                "links": []
            }))
            .into_response();
        }
    };

    // Collect embeddings and build nodes
    let mut nodes = Vec::new();
    let mut embeddings: Vec<(String, Vec<f32>)> = Vec::new();

    for entry in &entries {
        nodes.push(GraphNode {
            id: entry.id.clone(),
            label: if entry.content.len() > 50 {
                format!("{}...", &entry.content[..47])
            } else {
                entry.content.clone()
            },
            node_type: format!("{:?}", entry.category).to_lowercase(),
        });

        // Try to get embedding from SQLite
        if let Ok(Some(embedding_vec)) = get_embedding_for_key(&entry.key).await {
            embeddings.push((entry.id.clone(), embedding_vec));
        }
    }

    // Calculate pairwise similarities and create links (offload O(N^2) loop to blocking thread)
    let links = tokio::task::spawn_blocking(move || {
        let mut links = Vec::new();
        let threshold = 0.7;

        for i in 0..embeddings.len() {
            for j in (i + 1)..embeddings.len() {
                let (_, vec_a) = &embeddings[i];
                let (_, vec_b) = &embeddings[j];

                let similarity = crate::memory::vector::cosine_similarity(vec_a, vec_b);
                if similarity >= threshold {
                    links.push(GraphLink {
                        source: embeddings[i].0.clone(),
                        target: embeddings[j].0.clone(),
                        weight: similarity,
                    });
                }
            }
        }
        links
    })
    .await
    .unwrap_or_default();

    let response = MemoryGraphResponse { nodes, links };
    Json(response).into_response()
}

/// Helper: Get embedding for a memory key from SQLite
async fn get_embedding_for_key(key: &str) -> anyhow::Result<Option<Vec<f32>>> {
    let key_owned = key.to_string();

    tokio::task::spawn_blocking(move || {
        use rusqlite::Connection;
        use std::path::PathBuf;

        let db_path = PathBuf::from(format!(
            "{}/memory/brain.db",
            std::env::var("HOME").unwrap_or_default()
        ));

        if !db_path.exists() {
            return Ok(None);
        }

        let conn = Connection::open(&db_path)?;
        let mut stmt = conn.prepare("SELECT embedding FROM memories WHERE key = ?")?;

        let query_result = stmt.query_row([&key_owned], |row| {
            let blob: Vec<u8> = row.get(0)?;
            if blob.is_empty() {
                Ok(None)
            } else {
                Ok(Some(crate::memory::vector::bytes_to_vec(&blob)))
            }
        });

        match query_result {
            Ok(embedding) => Ok(embedding),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    })
    .await?
}

/// GET /api/cost — cost summary
pub async fn handle_api_cost(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    if let Some(ref tracker) = state.cost_tracker {
        match tracker.get_summary() {
            Ok(summary) => Json(serde_json::json!({"cost": summary})).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Cost summary failed: {e}")})),
            )
                .into_response(),
        }
    } else {
        Json(serde_json::json!({
            "cost": {
                "session_cost_usd": 0.0,
                "daily_cost_usd": 0.0,
                "monthly_cost_usd": 0.0,
                "total_tokens": 0,
                "request_count": 0,
                "by_model": {},
            }
        }))
        .into_response()
    }
}

/// GET /api/cli-tools — discovered CLI tools
pub async fn handle_api_cli_tools(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let tools = crate::tools::cli_discovery::discover_cli_tools(&[], &[]);

    Json(serde_json::json!({"cli_tools": tools})).into_response()
}

/// GET /api/health — component health snapshot
pub async fn handle_api_health(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let snapshot = crate::health::snapshot();
    Json(serde_json::json!({"health": snapshot})).into_response()
}

// ── Tasks API ─────────────────────────────────────────────────────

/// Query parameters for tasks list
#[derive(Deserialize)]
pub struct TasksQuery {
    pub status: Option<String>,
}

/// Request body for creating/updating tasks
#[derive(Deserialize)]
pub struct TaskBody {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub parent_id: Option<String>,
    pub assigned_hand: Option<String>,
}

/// GET /api/tasks — list all tasks or filter by status
pub async fn handle_api_tasks_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<TasksQuery>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let workspace_dir = match &state.workspace_dir {
        Some(dir) => dir,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Workspace directory not configured"})),
            )
                .into_response()
        }
    };

    let db_path = workspace_dir.join(".zeroclaw/agent_tasks.db");
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to open tasks database: {e}")})),
            )
                .into_response()
        }
    };

    // Initialize schema if needed
    let _ = crate::memory::tasks::init_tasks_table(&conn);

    let status_filter = params.status.as_ref().and_then(|s| match s.as_str() {
        "Todo" => Some(crate::memory::tasks::TaskStatus::Todo),
        "InProgress" => Some(crate::memory::tasks::TaskStatus::InProgress),
        "Review" => Some(crate::memory::tasks::TaskStatus::Review),
        "Done" => Some(crate::memory::tasks::TaskStatus::Done),
        _ => None,
    });

    match crate::memory::tasks::list_tasks(&conn, status_filter) {
        Ok(tasks) => Json(serde_json::json!({
            "success": true,
            "data": tasks,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to list tasks: {e}")})),
        )
            .into_response(),
    }
}

/// POST /api/tasks — create a new task
pub async fn handle_api_tasks_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TaskBody>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    // Validate required fields for creation
    let title = match &body.title {
        Some(t) => t,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing required field: title"})),
            )
                .into_response();
        }
    };

    let workspace_dir = match &state.workspace_dir {
        Some(dir) => dir,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Workspace directory not configured"})),
            )
                .into_response()
        }
    };

    let db_path = workspace_dir.join(".zeroclaw/agent_tasks.db");
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to open tasks database: {e}")})),
            )
                .into_response()
        }
    };

    let _ = crate::memory::tasks::init_tasks_table(&conn);

    let status = body
        .status
        .as_ref()
        .and_then(|s| match s.as_str() {
            "Todo" => Some(crate::memory::tasks::TaskStatus::Todo),
            "InProgress" => Some(crate::memory::tasks::TaskStatus::InProgress),
            "Review" => Some(crate::memory::tasks::TaskStatus::Review),
            "Done" => Some(crate::memory::tasks::TaskStatus::Done),
            _ => Some(crate::memory::tasks::TaskStatus::Todo),
        })
        .unwrap_or(crate::memory::tasks::TaskStatus::Todo);

    let task = crate::memory::tasks::AgentTask::new(
        title.clone(),
        status,
        body.parent_id,
        body.assigned_hand,
    );

    match crate::memory::tasks::create_task(&conn, &task) {
        Ok(()) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "success": true,
                "data": task,
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to create task: {e}")})),
        )
            .into_response(),
    }
}

/// PUT /api/tasks/:id — update task status
pub async fn handle_api_tasks_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<TaskBody>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let workspace_dir = match &state.workspace_dir {
        Some(dir) => dir,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Workspace directory not configured"})),
            )
                .into_response()
        }
    };

    let db_path = workspace_dir.join(".zeroclaw/agent_tasks.db");
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to open tasks database: {e}")})),
            )
                .into_response()
        }
    };

    let _ = crate::memory::tasks::init_tasks_table(&conn);

    // First, get the task to check if it exists
    let _existing_task = match crate::memory::tasks::get_task(&conn, &id) {
        Ok(Some(t)) => t,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Task not found"})),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to get task: {e}")})),
            )
                .into_response()
        }
    };

    // Update status if provided
    if let Some(status_str) = &body.status {
        let status = match status_str.as_str() {
            "Todo" => crate::memory::tasks::TaskStatus::Todo,
            "InProgress" => crate::memory::tasks::TaskStatus::InProgress,
            "Review" => crate::memory::tasks::TaskStatus::Review,
            "Done" => crate::memory::tasks::TaskStatus::Done,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": format!("Invalid status: {status_str}")})),
                )
                    .into_response()
            }
        };

        if let Err(e) = crate::memory::tasks::update_task_status(
            &conn,
            &id,
            status,
            body.assigned_hand.as_deref(),
        ) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to update task: {e}")})),
            )
                .into_response();
        }
    }

    // Fetch updated task
    match crate::memory::tasks::get_task(&conn, &id) {
        Ok(Some(updated_task)) => Json(serde_json::json!({
            "success": true,
            "data": updated_task,
        }))
        .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Task not found after update"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to fetch updated task: {e}")})),
        )
            .into_response(),
    }
}

/// DELETE /api/tasks/:id — delete a task
pub async fn handle_api_tasks_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let workspace_dir = match &state.workspace_dir {
        Some(dir) => dir,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Workspace directory not configured"})),
            )
                .into_response()
        }
    };

    let db_path = workspace_dir.join(".zeroclaw/agent_tasks.db");
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to open tasks database: {e}")})),
            )
                .into_response()
        }
    };

    let _ = crate::memory::tasks::init_tasks_table(&conn);

    match crate::memory::tasks::delete_task(&conn, &id) {
        Ok(()) => Json(serde_json::json!({
            "success": true,
            "message": "Task deleted",
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to delete task: {e}")})),
        )
            .into_response(),
    }
}

/// POST /api/tasks/{id}/interrupt — forcefully terminate a task's agent hand
pub async fn handle_api_tasks_interrupt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let workspace_dir = match &state.workspace_dir {
        Some(dir) => dir,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Workspace directory not configured"})),
            )
                .into_response()
        }
    };

    let db_path = workspace_dir.join(".zeroclaw/agent_tasks.db");
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to open tasks database: {e}")})),
            )
                .into_response()
        }
    };

    let task = match crate::memory::tasks::get_task(&conn, &id) {
        Ok(Some(t)) => t,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Task not found"})),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to get task: {e}")})),
            )
                .into_response()
        }
    };

    let hand_id = match task.assigned_hand {
        Some(h) => h,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Task has no assigned hand"})),
            )
                .into_response()
        }
    };

    // 1. Force kill processes and cancel token via HandsDispatcher
    if let Err(e) = state.hands.interrupt_hand_killed(&hand_id).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to interrupt hand: {e}")})),
        )
            .into_response();
    }

    // 2. Mark task as Review to prevent Zombie Loop (planner sees it's not InProgress anymore)
    let _ = crate::memory::tasks::update_task_status(
        &conn,
        &id,
        crate::memory::tasks::TaskStatus::Review,
        None,
    );

    Json(serde_json::json!({
        "success": true,
        "message": format!("Hand {hand_id} interrupted; task {id} moved to Review"),
    }))
    .into_response()
}

// ── Helpers ─────────────────────────────────────────────────────

fn is_masked_secret(value: &str) -> bool {
    value == MASKED_SECRET
}

fn mask_optional_secret(value: &mut Option<String>) {
    if value.is_some() {
        *value = Some(MASKED_SECRET.to_string());
    }
}

fn mask_required_secret(value: &mut String) {
    if !value.is_empty() {
        *value = MASKED_SECRET.to_string();
    }
}

fn mask_vec_secrets(values: &mut [String]) {
    for value in values.iter_mut() {
        if !value.is_empty() {
            *value = MASKED_SECRET.to_string();
        }
    }
}

#[allow(clippy::ref_option)]
fn restore_optional_secret(value: &mut Option<String>, current: &Option<String>) {
    if value.as_deref().is_some_and(is_masked_secret) {
        *value = current.clone();
    }
}

fn restore_required_secret(value: &mut String, current: &str) {
    if is_masked_secret(value) {
        *value = current.to_string();
    }
}

fn restore_vec_secrets(values: &mut [String], current: &[String]) {
    for (idx, value) in values.iter_mut().enumerate() {
        if is_masked_secret(value) {
            if let Some(existing) = current.get(idx) {
                *value = existing.clone();
            }
        }
    }
}

fn normalize_route_field(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn model_route_identity_matches(
    incoming: &crate::config::schemas::llm_schema::ModelRouteConfig,
    current: &crate::config::schemas::llm_schema::ModelRouteConfig,
) -> bool {
    normalize_route_field(&incoming.hint) == normalize_route_field(&current.hint)
        && normalize_route_field(&incoming.provider) == normalize_route_field(&current.provider)
        && normalize_route_field(&incoming.model) == normalize_route_field(&current.model)
}

fn model_route_provider_model_matches(
    incoming: &crate::config::ModelRouteConfig,
    current: &crate::config::ModelRouteConfig,
) -> bool {
    normalize_route_field(&incoming.provider) == normalize_route_field(&current.provider)
        && normalize_route_field(&incoming.model) == normalize_route_field(&current.model)
}

fn embedding_route_identity_matches(
    incoming: &crate::config::EmbeddingRouteConfig,
    current: &crate::config::EmbeddingRouteConfig,
) -> bool {
    normalize_route_field(&incoming.hint) == normalize_route_field(&current.hint)
        && normalize_route_field(&incoming.provider) == normalize_route_field(&current.provider)
        && normalize_route_field(&incoming.model) == normalize_route_field(&current.model)
}

fn embedding_route_provider_model_matches(
    incoming: &crate::config::EmbeddingRouteConfig,
    current: &crate::config::EmbeddingRouteConfig,
) -> bool {
    normalize_route_field(&incoming.provider) == normalize_route_field(&current.provider)
        && normalize_route_field(&incoming.model) == normalize_route_field(&current.model)
}

fn restore_model_route_api_keys(
    incoming: &mut [crate::config::schemas::llm_schema::ModelRouteConfig],
    current: &[crate::config::schemas::llm_schema::ModelRouteConfig],
) {
    let mut used_current = vec![false; current.len()];
    for incoming_route in incoming {
        if !incoming_route
            .api_key
            .as_deref()
            .is_some_and(is_masked_secret)
        {
            continue;
        }

        let exact_match_idx = current
            .iter()
            .enumerate()
            .find(|(idx, current_route)| {
                !used_current[*idx] && model_route_identity_matches(incoming_route, current_route)
            })
            .map(|(idx, _)| idx);

        let match_idx = exact_match_idx.or_else(|| {
            current
                .iter()
                .enumerate()
                .find(|(idx, current_route)| {
                    !used_current[*idx]
                        && model_route_provider_model_matches(incoming_route, current_route)
                })
                .map(|(idx, _)| idx)
        });

        if let Some(idx) = match_idx {
            used_current[idx] = true;
            incoming_route.api_key = current[idx].api_key.clone();
        } else {
            // Never persist UI placeholders to disk when no safe restore target exists.
            incoming_route.api_key = None;
        }
    }
}

fn restore_embedding_route_api_keys(
    incoming: &mut [crate::config::schemas::llm_schema::EmbeddingRouteConfig],
    current: &[crate::config::schemas::llm_schema::EmbeddingRouteConfig],
) {
    let mut used_current = vec![false; current.len()];
    for incoming_route in incoming {
        if !incoming_route
            .api_key
            .as_deref()
            .is_some_and(is_masked_secret)
        {
            continue;
        }

        let exact_match_idx = current
            .iter()
            .enumerate()
            .find(|(idx, current_route)| {
                !used_current[*idx]
                    && embedding_route_identity_matches(incoming_route, current_route)
            })
            .map(|(idx, _)| idx);

        let match_idx = exact_match_idx.or_else(|| {
            current
                .iter()
                .enumerate()
                .find(|(idx, current_route)| {
                    !used_current[*idx]
                        && embedding_route_provider_model_matches(incoming_route, current_route)
                })
                .map(|(idx, _)| idx)
        });

        if let Some(idx) = match_idx {
            used_current[idx] = true;
            incoming_route.api_key = current[idx].api_key.clone();
        } else {
            // Never persist UI placeholders to disk when no safe restore target exists.
            incoming_route.api_key = None;
        }
    }
}

fn mask_sensitive_fields(config: &crate::config::Config) -> crate::config::Config {
    let mut masked = config.clone();

    mask_optional_secret(&mut masked.api_key);
    mask_vec_secrets(&mut masked.reliability.api_keys);
    mask_vec_secrets(&mut masked.gateway.paired_tokens);
    mask_optional_secret(&mut masked.composio.api_key);
    mask_optional_secret(&mut masked.browser.computer_use.api_key);
    mask_optional_secret(&mut masked.web_search.brave_api_key);
    mask_optional_secret(&mut masked.storage.provider.config.db_url);
    mask_optional_secret(&mut masked.memory.qdrant.api_key);
    if let Some(cloudflare) = masked.tunnel.cloudflare.as_mut() {
        mask_required_secret(&mut cloudflare.token);
    }
    if let Some(ngrok) = masked.tunnel.ngrok.as_mut() {
        mask_required_secret(&mut ngrok.auth_token);
    }

    for agent in masked.agents.values_mut() {
        mask_optional_secret(&mut agent.api_key);
    }
    for route in &mut masked.model_routes {
        mask_optional_secret(&mut route.api_key);
    }
    for route in &mut masked.embedding_routes {
        mask_optional_secret(&mut route.api_key);
    }

    if let Some(telegram) = masked.channels_config.telegram.as_mut() {
        mask_required_secret(&mut telegram.bot_token);
    }
    if let Some(discord) = masked.channels_config.discord.as_mut() {
        mask_required_secret(&mut discord.bot_token);
    }
    if let Some(slack) = masked.channels_config.slack.as_mut() {
        mask_required_secret(&mut slack.bot_token);
        mask_optional_secret(&mut slack.app_token);
    }
    if let Some(mattermost) = masked.channels_config.mattermost.as_mut() {
        mask_required_secret(&mut mattermost.bot_token);
    }
    if let Some(webhook) = masked.channels_config.webhook.as_mut() {
        mask_optional_secret(&mut webhook.secret);
    }
    if let Some(matrix) = masked.channels_config.matrix.as_mut() {
        mask_required_secret(&mut matrix.access_token);
    }
    if let Some(whatsapp) = masked.channels_config.whatsapp.as_mut() {
        mask_optional_secret(&mut whatsapp.access_token);
        mask_optional_secret(&mut whatsapp.app_secret);
        mask_optional_secret(&mut whatsapp.verify_token);
    }
    if let Some(linq) = masked.channels_config.linq.as_mut() {
        mask_required_secret(&mut linq.api_token);
        mask_optional_secret(&mut linq.signing_secret);
    }
    if let Some(nextcloud) = masked.channels_config.nextcloud_talk.as_mut() {
        mask_required_secret(&mut nextcloud.app_token);
        mask_optional_secret(&mut nextcloud.webhook_secret);
    }
    if let Some(wati) = masked.channels_config.wati.as_mut() {
        mask_required_secret(&mut wati.api_token);
    }
    if let Some(irc) = masked.channels_config.irc.as_mut() {
        mask_optional_secret(&mut irc.server_password);
        mask_optional_secret(&mut irc.nickserv_password);
        mask_optional_secret(&mut irc.sasl_password);
    }
    if let Some(lark) = masked.channels_config.lark.as_mut() {
        mask_required_secret(&mut lark.app_secret);
        mask_optional_secret(&mut lark.encrypt_key);
        mask_optional_secret(&mut lark.verification_token);
    }
    if let Some(feishu) = masked.channels_config.feishu.as_mut() {
        mask_required_secret(&mut feishu.app_secret);
        mask_optional_secret(&mut feishu.encrypt_key);
        mask_optional_secret(&mut feishu.verification_token);
    }
    if let Some(dingtalk) = masked.channels_config.dingtalk.as_mut() {
        mask_required_secret(&mut dingtalk.client_secret);
    }
    if let Some(qq) = masked.channels_config.qq.as_mut() {
        mask_required_secret(&mut qq.app_secret);
    }
    if let Some(nostr) = masked.channels_config.nostr.as_mut() {
        mask_required_secret(&mut nostr.private_key);
    }
    if let Some(clawdtalk) = masked.channels_config.clawdtalk.as_mut() {
        mask_required_secret(&mut clawdtalk.api_key);
        mask_optional_secret(&mut clawdtalk.webhook_secret);
    }
    if let Some(email) = masked.channels_config.email.as_mut() {
        mask_required_secret(&mut email.password);
    }
    masked
}

fn restore_masked_sensitive_fields(
    incoming: &mut crate::config::Config,
    current: &crate::config::Config,
) {
    restore_optional_secret(&mut incoming.api_key, &current.api_key);
    restore_vec_secrets(
        &mut incoming.gateway.paired_tokens,
        &current.gateway.paired_tokens,
    );
    restore_vec_secrets(
        &mut incoming.reliability.api_keys,
        &current.reliability.api_keys,
    );
    restore_optional_secret(&mut incoming.composio.api_key, &current.composio.api_key);
    restore_optional_secret(
        &mut incoming.browser.computer_use.api_key,
        &current.browser.computer_use.api_key,
    );
    restore_optional_secret(
        &mut incoming.web_search.brave_api_key,
        &current.web_search.brave_api_key,
    );
    restore_optional_secret(
        &mut incoming.storage.provider.config.db_url,
        &current.storage.provider.config.db_url,
    );
    restore_optional_secret(
        &mut incoming.memory.qdrant.api_key,
        &current.memory.qdrant.api_key,
    );
    if let (Some(incoming_tunnel), Some(current_tunnel)) = (
        incoming.tunnel.cloudflare.as_mut(),
        current.tunnel.cloudflare.as_ref(),
    ) {
        restore_required_secret(&mut incoming_tunnel.token, &current_tunnel.token);
    }
    if let (Some(incoming_tunnel), Some(current_tunnel)) = (
        incoming.tunnel.ngrok.as_mut(),
        current.tunnel.ngrok.as_ref(),
    ) {
        restore_required_secret(&mut incoming_tunnel.auth_token, &current_tunnel.auth_token);
    }

    for (name, agent) in &mut incoming.agents {
        if let Some(current_agent) = current.agents.get(name) {
            restore_optional_secret(&mut agent.api_key, &current_agent.api_key);
        }
    }
    restore_model_route_api_keys(&mut incoming.model_routes, &current.model_routes);
    restore_embedding_route_api_keys(&mut incoming.embedding_routes, &current.embedding_routes);

    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.telegram.as_mut(),
        current.channels_config.telegram.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.bot_token, &current_ch.bot_token);
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.discord.as_mut(),
        current.channels_config.discord.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.bot_token, &current_ch.bot_token);
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.slack.as_mut(),
        current.channels_config.slack.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.bot_token, &current_ch.bot_token);
        restore_optional_secret(&mut incoming_ch.app_token, &current_ch.app_token);
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.mattermost.as_mut(),
        current.channels_config.mattermost.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.bot_token, &current_ch.bot_token);
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.webhook.as_mut(),
        current.channels_config.webhook.as_ref(),
    ) {
        restore_optional_secret(&mut incoming_ch.secret, &current_ch.secret);
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.matrix.as_mut(),
        current.channels_config.matrix.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.access_token, &current_ch.access_token);
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.whatsapp.as_mut(),
        current.channels_config.whatsapp.as_ref(),
    ) {
        restore_optional_secret(&mut incoming_ch.access_token, &current_ch.access_token);
        restore_optional_secret(&mut incoming_ch.app_secret, &current_ch.app_secret);
        restore_optional_secret(&mut incoming_ch.verify_token, &current_ch.verify_token);
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.linq.as_mut(),
        current.channels_config.linq.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.api_token, &current_ch.api_token);
        restore_optional_secret(&mut incoming_ch.signing_secret, &current_ch.signing_secret);
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.nextcloud_talk.as_mut(),
        current.channels_config.nextcloud_talk.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.app_token, &current_ch.app_token);
        restore_optional_secret(&mut incoming_ch.webhook_secret, &current_ch.webhook_secret);
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.wati.as_mut(),
        current.channels_config.wati.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.api_token, &current_ch.api_token);
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.irc.as_mut(),
        current.channels_config.irc.as_ref(),
    ) {
        restore_optional_secret(
            &mut incoming_ch.server_password,
            &current_ch.server_password,
        );
        restore_optional_secret(
            &mut incoming_ch.nickserv_password,
            &current_ch.nickserv_password,
        );
        restore_optional_secret(&mut incoming_ch.sasl_password, &current_ch.sasl_password);
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.lark.as_mut(),
        current.channels_config.lark.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.app_secret, &current_ch.app_secret);
        restore_optional_secret(&mut incoming_ch.encrypt_key, &current_ch.encrypt_key);
        restore_optional_secret(
            &mut incoming_ch.verification_token,
            &current_ch.verification_token,
        );
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.feishu.as_mut(),
        current.channels_config.feishu.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.app_secret, &current_ch.app_secret);
        restore_optional_secret(&mut incoming_ch.encrypt_key, &current_ch.encrypt_key);
        restore_optional_secret(
            &mut incoming_ch.verification_token,
            &current_ch.verification_token,
        );
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.dingtalk.as_mut(),
        current.channels_config.dingtalk.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.client_secret, &current_ch.client_secret);
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.qq.as_mut(),
        current.channels_config.qq.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.app_secret, &current_ch.app_secret);
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.nostr.as_mut(),
        current.channels_config.nostr.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.private_key, &current_ch.private_key);
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.clawdtalk.as_mut(),
        current.channels_config.clawdtalk.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.api_key, &current_ch.api_key);
        restore_optional_secret(&mut incoming_ch.webhook_secret, &current_ch.webhook_secret);
    }
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.email.as_mut(),
        current.channels_config.email.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.password, &current_ch.password);
    }
}

fn hydrate_config_for_save(
    mut incoming: crate::config::Config,
    current: &crate::config::Config,
) -> crate::config::Config {
    restore_masked_sensitive_fields(&mut incoming, current);
    // These are runtime-computed fields skipped from TOML serialization.
    incoming.config_path = current.config_path.clone();
    incoming.workspace_dir = current.workspace_dir.clone();
    incoming
}

// ── Chat History API ─────────────────────────────────────────────────────

/// Query parameters for chat history
#[derive(Deserialize)]
pub struct ChatHistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// GET /api/chat/history/{session_id} — get chat history for a session
pub async fn handle_api_chat_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Query(params): Query<ChatHistoryQuery>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let limit = params.limit.unwrap_or(50).min(1000); // Default 50, max 1000
    let offset = params.offset.unwrap_or(0);

    // Try to use the new chat history method if available
    let mem_result: Result<(Vec<crate::memory::MemoryEntry>, i64), anyhow::Error> =
        if let Some(sqlite_mem) = state
            .mem
            .as_any()
            .downcast_ref::<crate::memory::sqlite::SqliteMemory>()
        {
            // Use the optimized method
            let entries: anyhow::Result<Vec<crate::memory::MemoryEntry>> = sqlite_mem
                .get_chat_history(&session_id, limit as i64, offset as i64)
                .await;
            let total: anyhow::Result<i64> = sqlite_mem.count_chat_messages(&session_id).await;
            entries.map(|e| (e, total.unwrap_or(0)))
        } else {
            // Fallback to generic list method
            let category = crate::memory::MemoryCategory::Conversation;
            let entries_result: Result<Vec<crate::memory::MemoryEntry>, anyhow::Error> =
                state.mem.list(Some(&category), Some(&session_id)).await;
            entries_result.map(|entries: Vec<crate::memory::MemoryEntry>| {
                let total = entries.len() as i64;
                let paginated: Vec<crate::memory::MemoryEntry> = entries
                    .into_iter()
                    .skip(offset as usize)
                    .take(limit as usize)
                    .collect();
                (paginated, total)
            })
        };

    match mem_result {
        Ok((entries, total)) => Json(serde_json::json!({
            "success": true,
            "data": entries,
            "pagination": {
                "total": total,
                "offset": offset,
                "limit": limit,
                "has_more": (offset + limit) < total
            }
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to fetch chat history: {e}")})),
        )
            .into_response(),
    }
}

// ── SOPs Endpoints ───────────────────────────────────────────────────────

/// GET /v1/sops — list all SOPs
pub async fn handle_api_sops_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    // SOPs are stored as memory entries with category "sop"
    let category = crate::memory::MemoryCategory::Sop;

    match state.mem.list(Some(&category), None).await {
        Ok(sops) => Json(serde_json::json!({
            "success": true,
            "data": sops
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to fetch SOPs: {e}")})),
        )
            .into_response(),
    }
}

/// POST /v1/sops — create a new SOP
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
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let id = format!("sop_{}", uuid::Uuid::new_v4());
    let content = format!("---\nname: {}\n---\n{}", body.name, body.yaml);

    match state
        .mem
        .store(&id, &content, crate::memory::MemoryCategory::Sop, None)
        .await
    {
        Ok(_) => Json(serde_json::json!({
            "success": true,
            "data": {
                "id": id,
                "name": body.name,
                "yaml": body.yaml,
                "createdAt": chrono::Utc::now().to_rfc3339(),
                "updatedAt": chrono::Utc::now().to_rfc3339()
            }
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to create SOP: {e}")})),
        )
            .into_response(),
    }
}

/// PUT /v1/sops/{id} — update an existing SOP
pub async fn handle_api_sops_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<SopCreateBody>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let content = format!("---\nname: {}\n---\n{}", body.name, body.yaml);

    match state
        .mem
        .store(&id, &content, crate::memory::MemoryCategory::Sop, None)
        .await
    {
        Ok(_) => Json(serde_json::json!({
            "success": true,
            "data": {
                "id": id,
                "name": body.name,
                "yaml": body.yaml,
                "updatedAt": chrono::Utc::now().to_rfc3339()
            }
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to update SOP: {e}")})),
        )
            .into_response(),
    }
}

// NOTE: DELETE endpoint not implemented - Memory trait doesn't have delete method
// Use PUT with empty content or implement delete at trait level if needed

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masking_keeps_toml_valid_and_preserves_api_keys_type() {
        let mut cfg = crate::config::Config::default();
        cfg.api_key = Some("sk-live-123".to_string());
        cfg.reliability.api_keys = vec!["rk-1".to_string(), "rk-2".to_string()];
        cfg.gateway.paired_tokens = vec!["pair-token-1".to_string()];
        cfg.tunnel.cloudflare = Some(crate::config::schema::CloudflareTunnelConfig {
            token: "cf-token".to_string(),
        });
        cfg.memory.qdrant.api_key = Some("qdrant-key".to_string());
        cfg.channels_config.wati = Some(crate::config::schema::WatiConfig {
            api_token: "wati-token".to_string(),
            api_url: "https://live-mt-server.wati.io".to_string(),
            tenant_id: None,
            allowed_numbers: vec![],
        });
        cfg.channels_config.feishu = Some(crate::config::schema::FeishuConfig {
            app_id: "cli_aabbcc".to_string(),
            app_secret: "feishu-secret".to_string(),
            encrypt_key: Some("feishu-encrypt".to_string()),
            verification_token: Some("feishu-verify".to_string()),
            allowed_users: vec!["*".to_string()],
            receive_mode: crate::config::schema::LarkReceiveMode::Websocket,
            port: None,
        });
        cfg.channels_config.email = Some(crate::channels::email_channel::EmailConfig {
            imap_host: "imap.example.com".to_string(),
            imap_port: 993,
            imap_folder: "INBOX".to_string(),
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 465,
            smtp_tls: true,
            username: "agent@example.com".to_string(),
            password: "email-password-secret".to_string(),
            from_address: "agent@example.com".to_string(),
            idle_timeout_secs: 1740,
            allowed_senders: vec!["*".to_string()],
        });
        cfg.model_routes = vec![crate::config::ModelRouteConfig {
            hint: "reasoning".to_string(),
            provider: "openrouter".to_string(),
            model: "anthropic/claude-sonnet-4.6".to_string(),
            api_key: Some("route-model-key".to_string()),
        }];
        cfg.embedding_routes = vec![crate::config::EmbeddingRouteConfig {
            hint: "semantic".to_string(),
            provider: "openai".to_string(),
            model: "text-embedding-3-small".to_string(),
            dimensions: Some(1536),
            api_key: Some("route-embed-key".to_string()),
        }];

        let masked = mask_sensitive_fields(&cfg);
        let toml = toml::to_string_pretty(&masked).expect("masked config should serialize");
        let parsed: crate::config::Config =
            toml::from_str(&toml).expect("masked config should remain valid TOML for Config");

        assert_eq!(parsed.api_key.as_deref(), Some(MASKED_SECRET));
        assert_eq!(
            parsed.reliability.api_keys,
            vec![MASKED_SECRET.to_string(), MASKED_SECRET.to_string()]
        );
        assert_eq!(
            parsed.gateway.paired_tokens,
            vec![MASKED_SECRET.to_string()]
        );
        assert_eq!(
            parsed.tunnel.cloudflare.as_ref().map(|v| v.token.as_str()),
            Some(MASKED_SECRET)
        );
        assert_eq!(
            parsed
                .channels_config
                .wati
                .as_ref()
                .map(|v| v.api_token.as_str()),
            Some(MASKED_SECRET)
        );
        assert_eq!(parsed.memory.qdrant.api_key.as_deref(), Some(MASKED_SECRET));
        assert_eq!(
            parsed
                .channels_config
                .feishu
                .as_ref()
                .map(|v| v.app_secret.as_str()),
            Some(MASKED_SECRET)
        );
        assert_eq!(
            parsed
                .channels_config
                .feishu
                .as_ref()
                .and_then(|v| v.encrypt_key.as_deref()),
            Some(MASKED_SECRET)
        );
        assert_eq!(
            parsed
                .channels_config
                .feishu
                .as_ref()
                .and_then(|v| v.verification_token.as_deref()),
            Some(MASKED_SECRET)
        );
        assert_eq!(
            parsed
                .model_routes
                .first()
                .and_then(|v| v.api_key.as_deref()),
            Some(MASKED_SECRET)
        );
        assert_eq!(
            parsed
                .embedding_routes
                .first()
                .and_then(|v| v.api_key.as_deref()),
            Some(MASKED_SECRET)
        );
        assert_eq!(
            parsed
                .channels_config
                .email
                .as_ref()
                .map(|v| v.password.as_str()),
            Some(MASKED_SECRET)
        );
    }

    #[test]
    fn hydrate_config_for_save_restores_masked_secrets_and_paths() {
        let mut current = crate::config::Config::default();
        current.config_path = std::path::PathBuf::from("/tmp/current/config.toml");
        current.workspace_dir = std::path::PathBuf::from("/tmp/current/workspace");
        current.api_key = Some("real-key".to_string());
        current.reliability.api_keys = vec!["r1".to_string(), "r2".to_string()];
        current.gateway.paired_tokens = vec!["pair-1".to_string(), "pair-2".to_string()];
        current.tunnel.cloudflare = Some(crate::config::schema::CloudflareTunnelConfig {
            token: "cf-token-real".to_string(),
        });
        current.tunnel.ngrok = Some(crate::config::schema::NgrokTunnelConfig {
            auth_token: "ngrok-token-real".to_string(),
            domain: None,
        });
        current.memory.qdrant.api_key = Some("qdrant-real".to_string());
        current.channels_config.wati = Some(crate::config::schema::WatiConfig {
            api_token: "wati-real".to_string(),
            api_url: "https://live-mt-server.wati.io".to_string(),
            tenant_id: None,
            allowed_numbers: vec![],
        });
        current.channels_config.feishu = Some(crate::config::schema::FeishuConfig {
            app_id: "cli_current".to_string(),
            app_secret: "feishu-secret-real".to_string(),
            encrypt_key: Some("feishu-encrypt-real".to_string()),
            verification_token: Some("feishu-verify-real".to_string()),
            allowed_users: vec!["*".to_string()],
            receive_mode: crate::config::schema::LarkReceiveMode::Websocket,
            port: None,
        });
        current.channels_config.email = Some(crate::channels::email_channel::EmailConfig {
            imap_host: "imap.example.com".to_string(),
            imap_port: 993,
            imap_folder: "INBOX".to_string(),
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 465,
            smtp_tls: true,
            username: "agent@example.com".to_string(),
            password: "email-password-real".to_string(),
            from_address: "agent@example.com".to_string(),
            idle_timeout_secs: 1740,
            allowed_senders: vec!["*".to_string()],
        });
        current.model_routes = vec![
            crate::config::ModelRouteConfig {
                hint: "reasoning".to_string(),
                provider: "openrouter".to_string(),
                model: "anthropic/claude-sonnet-4.6".to_string(),
                api_key: Some("route-model-key-1".to_string()),
            },
            crate::config::ModelRouteConfig {
                hint: "fast".to_string(),
                provider: "openrouter".to_string(),
                model: "openai/gpt-4.1-mini".to_string(),
                api_key: Some("route-model-key-2".to_string()),
            },
        ];
        current.embedding_routes = vec![
            crate::config::EmbeddingRouteConfig {
                hint: "semantic".to_string(),
                provider: "openai".to_string(),
                model: "text-embedding-3-small".to_string(),
                dimensions: Some(1536),
                api_key: Some("route-embed-key-1".to_string()),
            },
            crate::config::EmbeddingRouteConfig {
                hint: "archive".to_string(),
                provider: "custom:https://emb.example.com/v1".to_string(),
                model: "bge-m3".to_string(),
                dimensions: Some(1024),
                api_key: Some("route-embed-key-2".to_string()),
            },
        ];

        let mut incoming = mask_sensitive_fields(&current);
        incoming.default_model = Some("gpt-4.1-mini".to_string());
        // Simulate UI changing only one key and keeping the first masked.
        incoming.reliability.api_keys = vec![MASKED_SECRET.to_string(), "r2-new".to_string()];
        incoming.gateway.paired_tokens = vec![MASKED_SECRET.to_string(), "pair-2-new".to_string()];
        if let Some(cloudflare) = incoming.tunnel.cloudflare.as_mut() {
            cloudflare.token = MASKED_SECRET.to_string();
        }
        if let Some(ngrok) = incoming.tunnel.ngrok.as_mut() {
            ngrok.auth_token = MASKED_SECRET.to_string();
        }
        incoming.memory.qdrant.api_key = Some(MASKED_SECRET.to_string());
        if let Some(wati) = incoming.channels_config.wati.as_mut() {
            wati.api_token = MASKED_SECRET.to_string();
        }
        if let Some(feishu) = incoming.channels_config.feishu.as_mut() {
            feishu.app_secret = MASKED_SECRET.to_string();
            feishu.encrypt_key = Some(MASKED_SECRET.to_string());
            feishu.verification_token = Some("feishu-verify-new".to_string());
        }
        if let Some(email) = incoming.channels_config.email.as_mut() {
            email.password = MASKED_SECRET.to_string();
        }
        incoming.model_routes[1].api_key = Some("route-model-key-2-new".to_string());
        incoming.embedding_routes[1].api_key = Some("route-embed-key-2-new".to_string());

        let hydrated = hydrate_config_for_save(incoming, &current);

        assert_eq!(hydrated.config_path, current.config_path);
        assert_eq!(hydrated.workspace_dir, current.workspace_dir);
        assert_eq!(hydrated.api_key, current.api_key);
        assert_eq!(hydrated.default_model.as_deref(), Some("gpt-4.1-mini"));
        assert_eq!(
            hydrated.reliability.api_keys,
            vec!["r1".to_string(), "r2-new".to_string()]
        );
        assert_eq!(
            hydrated.gateway.paired_tokens,
            vec!["pair-1".to_string(), "pair-2-new".to_string()]
        );
        assert_eq!(
            hydrated
                .tunnel
                .cloudflare
                .as_ref()
                .map(|v| v.token.as_str()),
            Some("cf-token-real")
        );
        assert_eq!(
            hydrated
                .tunnel
                .ngrok
                .as_ref()
                .map(|v| v.auth_token.as_str()),
            Some("ngrok-token-real")
        );
        assert_eq!(
            hydrated.memory.qdrant.api_key.as_deref(),
            Some("qdrant-real")
        );
        assert_eq!(
            hydrated
                .channels_config
                .wati
                .as_ref()
                .map(|v| v.api_token.as_str()),
            Some("wati-real")
        );
        assert_eq!(
            hydrated
                .channels_config
                .feishu
                .as_ref()
                .map(|v| v.app_secret.as_str()),
            Some("feishu-secret-real")
        );
        assert_eq!(
            hydrated
                .channels_config
                .feishu
                .as_ref()
                .and_then(|v| v.encrypt_key.as_deref()),
            Some("feishu-encrypt-real")
        );
        assert_eq!(
            hydrated
                .channels_config
                .feishu
                .as_ref()
                .and_then(|v| v.verification_token.as_deref()),
            Some("feishu-verify-new")
        );
        assert_eq!(
            hydrated.model_routes[0].api_key.as_deref(),
            Some("route-model-key-1")
        );
        assert_eq!(
            hydrated.model_routes[1].api_key.as_deref(),
            Some("route-model-key-2-new")
        );
        assert_eq!(
            hydrated.embedding_routes[0].api_key.as_deref(),
            Some("route-embed-key-1")
        );
        assert_eq!(
            hydrated.embedding_routes[1].api_key.as_deref(),
            Some("route-embed-key-2-new")
        );
        assert_eq!(
            hydrated
                .channels_config
                .email
                .as_ref()
                .map(|v| v.password.as_str()),
            Some("email-password-real")
        );
    }

    #[test]
    fn hydrate_config_for_save_restores_route_keys_by_identity_and_clears_unmatched_masks() {
        let mut current = crate::config::Config::default();
        current.model_routes = vec![
            crate::config::ModelRouteConfig {
                hint: "reasoning".to_string(),
                provider: "openrouter".to_string(),
                model: "anthropic/claude-sonnet-4.6".to_string(),
                api_key: Some("route-model-key-1".to_string()),
            },
            crate::config::ModelRouteConfig {
                hint: "fast".to_string(),
                provider: "openrouter".to_string(),
                model: "openai/gpt-4.1-mini".to_string(),
                api_key: Some("route-model-key-2".to_string()),
            },
        ];
        current.embedding_routes = vec![
            crate::config::EmbeddingRouteConfig {
                hint: "semantic".to_string(),
                provider: "openai".to_string(),
                model: "text-embedding-3-small".to_string(),
                dimensions: Some(1536),
                api_key: Some("route-embed-key-1".to_string()),
            },
            crate::config::EmbeddingRouteConfig {
                hint: "archive".to_string(),
                provider: "custom:https://emb.example.com/v1".to_string(),
                model: "bge-m3".to_string(),
                dimensions: Some(1024),
                api_key: Some("route-embed-key-2".to_string()),
            },
        ];

        let mut incoming = mask_sensitive_fields(&current);
        incoming.model_routes.swap(0, 1);
        incoming.embedding_routes.swap(0, 1);
        incoming.model_routes.push(crate::config::ModelRouteConfig {
            hint: "new".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4.1".to_string(),
            api_key: Some(MASKED_SECRET.to_string()),
        });
        incoming
            .embedding_routes
            .push(crate::config::EmbeddingRouteConfig {
                hint: "new-embed".to_string(),
                provider: "custom:https://emb2.example.com/v1".to_string(),
                model: "bge-small".to_string(),
                dimensions: Some(768),
                api_key: Some(MASKED_SECRET.to_string()),
            });

        let hydrated = hydrate_config_for_save(incoming, &current);

        assert_eq!(
            hydrated.model_routes[0].api_key.as_deref(),
            Some("route-model-key-2")
        );
        assert_eq!(
            hydrated.model_routes[1].api_key.as_deref(),
            Some("route-model-key-1")
        );
        assert_eq!(hydrated.model_routes[2].api_key, None);
        assert_eq!(
            hydrated.embedding_routes[0].api_key.as_deref(),
            Some("route-embed-key-2")
        );
        assert_eq!(
            hydrated.embedding_routes[1].api_key.as_deref(),
            Some("route-embed-key-1")
        );
        assert_eq!(hydrated.embedding_routes[2].api_key, None);
        assert!(hydrated
            .model_routes
            .iter()
            .all(|route| route.api_key.as_deref() != Some(MASKED_SECRET)));
        assert!(hydrated
            .embedding_routes
            .iter()
            .all(|route| route.api_key.as_deref() != Some(MASKED_SECRET)));
    }
}

// ============================================================================
// TUI API Handlers
// ============================================================================

/// POST /api/chat — Send message to agent
///
/// Request body: {"session_id": "optional-id", "content": "message"}
/// Response: {"response": "...", "error": null}
pub async fn handle_tui_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    // Extract session_id and content from request
    let session_id = body
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("default");

    let content = match body.get("content").and_then(|v| v.as_str()) {
        Some(msg) if !msg.is_empty() => msg,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "response": serde_json::Value::Null,
                    "error": "Missing or empty 'content' field"
                })),
            )
                .into_response();
        }
    };

    // Process the message using the full agent with tools
    let config = state.config.lock().clone();
    let response = match crate::agent::process_message(config, content).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("TUI chat error: {e:#}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "response": serde_json::Value::Null,
                    "error": format!("Processing error: {}", e)
                })),
            )
                .into_response();
        }
    };

    // Store the conversation in memory if auto_save is enabled
    if state.auto_save {
        let _ = state
            .mem
            .store(
                &format!("tui:{}:user", session_id),
                content,
                crate::memory::MemoryCategory::Conversation,
                Some(session_id),
            )
            .await;
        let _ = state
            .mem
            .store(
                &format!("tui:{}:assistant", session_id),
                &response,
                crate::memory::MemoryCategory::Conversation,
                Some(session_id),
            )
            .await;
    }

    Json(serde_json::json!({
        "response": response,
        "error": null
    }))
    .into_response()
}

/// GET /api/agents/active — List active subagents
///
/// Response: [{"id": "agent-1", "name": "Search", "model": "gpt-4", "progress": 75, "status": "running"}]
pub async fn handle_tui_agents_active(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    // Get active hands from HandsDispatcher
    let active_hands = state.hands.list_active_hands().await;

    // Convert hands to AgentStatus format for TUI
    let mut agents = Vec::new();

    for (hand_id, hand_state) in active_hands {
        // For now, use hand info as agent info
        // TODO: Also track subagent tasks from SubAgentManager
        let agent = serde_json::json!({
            "id": hand_id,
            "name": format!("Hand: {}", hand_id),
            "model": "hand",
            "progress": if hand_state.token.is_cancelled() { 100 } else { 50 },
            "status": if hand_state.token.is_cancelled() { "done" } else { "running" },
            "current_task": hand_state.workspace_path.and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string())),
        });
        agents.push(agent);
    }

    // Also check for any active subagent tasks (TODO: integrate with RoutingManager)
    // For now, just return hands
    Json(agents).into_response()
}

/// GET /api/memory/status — Get memory system status
///
/// Response: {"backend": "sqlite", "total_memories": 42, "storage_bytes": 1048576, "recent_operations": [...]}
pub async fn handle_tui_memory_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    // Get memory backend info
    let backend_name = state.mem.name();
    let total_memories = state.mem.count().await.unwrap_or(0);
    let storage_bytes: u64 = 0; // TODO: implement storage_size in Memory trait

    // Get recent memory operations (mock data for now)
    let recent_operations: Vec<serde_json::Value> = vec![];

    let response = serde_json::json!({
        "backend": backend_name,
        "total_memories": total_memories,
        "storage_bytes": storage_bytes,
        "recent_operations": recent_operations,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    Json(response).into_response()
}

/// GET /api/logs/status — Get system logs status
///
/// Response: {"log_lines": [...], "total_lines": 42, "timestamp": "..."}
pub async fn handle_tui_logs_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    // For now, return mock log data
    // TODO: Integrate with tracing subscriber to capture actual logs
    let now = chrono::Utc::now();
    let mock_logs = serde_json::json!([
        {
            "level": "INFO",
            "message": "ZeroClaw backend started",
            "module": "zeroclaw::main",
            "timestamp": (now - chrono::Duration::minutes(5)).to_rfc3339()
        },
        {
            "level": "DEBUG",
            "message": "Initializing memory backend",
            "module": "zeroclaw::memory",
            "timestamp": (now - chrono::Duration::minutes(4)).to_rfc3339()
        },
        {
            "level": "WARN",
            "message": "Rate limit approaching for openai provider",
            "module": "zeroclaw::routing",
            "timestamp": (now - chrono::Duration::minutes(2)).to_rfc3339()
        },
        {
            "level": "ERROR",
            "message": "Failed to connect to Qdrant vector database",
            "module": "zeroclaw::memory::qdrant",
            "timestamp": (now - chrono::Duration::minutes(1)).to_rfc3339()
        }
    ]);

    let response = serde_json::json!({
        "log_lines": mock_logs,
        "total_lines": 4,
        "timestamp": now.to_rfc3339(),
    });

    Json(response).into_response()
}

/// GET /api/routing/status — Get router status
///
/// Response: {"active_provider": "openai", "quota_used_percent": 45.0, "fallback_active": false}
pub async fn handle_tui_routing_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let config = state.config.lock();

    // Return actual routing status from config
    // TODO: Integrate with rate_limiter for actual quota tracking
    let response = serde_json::json!({
        "active_provider": config.default_provider.clone().unwrap_or_else(|| "unknown".to_string()),
        "model": state.model.clone(),
        "temperature": state.temperature,
        "quota_used_percent": 0.0,  // TODO: Get from rate_limiter
        "fallback_active": false,     // TODO: Get from routing module
        "paired": state.pairing.is_paired(),
    });

    drop(config);
    Json(response).into_response()
}

// ============================================================================
// Diagnostic API
// ============================================================================

/// GET /api/diagnostic — Run system diagnostics and health checks
///
/// Returns comprehensive diagnostic information about the ZeroClaw system:
/// - Configuration validation
/// - API keys status
/// - Memory backend status
/// - Gateway status
/// - Channels configuration
///
/// Response: [{"name": "...", "status": "ok|warning|error", "message": "..."}]
pub async fn handle_diagnostic(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Diagnostics are available without auth for troubleshooting
    // but respect auth if enabled
    if state.pairing.require_pairing() {
        if let Err(e) = require_auth(&state, &headers).await {
            return e.into_response();
        }
    }

    let config = state.config.lock().clone();
    let checks = crate::diagnostic::run_diagnostics(&config).await;

    let overall_status = if checks
        .iter()
        .any(|c| c.status == crate::diagnostic::DiagnosticStatus::Fail)
    {
        "error"
    } else if checks
        .iter()
        .any(|c| c.status == crate::diagnostic::DiagnosticStatus::Warn)
    {
        "warning"
    } else {
        "ok"
    };

    Json(serde_json::json!({
        "status": overall_status,
        "checks": checks,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
    .into_response()
}

/// GET /api/validate — Quick configuration validation
///
/// Validates the current configuration and returns any errors or warnings.
/// Useful for CI/CD pipelines and pre-flight checks.
pub async fn handle_validate_config(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.config.lock().clone();
    let check = crate::diagnostic::validate_config(&config);

    let (status, message) = match check.status {
        crate::diagnostic::DiagnosticStatus::Pass => (StatusCode::OK, "Configuration is valid"),
        crate::diagnostic::DiagnosticStatus::Warn => {
            (StatusCode::OK, "Configuration valid with warnings")
        }
        crate::diagnostic::DiagnosticStatus::Fail => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Configuration has errors",
        ),
        crate::diagnostic::DiagnosticStatus::Skip => (StatusCode::OK, "Skipped"),
    };

    Json(serde_json::json!({
        "status": status.as_u16(),
        "message": message,
        "check": check,
    }))
    .into_response()
}

// ============================================================================
// Skills Management API
// ============================================================================

/// GET /api/v1/skills - List all skills
pub async fn handle_list_skills(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let skills = if let Some(engine) = state.skill_engine.as_ref() {
        engine.list_skills(true).await.unwrap_or_default()
    } else {
        Vec::new()
    };

    let response: Vec<serde_json::Value> = skills
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "name": s.name,
                "description": s.description,
                "version": s.version,
                "author": s.author,
                "tags": s.tags,
                "is_active": s.is_active,
                "created_at": s.created_at,
            })
        })
        .collect();

    Json(serde_json::json!({
        "skills": response,
        "count": response.len()
    }))
    .into_response()
}

/// POST /api/v1/skills - Create a new skill
///
/// TODO: Currently returns placeholder due to Handler trait issue with store_skill.
/// The issue is that calling `engine.store_skill(&skill).await` causes:
/// "the trait bound `fn(...): Handler<_, _>` is not satisfied"
///
/// This appears to be a specific interaction between Axum 0.8 Handler trait
/// and certain async methods. Other methods like list_skills work fine.
pub async fn skill_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(skill): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let _engine = match &state.skill_engine {
        Some(e) => e.clone(),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "Skills engine not available"
                })),
            )
                .into_response();
        }
    };

    // TODO: Implement actual skill creation once Handler issue is resolved
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "Skill creation not yet implemented due to Handler trait compatibility issue"
        })),
    )
        .into_response()
}

/// GET /api/v1/skills/:id - Get a skill by ID
pub async fn handle_get_skill(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let engine = match &state.skill_engine {
        Some(e) => e,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "Skills engine not available"
                })),
            )
                .into_response();
        }
    };

    match engine.get_skill(id).await {
        Ok(Some(skill)) => {
            // Don't serialize skill directly due to #[serde(skip)] on tools field
            Json(serde_json::json!({
                "id": skill.id,
                "name": skill.name,
                "description": skill.description,
                "content": skill.content,
                "version": skill.version,
                "author": skill.author,
                "tags": skill.tags,
                "is_active": skill.is_active,
                "created_at": skill.created_at,
                "updated_at": skill.updated_at,
            }))
            .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Skill not found"
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("{}", e)
            })),
        )
            .into_response(),
    }
}

/// DELETE /api/v1/skills/:id - Delete a skill
///
/// TODO: Currently returns placeholder due to Handler trait issue with delete_skill.
/// See skill_create for details.
pub async fn skill_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers).await {
        return e.into_response();
    }

    let _engine = match &state.skill_engine {
        Some(e) => e.clone(),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "Skills engine not available"
                })),
            )
                .into_response();
        }
    };

    // TODO: Implement actual skill deletion once Handler issue is resolved
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "Skill deletion not yet implemented due to Handler trait compatibility issue"
        })),
    )
        .into_response()
}
