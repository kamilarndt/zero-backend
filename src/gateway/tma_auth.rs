//! Telegram Mini Apps (TMA) Authentication (Zero-Bloat)
//!
//! Minimalist TMA auth handler with:
//! - initData verification (HMAC-SHA256)
//! - JWT token generation
//! - Anti-replay protection (auth_date validation)

use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// TMA authentication request
#[derive(Debug, Clone, Deserialize)]
pub struct TMAAuthRequest {
    /// URL-encoded initData from Telegram WebApp
    pub init_data: String,
}

/// TMA authentication response
#[derive(Debug, Clone, Serialize)]
pub struct TMAAuthResponse {
    /// JWT Bearer token
    pub token: String,
    /// User information
    pub user: TMAUserInfo,
    /// Token expiration (seconds from now)
    pub expires_in: u64,
}

/// User information extracted from TMA initData
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TMAUserInfo {
    pub id: i64,
    pub first_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
}

/// Create TMA authentication router
pub fn tma_auth_router() -> Router {
    Router::new().route("/api/v1/telegram/tma/auth", post(handle_tma_auth))
}

/// Handle TMA authentication (POST /api/v1/telegram/tma/auth)
#[allow(clippy::too_many_arguments)]
pub async fn handle_tma_auth(Json(req): Json<TMAAuthRequest>) -> impl IntoResponse {
    // TODO: Extract bot_token and jwt_secret from actual state/config
    let bot_token = ""; // TODO: from config
    let jwt_secret = b"placeholder_secret"; // TODO: from config

    // ── Parse initData (URL-encoded)
    let params: std::collections::HashMap<String, String> =
        match serde_urlencoded::from_str(&req.init_data) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("TMA auth: failed to parse initData: {}", e);
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Invalid init_data",
                        "message": "Failed to parse URL-encoded initData"
                    })),
                );
            }
        };

    // ── Extract hash
    let hash = match params.get("hash") {
        Some(h) => h.clone(),
        None => {
            tracing::warn!("TMA auth: missing hash parameter");
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Missing hash",
                    "message": "init_data must include hash parameter"
                })),
            );
        }
    };

    // ── Create data check string
    let data_check_string = create_data_check_string(&params);

    // ── Verify HMAC-SHA256 signature
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    // Create secret key: HMAC-SHA256(bot_token, "WebAppData")
    let mut secret_key_mac =
        HmacSha256::new_from_slice(bot_token.as_bytes()).expect("HMAC key should be valid");
    secret_key_mac.update(b"WebAppData");
    let secret_key = secret_key_mac.finalize().into_bytes();

    // Compute expected hash: HMAC-SHA256(secret_key, data_check_string)
    let mut hash_mac = HmacSha256::new_from_slice(&secret_key).expect("HMAC key should be valid");
    hash_mac.update(data_check_string.as_bytes());
    let expected_hash = hex::encode(hash_mac.finalize().into_bytes());

    if hash != expected_hash {
        tracing::warn!("TMA auth: invalid signature (replay attack or tampering)");
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Invalid signature",
                "message": "HMAC-SHA256 verification failed"
            })),
        );
    }

    // ── Anti-replay: verify auth_date (max 5 minutes old)
    if let Some(auth_date_str) = params.get("auth_date") {
        if let Ok(auth_timestamp) = auth_date_str.parse::<i64>() {
            use chrono::Utc;
            let current_time = Utc::now().timestamp();
            let max_age = 300; // 5 minutes

            if (current_time - auth_timestamp).abs() > max_age {
                tracing::warn!("TMA auth: initData too old (replay attack protection)");
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({
                        "error": "Expired init_data",
                        "message": "auth_date is too old (replay attack protection)"
                    })),
                );
            }
        }
    }

    // ── Parse user
    let user_json = match params.get("user") {
        Some(u) => u,
        None => {
            tracing::warn!("TMA auth: missing user parameter");
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Missing user",
                    "message": "init_data must include user parameter"
                })),
            );
        }
    };

    let user_data: TMAUserInfo = match serde_json::from_str(user_json) {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!("TMA auth: failed to parse user JSON: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid user",
                    "message": "Failed to parse user JSON"
                })),
            );
        }
    };

    // ── Generate JWT token (24 hours expiration)
    use chrono::{Duration, Utc};
    use jsonwebtoken::{encode, EncodingKey, Header};

    let expiration = Utc::now() + Duration::hours(24);

    let claims = serde_json::json!({
        "sub": user_data.id,
        "name": user_data.first_name,
        "username": user_data.username,
        "exp": expiration.timestamp(),
    });

    let token = match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret),
    ) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("TMA auth: failed to generate JWT: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Token generation failed",
                    "message": "Failed to generate JWT token"
                })),
            );
        }
    };

    // ── Log successful authentication
    tracing::info!(
        "✅ TMA auth: user_id={}, username={:?}",
        user_data.id,
        user_data.username
    );

    // ── Success response
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "token": token,
            "user": user_data,
            "expires_in": 86400 // 24 hours in seconds
        })),
    )
}

/// Create data check string from Telegram initData parameters
fn create_data_check_string(params: &std::collections::HashMap<String, String>) -> String {
    let mut sorted: Vec<_> = params.iter().filter(|(k, _)| *k != "hash").collect();
    sorted.sort_by_key(|(k, _)| *k);

    sorted
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n")
}

/// TMA Auth state shared across handlers
#[derive(Clone)]
pub struct TMAAuthState {
    pub bot_token: Arc<String>,
    pub jwt_secret: Arc<[u8]>,
}
