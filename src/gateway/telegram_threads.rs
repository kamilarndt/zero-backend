// Zero-Bloat Telegram Threads REST API
// Thread management for Telegram TMA integration

use axum::{
    extract::{Json, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Global in-memory storage for threads
fn global_db() -> &'static DbAdapter {
    static DB: OnceLock<DbAdapter> = OnceLock::new();
    DB.get_or_init(|| DbAdapter::new())
}

/// Get the enabled skills for a Telegram chat (public API for telegram channel)
pub async fn get_skills_for_telegram_chat(chat_id: &str) -> Vec<String> {
    global_db().get_skills_for_chat(chat_id).await
}

/// Associate a Telegram chat with a thread (public API for telegram channel)
pub async fn associate_telegram_chat_with_thread(
    chat_id: &str,
    thread_id: &str,
) -> anyhow::Result<()> {
    global_db()
        .associate_chat_with_thread(chat_id, thread_id)
        .await
}

/// Get the active thread ID for a Telegram chat (public API for telegram channel)
pub async fn get_thread_id_for_telegram_chat(chat_id: &str) -> Option<String> {
    global_db().get_thread_for_chat(chat_id).await
}

/// Thread response
#[derive(Debug, Serialize, Clone)]
pub struct ThreadResponse {
    pub id: String,
    pub title: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub active_skills: Vec<String>,
}

/// Create thread request
#[derive(Debug, Deserialize)]
pub struct CreateThreadRequest {
    pub title: String,
    /// Optional Telegram chat_id to associate with this thread
    pub chat_id: Option<String>,
}

/// Update skills request
#[derive(Debug, Deserialize)]
pub struct UpdateSkillsRequest {
    pub skills: Vec<String>,
}

/// Set active thread request
#[derive(Debug, Deserialize)]
pub struct SetActiveThreadRequest {
    pub thread_id: String,
}

/// Generic API response
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Skill response for TMA
#[derive(Debug, Serialize)]
pub struct SkillResponse {
    pub name: String,
    pub description: String,
    pub category: String,
}

/// Get available skills for TMA
pub async fn get_available_skills() -> impl IntoResponse {
    // Return a hardcoded list of available skills for now
    // TODO: Load from ~/.zeroclaw/workspace/skills/
    let skills = vec![
        SkillResponse {
            name: "web-search".to_string(),
            description: "Search the web for information".to_string(),
            category: "Research".to_string(),
        },
        SkillResponse {
            name: "code-execution".to_string(),
            description: "Execute code in a sandboxed environment".to_string(),
            category: "Development".to_string(),
        },
        SkillResponse {
            name: "file-operations".to_string(),
            description: "Read and write files".to_string(),
            category: "System".to_string(),
        },
        SkillResponse {
            name: "memory".to_string(),
            description: "Store and retrieve information across conversations".to_string(),
            category: "Memory".to_string(),
        },
    ];

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "data": skills
        })),
    )
}

/// Create Telegram threads router
pub fn telegram_threads_router() -> Router {
    Router::new()
        .route("/", get(get_threads))
        .route("/", post(create_thread))
        .route("/:id/skills", put(update_thread_skills))
        .route("/active", post(set_active_thread))
}

/// Get all threads for the authenticated user
pub async fn get_threads() -> impl IntoResponse {
    // TODO: Extract user_id from JWT token in headers
    let db = global_db();

    match db.get_threads_for_user("user_placeholder").await {
        Ok(threads) => {
            let responses: Vec<ThreadResponse> = threads
                .into_iter()
                .map(|t| ThreadResponse {
                    id: t.id,
                    title: t.title,
                    is_active: t.is_active,
                    created_at: t.created_at,
                    updated_at: t.updated_at,
                    active_skills: t.active_skills,
                })
                .collect();

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "success": true,
                    "data": responses
                })),
            )
        }
        Err(e) => {
            tracing::error!("Failed to get threads: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "data": null,
                    "error": "Failed to fetch threads"
                })),
            )
        }
    }
}

/// Create a new thread
pub async fn create_thread(Json(payload): Json<CreateThreadRequest>) -> impl IntoResponse {
    // TODO: Extract user_id from JWT token in headers
    let db = global_db();

    match db.create_thread("user_placeholder", &payload.title).await {
        Ok(thread_id) => {
            // Set the new thread as active
            let _ = db.set_active_thread("user_placeholder", &thread_id).await;

            // Associate with chat_id if provided
            if let Some(ref chat_id) = payload.chat_id {
                let _ = db.associate_chat_with_thread(chat_id, &thread_id).await;
                tracing::info!("Associated chat {} with thread {}", chat_id, thread_id);
            }

            // Return the thread_id for frontend to use
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "success": true,
                    "data": thread_id
                })),
            )
        }
        Err(e) => {
            tracing::error!("Failed to create thread: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "data": null,
                    "error": "Failed to create thread"
                })),
            )
        }
    }
}

/// Update skills for a thread
pub async fn update_thread_skills(
    Path(id): Path<String>,
    Json(payload): Json<UpdateSkillsRequest>,
) -> impl IntoResponse {
    let db = global_db();

    match db.set_thread_skills(&id, &payload.skills).await {
        Ok(_) => {
            // Fetch and return updated threads
            match db.get_threads_for_user("user_placeholder").await {
                Ok(threads) => {
                    let responses: Vec<ThreadResponse> = threads
                        .into_iter()
                        .map(|t| ThreadResponse {
                            id: t.id,
                            title: t.title,
                            is_active: t.is_active,
                            created_at: t.created_at,
                            updated_at: t.updated_at,
                            active_skills: t.active_skills,
                        })
                        .collect();

                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "success": true,
                            "data": responses
                        })),
                    )
                }
                Err(_) => {
                    let empty: Vec<ThreadResponse> = vec![];
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "success": true,
                            "data": empty
                        })),
                    )
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to update thread skills: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "data": null,
                    "error": "Failed to update skills"
                })),
            )
        }
    }
}

/// Set the active thread
pub async fn set_active_thread(Json(payload): Json<SetActiveThreadRequest>) -> impl IntoResponse {
    let db = global_db();

    match db
        .set_active_thread("user_placeholder", &payload.thread_id)
        .await
    {
        Ok(_) => {
            // Fetch and return updated threads
            match db.get_threads_for_user("user_placeholder").await {
                Ok(threads) => {
                    let responses: Vec<ThreadResponse> = threads
                        .into_iter()
                        .map(|t| ThreadResponse {
                            id: t.id,
                            title: t.title,
                            is_active: t.is_active,
                            created_at: t.created_at,
                            updated_at: t.updated_at,
                            active_skills: t.active_skills,
                        })
                        .collect();

                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "success": true,
                            "data": responses
                        })),
                    )
                }
                Err(_) => {
                    let empty: Vec<ThreadResponse> = vec![];
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "success": true,
                            "data": empty
                        })),
                    )
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to set active thread: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "data": null,
                    "error": "Failed to set active thread"
                })),
            )
        }
    }
}

/// App state placeholder
#[derive(Clone)]
pub struct AppState {
    pub db: DbAdapter,
}

/// In-memory thread storage
#[derive(Clone)]
pub struct DbAdapter {
    threads: Arc<Mutex<HashMap<String, Thread>>>,
    active_thread: Arc<Mutex<Option<String>>>,
    /// Mapping from Telegram chat_id to thread_id (e.g., "123456789" -> "thread_1742345678")
    chat_to_thread: Arc<Mutex<HashMap<String, String>>>,
}

impl Default for DbAdapter {
    fn default() -> Self {
        Self {
            threads: Arc::new(Mutex::new(HashMap::new())),
            active_thread: Arc::new(Mutex::new(None)),
            chat_to_thread: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl DbAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get_threads_for_user(&self, _user_id: &str) -> anyhow::Result<Vec<Thread>> {
        let threads = self.threads.lock().unwrap();
        let active = self.active_thread.lock().unwrap();
        Ok(threads
            .values()
            .cloned()
            .map(|mut t| {
                t.is_active = active.as_ref() == Some(&t.id);
                t
            })
            .collect())
    }

    pub async fn create_thread(&self, _user_id: &str, title: &str) -> anyhow::Result<String> {
        let thread_id = format!(
            "thread_{}",
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
        );
        let now = format!(
            "{:?}",
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
        );

        let thread = Thread {
            id: thread_id.clone(),
            title: title.to_string(),
            is_active: false,
            created_at: now.clone(),
            updated_at: Some(now),
            active_skills: vec![],
        };

        self.threads
            .lock()
            .unwrap()
            .insert(thread_id.clone(), thread);
        Ok(thread_id)
    }

    pub async fn set_thread_skills(
        &self,
        thread_id: &str,
        skills: &[String],
    ) -> anyhow::Result<bool> {
        let mut threads = self.threads.lock().unwrap();
        if let Some(thread) = threads.get_mut(thread_id) {
            thread.active_skills = skills.to_vec();
            let now = format!(
                "{:?}",
                SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
            );
            thread.updated_at = Some(now);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn set_active_thread(&self, _user_id: &str, thread_id: &str) -> anyhow::Result<bool> {
        let mut active = self.active_thread.lock().unwrap();
        *active = Some(thread_id.to_string());
        Ok(true)
    }

    /// Associate a Telegram chat_id with a thread_id
    pub async fn associate_chat_with_thread(
        &self,
        chat_id: &str,
        thread_id: &str,
    ) -> anyhow::Result<()> {
        let mut chat_to_thread = self.chat_to_thread.lock().unwrap();
        chat_to_thread.insert(chat_id.to_string(), thread_id.to_string());
        Ok(())
    }

    /// Get the active thread ID for a given Telegram chat_id
    pub async fn get_thread_for_chat(&self, chat_id: &str) -> Option<String> {
        let chat_to_thread = self.chat_to_thread.lock().unwrap();
        chat_to_thread.get(chat_id).cloned()
    }

    /// Get the skills enabled for a thread
    pub async fn get_skills_for_thread(&self, thread_id: &str) -> Vec<String> {
        let threads = self.threads.lock().unwrap();
        threads
            .get(thread_id)
            .map(|t| t.active_skills.clone())
            .unwrap_or_default()
    }

    /// Get the skills enabled for a Telegram chat (via thread association)
    pub async fn get_skills_for_chat(&self, chat_id: &str) -> Vec<String> {
        if let Some(thread_id) = self.get_thread_for_chat(chat_id).await {
            self.get_skills_for_thread(&thread_id).await
        } else {
            vec![]
        }
    }

    /// Set the active thread for a specific chat (for TMA Hub integration)
    pub async fn set_active_thread_for_chat(
        &self,
        chat_id: &str,
        thread_id: &str,
    ) -> anyhow::Result<()> {
        // First set as the global active thread
        self.set_active_thread("user_placeholder", thread_id)
            .await?;
        // Then associate the chat with this thread
        self.associate_chat_with_thread(chat_id, thread_id).await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Thread {
    pub id: String,
    pub title: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub active_skills: Vec<String>,
}
