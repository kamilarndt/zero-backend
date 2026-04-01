//! Gateway state management.
//!
//! This module provides:
//! - AppState: Shared state for all axum handlers
//! - WsSession: WebSocket session data
//! - Memory key helpers for different channel types

use crate::channels::{Channel, LinqChannel, NextcloudTalkChannel, WatiChannel, WhatsAppChannel};
use crate::config::schema::SkillsConfig;
use crate::config::Config;
use crate::cost::CostTracker;
use crate::memory::Memory;
use crate::providers::ChatMessage;
use crate::runtime;
use crate::security::pairing::PairingGuard;
use crate::security::SecurityPolicy;
use crate::skills::{SkillEvaluator, SkillsEngine, VectorSkillLoader};
use crate::tools::traits::ToolSpec;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{extract::ConnectInfo, Json};

/// WebSocket session data (shared between ws.rs and gateway state)
#[derive(Clone)]
pub struct WsSession {
    pub id: String,
    pub chat_history: Vec<ChatMessage>,
}

/// Memory key helpers for different channel types
pub fn webhook_memory_key() -> String {
    format!("webhook_msg_{}", Uuid::new_v4())
}

pub fn whatsapp_memory_key(msg: &crate::channels::traits::ChannelMessage) -> String {
    format!("whatsapp_{}_{}", msg.sender, msg.id)
}

pub fn linq_memory_key(msg: &crate::channels::traits::ChannelMessage) -> String {
    format!("linq_{}_{}", msg.sender, msg.id)
}

pub fn wati_memory_key(msg: &crate::channels::traits::ChannelMessage) -> String {
    format!("wati_{}_{}", msg.sender, msg.id)
}

pub fn nextcloud_talk_memory_key(msg: &crate::channels::traits::ChannelMessage) -> String {
    format!("nextcloud_talk_{}_{}", msg.sender, msg.id)
}

/// Sliding window used by gateway rate limiting.
pub const RATE_LIMIT_WINDOW_SECS: u64 = 60;
/// Fallback max distinct client keys tracked in gateway rate limiter.
pub const RATE_LIMIT_MAX_KEYS_DEFAULT: usize = 10_000;
/// Fallback max distinct idempotency keys retained in gateway memory.
pub const IDEMPOTENCY_MAX_KEYS_DEFAULT: usize = 10_000;

/// Normalize max keys configuration with fallback value
pub fn normalize_max_keys(configured: usize, fallback: usize) -> usize {
    if configured == 0 {
        fallback.max(1)
    } else {
        configured
    }
}

/// Shared state for all axum handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub provider: Arc<dyn crate::providers::Provider>,
    pub model: String,
    pub temperature: f64,
    pub mem: Arc<dyn Memory>,
    pub auto_save: bool,
    /// SHA-256 hash of `X-Webhook-Secret` (hex-encoded), never plaintext.
    pub webhook_secret_hash: Option<Arc<str>>,
    pub pairing: Arc<PairingGuard>,
    pub trust_forwarded_headers: bool,
    pub rate_limiter: Arc<crate::gateway::middleware::GatewayRateLimiter>,
    pub idempotency_store: Arc<crate::gateway::middleware::IdempotencyStore>,
    pub whatsapp: Option<Arc<WhatsAppChannel>>,
    /// `WhatsApp` app secret for webhook signature verification (`X-Hub-Signature-256`)
    pub whatsapp_app_secret: Option<Arc<str>>,
    pub linq: Option<Arc<LinqChannel>>,
    /// Linq webhook signing secret for signature verification
    pub linq_signing_secret: Option<Arc<str>>,
    pub nextcloud_talk: Option<Arc<NextcloudTalkChannel>>,
    /// Nextcloud Talk webhook secret for signature verification
    pub nextcloud_talk_webhook_secret: Option<Arc<str>>,
    pub wati: Option<Arc<WatiChannel>>,
    /// Observability backend for metrics scraping
    pub observer: Arc<dyn crate::observability::Observer>,
    /// Registered tool specs (for web dashboard tools page)
    pub tools_registry: Arc<Vec<ToolSpec>>,
    /// Hands dispatcher for agent task management
    pub hands: Arc<crate::agent::hands::HandsDispatcher>,
    /// Cost tracker (optional, for web dashboard cost page)
    pub cost_tracker: Option<Arc<CostTracker>>,
    /// SSE broadcast channel for real-time events
    pub event_tx: tokio::sync::broadcast::Sender<serde_json::Value>,
    /// Workspace directory for task database access
    pub workspace_dir: Option<std::path::PathBuf>,
    /// JWT secret for token verification
    pub jwt_secret: Arc<[u8]>,
    // Skills Engine v2.0 (optional, requires Qdrant)
    pub skill_engine: Option<Arc<SkillsEngine>>,
    pub skill_loader: Option<Arc<VectorSkillLoader>>,
    pub skill_evaluator: Option<Arc<SkillEvaluator>>,
    /// Task classifier for intelligent model routing (zeroclaw-auto-router)
    pub classifier: Option<Arc<crate::routing::Classifier>>,
    /// Skills configuration (optional, from config.skills)
    pub skills_config: Option<SkillsConfig>,
    /// WebSocket session storage (session_id -> session data)
    pub sessions: Arc<Mutex<HashMap<String, WsSession>>>,
}

impl AppState {
    /// Extract client key from request for rate limiting
    pub fn client_key_from_request(
        &self,
        peer_addr: Option<SocketAddr>,
        headers: &HeaderMap,
    ) -> String {
        if self.trust_forwarded_headers {
            if let Some(ip) = crate::gateway::middleware::forwarded_client_ip(headers) {
                return ip.to_string();
            }
        }

        peer_addr
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

/// Response helpers for handlers
#[derive(serde::Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: i64,
}

#[derive(serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn error_response(status: StatusCode, message: impl Into<String>) -> impl IntoResponse {
    (
        status,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_keys() {
        let msg = crate::channels::traits::ChannelMessage {
            id: "test-id".to_string(),
            sender: "test-sender".to_string(),
            content: "test".to_string(),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        let wa_key = whatsapp_memory_key(&msg);
        assert!(wa_key.starts_with("whatsapp_test-sender_"));

        let linq_key = linq_memory_key(&msg);
        assert!(linq_key.starts_with("linq_test-sender_"));
    }

    #[test]
    fn test_normalize_max_keys() {
        assert_eq!(normalize_max_keys(0, 100), 100);
        assert_eq!(normalize_max_keys(50, 100), 50);
        assert_eq!(normalize_max_keys(0, 0), 1);
    }
}
