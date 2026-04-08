//! OpenAI-compatible `/v1/chat/completions` and `/v1/models` endpoints.

use super::format_sse_event;
use super::AppState;
use crate::providers::traits::{ChatMessage, ChatResponse, ToolCall};
use crate::tools::traits::ToolSpec;
// Import routing types for zeroclaw-auto-router
use crate::routing::{ClassificationInput, ClassificationResult};
use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

pub const CHAT_COMPLETIONS_MAX_BODY_SIZE: usize = 524_288;

#[derive(Debug, Deserialize)]
pub struct ChatCompletionsRequest {
    #[serde(default)]
    pub model: Option<String>,
    pub messages: Vec<ChatCompletionsMessage>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub stream: Option<bool>,
    /// tldraw Agent: Tools/actions available to the LLM (Zod schemas)
    #[serde(default)]
    pub tools: Option<Vec<ToolDefinition>>,
}

/// Tool definition for function calling (tldraw agent actions)
#[derive(Debug, Deserialize, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

impl From<ToolDefinition> for ToolSpec {
    fn from(def: ToolDefinition) -> Self {
        Self {
            name: def.name,
            description: def.description,
            parameters: def.parameters,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatCompletionsMessage {
    pub role: String,
    pub content: String,
    /// tldraw Agent: Extracted image URLs from multimodal content (base64 data URLs)
    pub image_urls: Vec<String>,
    // Optional OpenAI fields that Cline may send
    pub name: Option<String>,
    pub tool_calls: Option<serde_json::Value>,
    pub tool_call_id: Option<String>,
    pub tool_id: Option<String>,
    pub refusal: Option<String>,
}

// Manual Deserialize implementation for multimodal support
impl<'de> serde::Deserialize<'de> for ChatCompletionsMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, MapAccess, Visitor};
        use std::fmt;

        // Helper struct for field deserialization
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Role,
            Content,
            Name,
            ToolCalls,
            ToolCallId,
            ToolId,
            Refusal,
        }

        struct ChatMessageVisitor;

        impl<'de> Visitor<'de> for ChatMessageVisitor {
            type Value = ChatCompletionsMessage;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct ChatCompletionsMessage")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut role = None;
                let mut raw_content: Option<serde_json::Value> = None;
                let mut name = None;
                let mut tool_calls = None;
                let mut tool_call_id = None;
                let mut tool_id = None;
                let mut refusal = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Role => {
                            if role.is_some() {
                                return Err(Error::duplicate_field("role"));
                            }
                            role = Some(map.next_value()?);
                        }
                        Field::Content => {
                            if raw_content.is_some() {
                                return Err(Error::duplicate_field("content"));
                            }
                            raw_content = Some(map.next_value()?);
                        }
                        Field::Name => {
                            if name.is_some() {
                                return Err(Error::duplicate_field("name"));
                            }
                            name = Some(map.next_value()?);
                        }
                        Field::ToolCalls => {
                            if tool_calls.is_some() {
                                return Err(Error::duplicate_field("tool_calls"));
                            }
                            tool_calls = Some(map.next_value()?);
                        }
                        Field::ToolCallId => {
                            if tool_call_id.is_some() {
                                return Err(Error::duplicate_field("tool_call_id"));
                            }
                            tool_call_id = Some(map.next_value()?);
                        }
                        Field::ToolId => {
                            if tool_id.is_some() {
                                return Err(Error::duplicate_field("tool_id"));
                            }
                            tool_id = Some(map.next_value()?);
                        }
                        Field::Refusal => {
                            if refusal.is_some() {
                                return Err(Error::duplicate_field("refusal"));
                            }
                            refusal = Some(map.next_value()?);
                        }
                    }
                }

                let role = role.ok_or_else(|| Error::missing_field("role"))?;

                // Process content to extract text and image URLs
                let (content, image_urls) = if let Some(content_val) = raw_content {
                    deserialize_content_extract::<'de, _>(content_val).map_err(Error::custom)?
                } else {
                    (String::new(), Vec::new())
                };

                Ok(ChatCompletionsMessage {
                    role,
                    content,
                    image_urls,
                    name: name.or_else(|| None), // Default to None
                    tool_calls: tool_calls.or_else(|| None), // Default to None
                    tool_call_id: tool_call_id.or_else(|| None), // Default to None
                    tool_id: tool_id.or_else(|| None), // Default to None
                    refusal: refusal.or_else(|| None), // Default to None
                })
            }
        }

        // String deserialization (for edge cases)
        const FIELDS: &[&str] = &[
            "role",
            "content",
            "name",
            "tool_calls",
            "tool_call_id",
            "tool_id",
            "refusal",
        ];
        deserializer.deserialize_struct("ChatCompletionsMessage", FIELDS, ChatMessageVisitor)
    }
}

/// Deserialize content field - handles both string and array formats
/// Returns (text_content, image_urls)
fn deserialize_content_extract<'de, D>(deserializer: D) -> Result<(String, Vec<String>), D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    // Try to deserialize as a string first
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Content {
        String(String),
        Array(Vec<serde_json::Value>),
    }

    match Content::deserialize(deserializer)? {
        Content::String(s) => Ok((s, Vec::new())),
        Content::Array(arr) => {
            // Extract text and images from array format:
            // [{"type": "text", "text": "..."}, {"type": "image_url", "image_url": {"url": "data:image/png;base64,..."}}]
            let mut text_parts = Vec::new();
            let mut image_urls = Vec::new();

            for item in arr.iter() {
                if let Some(obj) = item.as_object() {
                    let content_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");

                    match content_type {
                        "text" => {
                            if let Some(text) = obj.get("text").and_then(|t| t.as_str()) {
                                text_parts.push(text.to_string());
                            }
                        }
                        "image_url" => {
                            if let Some(img_obj) = obj.get("image_url").and_then(|v| v.as_object())
                            {
                                if let Some(url) = img_obj.get("url").and_then(|u| u.as_str()) {
                                    image_urls.push(url.to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            let text = if text_parts.is_empty() {
                // Fallback: serialize entire array as string
                serde_json::to_string(&arr).map_err(serde::de::Error::custom)?
            } else {
                text_parts.join("")
            };

            Ok((text, image_urls))
        }
    }
}

/// Custom deserializer for ChatCompletionsMessage to handle multimodal content
fn deserialize_chat_message<'de, D>(deserializer: D) -> Result<ChatCompletionsMessage, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    // Helper struct for raw deserialization
    #[derive(Deserialize)]
    struct RawMessage {
        role: String,
        #[serde(default)]
        content: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        tool_calls: Option<serde_json::Value>,
        #[serde(default)]
        tool_call_id: Option<String>,
        #[serde(default)]
        tool_id: Option<String>,
        #[serde(default)]
        refusal: Option<String>,
    }

    // Try to deserialize as standard object first
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum MessageFormat {
        Standard(RawMessage),
        Multimodal(serde_json::Value),
    }

    let msg = match MessageFormat::deserialize(deserializer)? {
        MessageFormat::Standard(raw) => {
            // Simple string content - no images
            ChatCompletionsMessage {
                role: raw.role,
                content: raw.content,
                image_urls: Vec::new(),
                name: raw.name,
                tool_calls: raw.tool_calls,
                tool_call_id: raw.tool_call_id,
                tool_id: raw.tool_id,
                refusal: raw.refusal,
            }
        }
        MessageFormat::Multimodal(value) => {
            // Check if it has multimodal content array
            let obj = value
                .as_object()
                .ok_or_else(|| serde::de::Error::custom("Expected object"))?;
            let role = obj
                .get("role")
                .and_then(|v| v.as_str())
                .ok_or_else(|| serde::de::Error::custom("Missing role"))?
                .to_string();

            let (content, image_urls) = if let Some(content_val) = obj.get("content") {
                deserialize_content_extract(content_val).map_err(serde::de::Error::custom)?
            } else {
                (String::new(), Vec::new())
            };

            ChatCompletionsMessage {
                role,
                content,
                image_urls,
                name: obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                tool_calls: obj.get("tool_calls").cloned(),
                tool_call_id: obj
                    .get("tool_call_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                tool_id: obj
                    .get("tool_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                refusal: obj
                    .get("refusal")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            }
        }
    };

    Ok(msg)
}

// Legacy deserialize_content for backward compatibility (not used anymore but kept for safety)
fn deserialize_content<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let (text, _images) = deserialize_content_extract(deserializer)?;
    Ok(text)
}

#[derive(Debug, Serialize)]
pub struct ChatCompletionsResponse {
    pub id: String,
    pub object: &'static str,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatCompletionsChoice>,
    pub usage: ChatCompletionsUsage,
}

#[derive(Debug, Serialize)]
pub struct ChatCompletionsChoice {
    pub index: u32,
    pub message: ChatCompletionsResponseMessage,
    pub finish_reason: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ChatCompletionsResponseMessage {
    pub role: &'static str,
    pub content: String,
    /// tldraw Agent: Tool calls returned by the LLM (transparent proxying)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Serialize)]
pub struct ChatCompletionsUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Serialize)]
struct ChatCompletionsChunk {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
    choices: Vec<ChunkChoice>,
}

#[derive(Debug, Serialize)]
struct ChunkChoice {
    index: u32,
    delta: ChunkDelta,
    finish_reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
struct ChunkDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    pub object: &'static str,
    pub data: Vec<ModelObject>,
}

#[derive(Debug, Serialize)]
pub struct ModelObject {
    pub id: String,
    pub object: &'static str,
    pub created: u64,
    pub owned_by: String,
}

/// Convert OpenAI messages to ZeroClaw ChatMessage format.
///
/// # Arguments
/// * `messages` - OpenAI format messages from request
///
/// # Returns
/// Vector of ZeroClaw ChatMessage objects
pub fn convert_openai_messages(messages: Vec<ChatCompletionsMessage>) -> Vec<ChatMessage> {
    messages
        .into_iter()
        .map(|msg| match msg.role.as_str() {
            "system" => ChatMessage::system(msg.content),
            "user" => ChatMessage::user(msg.content),
            "assistant" => ChatMessage::assistant(msg.content),
            _ => ChatMessage::user(msg.content), // Default to user for unknown roles
        })
        .collect()
}

pub async fn handle_v1_chat_completions(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let rate_key =
        super::client_key_from_request(Some(peer_addr), &headers, state.trust_forwarded_headers);
    if !state.rate_limiter.allow_webhook(&rate_key) {
        let err = serde_json::json!({
            "error": {
                "message": "Rate limit exceeded. Please retry later.",
                "type": "rate_limit_error",
                "code": "rate_limit_exceeded"
            }
        });
        return (StatusCode::TOO_MANY_REQUESTS, Json(err)).into_response();
    }

    if state.pairing.require_pairing() {
        let auth = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if !(state.pairing.is_authenticated(token).await) {
            let err = serde_json::json!({
                "error": {
                    "message": "Invalid API key. Pair first via POST /pair",
                    "type": "invalid_request_error",
                    "code": "invalid_api_key"
                }
            });
            return (StatusCode::UNAUTHORIZED, Json(err)).into_response();
        }
    }

    if body.len() > CHAT_COMPLETIONS_MAX_BODY_SIZE {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({"error": "Payload too large"})),
        )
            .into_response();
    }

    // Log request body for debugging (truncated if too large)
    let body_str = String::from_utf8_lossy(&body);
    let body_preview = if body_str.len() > 500 {
        format!(
            "{}... (truncated, total {} bytes)",
            &body_str[..500],
            body.len()
        )
    } else {
        body_str.to_string()
    };
    tracing::info!("Request body preview: {}", body_preview);

    let request: ChatCompletionsRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Invalid JSON: {e}")})),
            )
                .into_response()
        }
    };

    // Log request details for debugging
    tracing::info!(
        "Received chat completions request: stream={}, model={}, messages_count={}",
        request.stream.is_some(),
        request.model.as_deref().unwrap_or("default"),
        request.messages.len()
    );

    let model = request.model.unwrap_or_else(|| state.model.clone());
    // Map "auto" model to default model (Cline uses "auto")
    let model = if model == "auto" {
        state.model.clone()
    } else if model == "zeroclaw-auto-router" {
        // 🧠 Virtual Model: Intelligent routing based on task classification
        match &state.classifier {
            Some(classifier) => {
                // Extract user query for classification from request.messages
                let user_query = request
                    .messages
                    .iter()
                    .filter(|m| m.role == "user")
                    .last()
                    .map(|m| m.content.clone())
                    .unwrap_or_default();

                // tldraw Agent: Detect images in messages for vision routing
                let has_images = request.messages.iter().any(|m| !m.image_urls.is_empty());

                // Extract MIME types from image URLs for vision classification
                let image_mime_types: Vec<String> = request
                    .messages
                    .iter()
                    .flat_map(|m| m.image_urls.iter())
                    .filter_map(|url| {
                        // Extract MIME type from data URL: "data:image/png;base64,..."
                        if url.starts_with("data:") {
                            url.split(',')
                                .next()
                                .and_then(|part| part.split(':').nth(1))
                                .map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();

                // Log vision detection
                if has_images {
                    tracing::info!(
                        "🎨 Vision content detected: {} image(s) found, MIME types: {:?}",
                        image_mime_types.len(),
                        image_mime_types
                    );
                }

                // Classify the task and get model hint (with vision support)
                let input: ClassificationInput = if has_images {
                    ClassificationInput::with_mime_types(&user_query, image_mime_types)
                } else {
                    ClassificationInput::text_only(&user_query)
                };
                let result: ClassificationResult = classifier.classify(&input);

                tracing::info!(
                    "🧠 Auto-router: classified as {:?} → model hint: {}",
                    result.task_type,
                    result.model
                );

                // Map hint to actual model (from config.toml [[model_routes]])
                // This uses the same routing logic as configured in TOML
                result.model
            }
            None => {
                tracing::warn!("🧠 Auto-router requested but classifier not initialized, falling back to default");
                state.model.clone()
            }
        }
    } else {
        model
    };
    let temperature = request.temperature.unwrap_or(state.temperature);

    // 🛡️ EARLY BAIL-OUT FOR TOOL REQUESTS (ZeroClaw Architect Directive)
    // For tldraw Agent: If request has tools, force non-streaming to get tool_calls
    let has_tools = request.tools.is_some() && request.tools.as_ref().unwrap().len() > 0;
    let stream = if has_tools {
        // Force non-streaming when tools are present (tldraw requirement)
        tracing::info!("🛠️ Early Bail-out: Request has tools, forcing non-streaming mode");
        false
    } else {
        request.stream.unwrap_or(false)
    };

    // Convert OpenAI messages to ZeroClaw ChatMessage format
    let messages: Vec<ChatMessage> = convert_openai_messages(request.messages);

    // Extract user message for skill matching
    let user_query = messages
        .iter()
        .filter(|m| m.role == "user")
        .last()
        .map(|m| m.content.clone())
        .unwrap_or_default();

    // Enrich system prompt with matching skills
    let base_system_prompt = "You are a helpful AI assistant.";
    let enriched_system = match &state.skill_loader {
        Some(loader) => {
            use crate::skills::loader::SkillLoader;
            loader
                .enrich_system_prompt(&user_query, &base_system_prompt)
                .await
                .unwrap_or_else(|_| base_system_prompt.to_string())
        }
        None => base_system_prompt.to_string(),
    };

    // TODO: Pass enriched_system to the agent/provider
    // For now, the integration point is ready
    tracing::debug!(
        "Enriched system prompt with skills for query: {}",
        user_query
    );

    if stream {
        // 🔍 REGULAR STREAMING (no tools) - Keep old logic for non-tldraw clients
        // tldraw requests with tools are handled as non-streaming above
        // Tool streaming is NOT implemented here - uses word splitting simulation

        // Build messages with system prompt
        let mut full_messages = vec![ChatMessage::system(enriched_system)];
        full_messages.extend(messages);

        // Get response from provider
        let response_text = match state
            .provider
            .chat_with_history(&full_messages, &model, temperature)
            .await
        {
            Ok(text) => text,
            Err(e) => {
                // For streaming, return error as SSE event
                let error_json = serde_json::json!({
                    "error": {
                        "message": format!("Provider error: {}", e),
                        "type": "provider_error"
                    }
                });

                let sse_error = format!("data: {}\n\ndata: [DONE]\n\n", error_json);
                return axum::response::Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "text/event-stream")
                    .header("Cache-Control", "no-cache")
                    .header("Connection", "keep-alive")
                    .header("X-Accel-Buffering", "no")
                    .body(Body::from(sse_error))
                    .unwrap()
                    .into_response();
            }
        };

        // Check if response is an error message (don't stream errors)
        if response_text.contains("Provider error:") || response_text.contains("error") {
            let error_chunk = super::openai_sse_types::SSEChunk {
                id: format!("chatcmpl-{}", Uuid::new_v4()),
                object: "chat.completion.chunk",
                created: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                model: model.clone(),
                choices: vec![super::openai_sse_types::DeltaChoice {
                    index: 0,
                    delta: super::openai_sse_types::DeltaDelta {
                        content: Some(response_text.clone()),
                        tool_calls: None,
                    },
                    finish_reason: None, // Error chunks don't have finish_reason
                }],
            };

            let sse_body = format!("{}\ndata: [DONE]\n\n", format_sse_event(&error_chunk));
            return axum::response::Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/event-stream")
                .header("Cache-Control", "no-cache")
                .header("Connection", "keep-alive")
                .header("X-Accel-Buffering", "no")
                .body(Body::from(sse_body))
                .unwrap()
                .into_response();
        }

        // 🚫 NOTE: Tool streaming is NOT implemented here
        // tldraw requests with tools are forced to non-streaming above
        // This preserves word boundaries better than char-based chunking
        let words: Vec<&str> = response_text.split_whitespace().collect();
        let chunks: Vec<String> = words
            .chunks(5) // 5 words per chunk
            .map(|w| w.join(" "))
            .collect();

        let mut sse_body = String::new();
        let response_id = format!("chatcmpl-{}", Uuid::new_v4());
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for chunk in chunks.iter() {
            let sse_chunk = super::openai_sse_types::SSEChunk {
                id: response_id.clone(),
                object: "chat.completion.chunk",
                created,
                model: model.clone(),
                choices: vec![super::openai_sse_types::DeltaChoice {
                    index: 0,
                    delta: super::openai_sse_types::DeltaDelta {
                        content: Some(chunk.clone()),
                        tool_calls: None,
                    },
                    finish_reason: None,
                }],
            };
            sse_body.push_str(&format_sse_event(&sse_chunk));
        }

        // Final chunk with finish_reason - delta should be None, not empty string
        let final_sse_chunk = super::openai_sse_types::SSEChunk {
            id: response_id,
            object: "chat.completion.chunk",
            created,
            model: model.clone(),
            choices: vec![super::openai_sse_types::DeltaChoice {
                index: 0,
                delta: super::openai_sse_types::DeltaDelta {
                    content: None, // Important: None not empty string for final chunk
                    tool_calls: None,
                },
                finish_reason: Some("stop"),
            }],
        };
        sse_body.push_str(&format_sse_event(&final_sse_chunk));
        sse_body.push_str("data: [DONE]\n\n");

        // Log SSE body for debugging (truncated if too large)
        if sse_body.len() > 2000 {
            tracing::info!(
                "SSE response preview (first 2000 chars): {}...",
                &sse_body[..2000]
            );
        } else {
            tracing::info!("SSE response: {}", sse_body);
        }

        axum::response::Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .header("X-Accel-Buffering", "no") // Disable nginx buffering
            .body(Body::from(sse_body))
            .unwrap()
            .into_response()
    } else {
        // Non-streaming: use provider to get actual response
        let mut full_messages = vec![ChatMessage::system(enriched_system)];
        full_messages.extend(messages);

        // tldraw Agent: Convert tools if present
        let tools: Option<Vec<ToolSpec>> = request.tools.as_ref().map(|defs| {
            defs.iter()
                .map(|def| ToolSpec {
                    name: def.name.clone(),
                    description: def.description.clone(),
                    parameters: def.parameters.clone(),
                })
                .collect()
        });

        let tools_json: Option<Vec<serde_json::Value>> = request.tools.as_ref().map(|defs| {
            defs.iter()
                .map(|def| serde_json::to_value(def).unwrap())
                .collect()
        });

        let chat_response = if let Some(_tools_spec) = tools {
            // Use chat_with_tools for tldraw agent actions
            state
                .provider
                .chat_with_tools(
                    &full_messages,
                    &tools_json.unwrap_or_default(),
                    &model,
                    temperature,
                )
                .await
        } else {
            // Fallback to regular chat
            match state
                .provider
                .chat_with_history(&full_messages, &model, temperature)
                .await
            {
                Ok(text) => Ok(ChatResponse {
                    text: Some(text),
                    tool_calls: Vec::new(),
                    usage: None,
                    reasoning_content: None,
                }),
                Err(e) => Err(e),
            }
        };

        let chat_response = match chat_response {
            Ok(resp) => resp,
            Err(e) => {
                let error_json = serde_json::json!({
                    "error": {
                        "message": format!("Provider error: {}", e),
                        "type": "provider_error"
                    }
                });
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(error_json)).into_response();
            }
        };

        // tldraw Agent: Include tool_calls in response if present
        let response_content = chat_response.text.clone().unwrap_or_default();
        let has_tool_calls = !chat_response.tool_calls.is_empty();
        let response_tools = if has_tool_calls {
            Some(chat_response.tool_calls)
        } else {
            None
        };

        let response = ChatCompletionsResponse {
            id: format!("chatcmpl-{}", Uuid::new_v4()),
            object: "chat.completion",
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            model,
            choices: vec![ChatCompletionsChoice {
                index: 0,
                message: ChatCompletionsResponseMessage {
                    role: "assistant",
                    content: response_content,
                    tool_calls: response_tools,
                },
                finish_reason: if has_tool_calls { "tool_calls" } else { "stop" },
            }],
            usage: ChatCompletionsUsage {
                prompt_tokens: chat_response
                    .usage
                    .as_ref()
                    .and_then(|u| u.input_tokens)
                    .unwrap_or(0) as u32,
                completion_tokens: chat_response
                    .usage
                    .as_ref()
                    .and_then(|u| u.output_tokens)
                    .unwrap_or(0) as u32,
                total_tokens: 0,
            },
        };
        (
            StatusCode::OK,
            Json(serde_json::to_value(response).unwrap()),
        )
            .into_response()
    }
}

pub async fn handle_v1_models(
    State(state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    // Build list of available models
    let mut models = vec![ModelObject {
        id: state.model.clone(),
        object: "model",
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        owned_by: "zeroclaw".to_string(),
    }];

    // Add virtual model if classifier is enabled
    if state.classifier.is_some() {
        models.push(ModelObject {
            id: "zeroclaw-auto-router".to_string(),
            object: "model",
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            owned_by: "zeroclaw".to_string(),
        });
    }

    let response = ModelsResponse {
        object: "list",
        data: models,
    };
    (
        StatusCode::OK,
        Json(serde_json::to_value(response).unwrap()),
    )
}
