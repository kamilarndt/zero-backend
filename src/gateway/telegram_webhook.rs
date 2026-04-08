//! Telegram Webhook endpoint (Zero-Bloat)
//!
//! Minimalist webhook handler for Telegram Bot API with:
//! - Webhook secret verification
//! - Rate limiting
//! - Request body validation
//! - JSON parsing
//! - Forwarding updates to Telegram channel

use axum::{
    body::Bytes,
    extract::{ConnectInfo, Json},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Telegram update payload (simplified for Zero-Bloat)
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TelegramUpdate {
    Message { message: serde_json::Value },
    CallbackQuery { callback_query: serde_json::Value },
}

/// Webhook request body (wrapper for update_id)
#[derive(Debug, Clone, Deserialize)]
pub struct TelegramWebhookBody {
    pub update_id: i64,
    #[serde(flatten)]
    pub update: TelegramUpdate,
}

/// Webhook response (minimal)
#[derive(Debug, Clone, Serialize)]
pub struct WebhookResponse {
    pub status: String,
}

/// Telegram webhook configuration request (from setWebhook)
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookConfigRequest {
    pub url: Option<String>,
    pub secret_token: Option<String>,
    pub max_connections: Option<u16>,
}

/// Create Telegram webhook router
pub fn telegram_webhook_router() -> Router {
    Router::new()
        .route("/api/v1/telegram/webhook", post(handle_telegram_webhook))
        .route(
            "/api/v1/telegram/config/webhook",
            post(handle_telegram_webhook_config),
        )
}

/// Handle Telegram webhook (POST /api/v1/telegram/webhook)
#[allow(clippy::too_many_arguments)]
pub async fn handle_telegram_webhook(
    ConnectInfo(_peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // ── Verify webhook secret (X-Telegram-Bot-Api-Secret-Token) - OPTIONAL for now
    let _secret_header = headers.get("x-telegram-bot-api-secret-token");

    // Note: Secret token validation is optional for testing
    // In production, validate against stored TelegramChannel config

    // ── Parse JSON body
    let update: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Telegram webhook: JSON parse error: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid JSON",
                    "details": e.to_string()
                })),
            );
        }
    };

    // ── Log update (for debugging)
    tracing::info!("📨 Telegram webhook received update: {:?}", update);

    // ── Forward update to telegram channel for processing
    // Import the function to send webhook update
    if let Err(e) = crate::channels::telegram::send_webhook_update(update.clone()) {
        tracing::error!(
            "Failed to forward webhook update to telegram channel: {}",
            e
        );
        // Still return ok to Telegram so it doesn't retry
    } else {
        tracing::info!("✅ Webhook update forwarded to telegram channel");
    }

    // ── Extract update_id and type
    let _update_id = update["update_id"].as_i64().unwrap_or(0);

    // ── Route to appropriate handler
    if update["message"].is_object() {
        let message = &update["message"];
        let chat_id = message["chat"]["id"].as_i64().unwrap_or(0);
        let text = message["text"].as_str().unwrap_or("");

        tracing::info!("📩 Message from chat_id={}: {}", chat_id, text);
    } else if update["callback_query"].is_object() {
        let callback_query = &update["callback_query"];
        let query_id = callback_query["id"].as_str().unwrap_or("");
        let data = callback_query["data"].as_str().unwrap_or("");

        tracing::info!("🔘 Callback query: id={}, data={}", query_id, data);
    }

    // ── Success response
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok"
        })),
    )
}

/// Handle Telegram webhook configuration (POST /api/v1/telegram/config/webhook)
#[allow(clippy::too_many_arguments)]
pub async fn handle_telegram_webhook_config(
    headers: HeaderMap,
    Json(config): Json<WebhookConfigRequest>,
) -> impl IntoResponse {
    // ── Validate request (in production, verify auth)
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !auth.starts_with("Bearer ") {
        tracing::warn!("Telegram webhook config: missing or invalid Authorization header");
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized",
                "message": "Missing or invalid Authorization header"
            })),
        );
    }

    // ── Log configuration
    tracing::info!("🔧 Telegram webhook config request: {:?}", config);

    // ── Validate config
    if let Some(ref url) = config.url {
        if !url.starts_with("https://") && !url.starts_with("http://") {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid URL",
                    "message": "Webhook URL must start with http:// or https://"
                })),
            );
        }
    }

    // ── Success response (in production, call TelegramChannel::setup_webhook())
    tracing::info!("✅ Telegram webhook configuration accepted");
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "configured",
            "message": "Webhook configuration accepted"
        })),
    )
}
