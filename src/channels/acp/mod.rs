//! Agent Client Protocol (ACP) implementation for Zed Editor integration.
//!
//! This module implements the ACP protocol which allows Zed Editor to communicate
//! with ZeroClaw via JSON-RPC over stdio. The protocol uses standard input/output
//! for bidirectional JSON-RPC message exchange.
//!
//! # Protocol Overview
//!
//! - Client (Zed) sends JSON-RPC requests to stdin
//! - Server (ZeroClaw) sends JSON-RPC responses to stdout
//! - All logs are redirected to a file to avoid corrupting the JSON-RPC stream
//!
//! # JSON-RPC Methods
//!
//! - `initialize`: Protocol initialization handshake
//! - `text/append`: Append text to a pending response
//! - `text/delta`: Send incremental text updates
//! - `text/complete`: Finalize a response
//! - `prompt`: Process a user prompt through the Agent
//! - `shutdown`: Graceful connection termination
//!
//! # Zed Commands
//!
//! The agent can return `zedCommands` in the response to control the Zed UI:
//! - `openFile`: Open a file at optional line number
//! - `scrollTo`: Scroll to a specific line
//! - `highlight`: Highlight a range of text

pub mod commands;
pub mod streaming;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::io::{self, BufRead, BufReader, Write};
use std::sync::Arc;
use std::pin::Pin;
use tokio::sync::Mutex;
use uuid::Uuid;
use futures_util::StreamExt;

use crate::config::Config;
use crate::agent::loop_::process_message;
use crate::channels::load_skills_by_name;
use crate::skills;

// Re-export commands for convenience
pub use commands::parse_zed_commands;

// Import streaming functionality
use streaming::process_prompt_true_streaming;

/// JSON-RPC request as defined by the ACP protocol.
#[derive(Debug, Clone, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: Option<String>,
    id: Option<JsonValue>,
    method: String,
    params: Option<JsonValue>,
}

/// JSON-RPC response as defined by the ACP protocol.
#[derive(Debug, Clone, Serialize)]
struct JsonRpcResponse {
    jsonrpc: Option<String>,
    id: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC error as defined by the protocol.
#[derive(Debug, Clone, Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<JsonValue>,
}

impl JsonRpcError {
    /// Create a new error with code and message.
    fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Standard error codes from JSON-RPC specification.
    const PARSE_ERROR_CODE: i64 = -32700;
    const INVALID_REQUEST_CODE: i64 = -32600;
    const METHOD_NOT_FOUND_CODE: i64 = -32601;
    const INVALID_PARAMS_CODE: i64 = -32602;
    const INTERNAL_ERROR_CODE: i64 = -32603;

    fn parse_error() -> Self {
        Self {
            code: Self::PARSE_ERROR_CODE,
            message: "Parse error".into(),
            data: None,
        }
    }

    fn invalid_request() -> Self {
        Self {
            code: Self::INVALID_REQUEST_CODE,
            message: "Invalid Request".into(),
            data: None,
        }
    }

    fn method_not_found() -> Self {
        Self {
            code: Self::METHOD_NOT_FOUND_CODE,
            message: "Method not found".into(),
            data: None,
        }
    }

    fn invalid_params() -> Self {
        Self {
            code: Self::INVALID_PARAMS_CODE,
            message: "Invalid params".into(),
            data: None,
        }
    }

    fn internal_error() -> Self {
        Self {
            code: Self::INTERNAL_ERROR_CODE,
            message: "Internal error".into(),
            data: None,
        }
    }

    fn internal_error_with_message(message: impl Into<String>) -> Self {
        Self {
            code: Self::INTERNAL_ERROR_CODE,
            message: message.into(),
            data: None,
        }
    }
}

/// Build an enriched prompt with active skills injected.
///
/// When specific skills are requested, this function loads them and
/// prepends their instructions to the user prompt.
fn build_prompt_with_skills(config: &Config, prompt: &str, active_skill_names: &[String]) -> String {
    if active_skill_names.is_empty() {
        return prompt.to_string();
    }

    // Load all available skills
    let all_skills = skills::load_skills_with_config(&config.workspace_dir, config);

    // Filter to only the requested skills
    let filtered_skills: Vec<skills::Skill> = load_skills_by_name(&all_skills, active_skill_names);

    if filtered_skills.is_empty() {
        tracing::warn!(
            requested = ?active_skill_names,
            "No matching skills found"
        );
        return prompt.to_string();
    }

    // Convert skills to prompt format
    let skills_prompt = skills::skills_to_prompt_with_mode(
        &filtered_skills,
        &config.workspace_dir,
        config.skills.prompt_injection_mode,
    );

    if skills_prompt.is_empty() {
        prompt.to_string()
    } else {
        format!(
            "{}\n\nUser request: {}",
            skills_prompt.trim(),
            prompt
        )
    }
}

/// Chunk size for pseudo-streaming responses (in characters).
const STREAM_CHUNK_SIZE: usize = 50;

/// Stream a prompt through ZeroClaw with pseudo-streaming support for ACP.
///
/// This is the fallback implementation that uses "pseudo-streaming" - it gets
/// the full response from the agent then streams it in chunks. This provides
/// the UI experience of streaming without requiring provider-level streaming support.
async fn process_prompt_pseudo_streaming(
    config: &Config,
    prompt: &str,
) -> Result<Pin<Box<dyn futures_util::Stream<Item = Result<String>> + Send>>> {
    // Get the full response from the agent
    let result_text = process_message(config.clone(), prompt).await?;

    // Parse Zed commands from the response
    let (cleaned_text, zed_commands) = parse_zed_commands(&result_text);

    let request_id = Uuid::new_v4().to_string();
    let request_id_for_stream = request_id.clone();

    // Create a stream that emits the response in chunks
    let chunks: Vec<String> = cleaned_text
        .chars()
        .collect::<Vec<char>>()
        .chunks(STREAM_CHUNK_SIZE)
        .map(|chunk| chunk.iter().collect())
        .collect();

    // Convert chunks to a stream
    let chunk_count = chunks.len();
    let stream = futures_util::stream::iter(chunks.into_iter().enumerate()).then(
        move |(index, chunk)| {
            let req_id = request_id_for_stream.clone();
            let is_last = index + 1 == chunk_count;

            async move {
                let notification = json!({
                    "jsonrpc": "2.0",
                    "method": "progress",
                    "params": {
                        "requestId": req_id,
                        "type": "text/delta",
                        "text": chunk,
                        "isLast": is_last
                    }
                });

                Ok(serde_json::to_string(&notification)?)
            }
        },
    );

    // Add final completion message
    let final_stream = stream.chain(futures_util::stream::once(async move {
        let final_response = json!({
            "requestId": request_id,
            "status": "complete",
            "response": cleaned_text,
            "zedCommands": zed_commands
        });

        Ok(serde_json::to_string(&final_response)?)
    }));

    Ok(Box::pin(final_stream))
}

/// Build prompt with Zed commands instructions for ACP mode.
///
/// When running in ACP mode (Zed Editor integration), the agent needs to know
/// about special commands it can use to control the editor UI.
fn build_prompt_with_zed_instructions(prompt: &str) -> String {
    format!(
        "{}\n\n{}",
        "You are running in Zed Editor ACP mode. You can control the editor using special commands:\n\
         - [[ZED:open:path/to/file]] - Open a file\n\
         - [[ZED:open:path/to/file:42]] - Open file at line 42\n\
         - [[ZED:scroll:100]] - Scroll to line 100\n\
         - [[ZED:highlight:10:20]] - Highlight lines 10-20\n\
         - [[ZED:info:message]] - Show info notification\n\
         - [[ZED:warn:message]] - Show warning\n\
         - [[ZED:error:message]] - Show error\n\n\
         When you need to show code or point to a specific location, use these commands.",
        prompt
    )
}

/// ACP protocol handler that manages JSON-RPC communication over stdio.
pub struct AcpHandler {
    /// Pending response text being accumulated before sending.
    pending_text: Arc<Mutex<String>>,
    /// Unique identifier for this ACP session.
    session_id: String,
    /// ZeroClaw configuration for Agent processing.
    config: Config,
}

impl AcpHandler {
    /// Create a new ACP handler with the given configuration.
    pub fn new(config: Config) -> Self {
        Self {
            pending_text: Arc::new(Mutex::new(String::new())),
            session_id: Uuid::new_v4().to_string(),
            config,
        }
    }

    /// Handle the `initialize` method from the ACP protocol.
    async fn handle_initialize(&self, params: Option<JsonValue>) -> Result<JsonValue> {
        let client_info = params
            .as_ref()
            .and_then(|p| p.get("clientInfo"))
            .and_then(|ci| ci.as_str())
            .unwrap_or("unknown");

        let _capabilities = params
            .as_ref()
            .and_then(|p| p.get("capabilities"))
            .cloned()
            .unwrap_or_else(|| json!({}));

        tracing::info!(
            session_id = %self.session_id,
            client = %client_info,
            "ACP protocol initialized"
        );

        // Return server capabilities
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {
                "name": "ZeroClaw",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "capabilities": {
                "streaming": true,
                "tools": true,
                "memory": true,
            }
        }))
    }

    /// Handle the `text/append` method - append text to pending response.
    async fn handle_text_append(&self, params: Option<JsonValue>) -> Result<JsonValue> {
        let text = params
            .as_ref()
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .context("Missing 'text' parameter in text/append")?;

        let mut pending = self.pending_text.lock().await;
        pending.push_str(text);
        let len = pending.len();

        tracing::debug!(
            session_id = %self.session_id,
            chars = text.len(),
            total = len,
            "ACP: text appended"
        );

        Ok(json!({
            "accepted": true,
            "length": len
        }))
    }

    /// Handle the `text/complete` method - finalize and send response.
    async fn handle_text_complete(&self, params: Option<JsonValue>) -> Result<JsonValue> {
        let _request_id = params.as_ref().and_then(|p| p.get("requestId"));

        let pending = self.pending_text.lock().await;
        let text = pending.clone();

        tracing::info!(
            session_id = %self.session_id,
            chars = text.len(),
            "ACP: text completed"
        );

        drop(pending);

        Ok(json!({
            "text": text
        }))
    }

    /// Handle incoming prompt/request from Zed - with true streaming support.
    ///
    /// This method attempts to use true streaming if the provider supports it,
    /// otherwise falls back to pseudo-streaming.
    async fn handle_prompt_streaming(&self, params: Option<JsonValue>) -> Result<()> {
        let prompt = params
            .as_ref()
            .and_then(|p| p.get("prompt"))
            .and_then(|t| t.as_str())
            .context("Missing 'prompt' parameter")?;

        let request_id = params
            .as_ref()
            .and_then(|p| p.get("requestId"))
            .and_then(|t| t.as_str())
            .map(String::from)
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Extract active_skills from params if provided
        let active_skills: Vec<String> = params
            .as_ref()
            .and_then(|p| p.get("activeSkills"))
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let _context = params
            .as_ref()
            .and_then(|p| p.get("context"))
            .cloned()
            .unwrap_or_else(|| json!({}));

        tracing::info!(
            session_id = %self.session_id,
            prompt_len = prompt.len(),
            active_skills_count = active_skills.len(),
            "ACP: received prompt (streaming)"
        );

        // Build prompt with Zed commands instructions (for ACP/Zed integration)
        let prompt_with_zed = build_prompt_with_zed_instructions(prompt);

        // Build prompt with skills if specified
        let enriched_prompt = build_prompt_with_skills(&self.config, &prompt_with_zed, &active_skills);

        // Try true streaming first, fall back to pseudo-streaming if provider doesn't support it
        let mut stream = match process_prompt_true_streaming(&self.config, &enriched_prompt, request_id.clone()).await {
            Ok(s) => {
                tracing::debug!("Using true streaming for ACP prompt");
                s
            }
            Err(e) if e.to_string().contains("does not support streaming") => {
                tracing::warn!("Provider doesn't support streaming, falling back to pseudo-streaming: {}", e);
                process_prompt_pseudo_streaming(&self.config, &enriched_prompt).await?
            }
            Err(e) => {
                return Err(e.context("Streaming failed"));
            }
        };

        // Use a channel to send chunks to a blocking stdout writer task
        // This prevents blocking the async executor during stdio writes
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);

        // Spawn a blocking task for stdout writes
        let stdout_task = tokio::task::spawn_blocking(move || {
            let stdout = io::stdout();
            let mut stdout_lock = stdout.lock();

            while let Some(chunk_json) = rx.blocking_recv() {
                if let Err(e) = writeln!(stdout_lock, "{}", chunk_json) {
                    tracing::error!("Failed to write to stdout: {}", e);
                    break;
                }
                if let Err(e) = stdout_lock.flush() {
                    tracing::error!("Failed to flush stdout: {}", e);
                    break;
                }
            }

            Ok::<(), anyhow::Error>(())
        });

        // Stream all chunks to the channel
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk_json) => {
                    if let Err(e) = tx.send(chunk_json).await {
                        tracing::error!("Failed to send chunk to stdout task: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    // Write error notification
                    let error_notification = json!({
                        "jsonrpc": "2.0",
                        "method": "progress",
                        "params": {
                            "requestId": request_id,
                            "type": "error",
                            "error": e.to_string()
                        }
                    });
                    let _ = tx.send(error_notification.to_string()).await;
                    break;
                }
            }
        }

        // Drop the sender to signal the stdout task to finish
        drop(tx);

        // Wait for the stdout task to complete
        stdout_task.await??;

        Ok(())
    }

    /// Handle incoming prompt/request from Zed (fallback non-streaming).
    async fn handle_prompt(&self, params: Option<JsonValue>) -> Result<JsonValue> {
        let prompt = params
            .as_ref()
            .and_then(|p| p.get("prompt"))
            .and_then(|t| t.as_str())
            .context("Missing 'prompt' parameter")?;

        // Extract active_skills from params if provided
        let active_skills: Vec<String> = params
            .as_ref()
            .and_then(|p| p.get("activeSkills"))
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let _context = params
            .as_ref()
            .and_then(|p| p.get("context"))
            .cloned()
            .unwrap_or_else(|| json!({}));

        tracing::info!(
            session_id = %self.session_id,
            prompt_len = prompt.len(),
            active_skills_count = active_skills.len(),
            "ACP: received prompt"
        );

        // Build prompt with Zed commands instructions (for ACP/Zed integration)
        let prompt_with_zed = build_prompt_with_zed_instructions(prompt);

        // Build prompt with skills if specified
        let enriched_prompt = build_prompt_with_skills(&self.config, &prompt_with_zed, &active_skills);

        // Process the prompt through ZeroClaw Agent
        let response = process_message(self.config.clone(), &enriched_prompt).await;

        match response {
            Ok(result_text) => {
                // Parse Zed commands from agent response
                let (cleaned_text, zed_commands) = parse_zed_commands(&result_text);

                if !zed_commands.is_empty() {
                    tracing::info!(
                        session_id = %self.session_id,
                        commands_count = zed_commands.len(),
                        "ACP: extracted Zed commands"
                    );
                }

                tracing::info!(
                    session_id = %self.session_id,
                    response_len = cleaned_text.len(),
                    "ACP: agent response generated"
                );

                let mut result = json!({
                    "requestId": Uuid::new_v4().to_string(),
                    "status": "complete",
                    "response": cleaned_text
                });

                // Add zedCommands if any were found
                if !zed_commands.is_empty() {
                    if let Some(obj) = result.as_object_mut() {
                        obj.insert(
                            "zedCommands".to_string(),
                            serde_json::to_value(zed_commands).unwrap_or_else(|_| json!([])),
                        );
                    }
                }

                Ok(result)
            }
            Err(e) => {
                tracing::error!(
                    session_id = %self.session_id,
                    error = %e,
                    "ACP: agent processing failed"
                );

                Ok(json!({
                    "requestId": Uuid::new_v4().to_string(),
                    "status": "error",
                    "error": e.to_string()
                }))
            }
        }
    }

    /// Handle the `shutdown` method - graceful termination.
    async fn handle_shutdown(&self, _params: Option<JsonValue>) -> Result<JsonValue> {
        tracing::info!(
            session_id = %self.session_id,
            "ACP: shutdown requested"
        );

        Ok(json!({
            "shutdown": true
        }))
    }

    /// Process a single JSON-RPC request and return the response.
    async fn process_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();
        let method = request.method.clone();
        let params = request.params.clone();

        let result = match method.as_str() {
            "initialize" => self.handle_initialize(params).await,
            "text/append" => self.handle_text_append(params).await,
            "text/complete" => self.handle_text_complete(params).await,
            "prompt" => {
                // Use streaming for prompt - handle_prompt_streaming writes directly to stdout
                // Return a simple ack response
                match self.handle_prompt_streaming(params).await {
                    Ok(()) => Ok(json!({"status": "streaming"})),
                    Err(e) => {
                        // Return error as JSON-RPC response
                        Ok(json!({"status": "error", "error": e.to_string()}))
                    }
                }
            }
            "shutdown" => self.handle_shutdown(params).await,
            _ => {
                tracing::warn!(session_id = %self.session_id, method = %method, "ACP: unknown method");
                Err(anyhow::anyhow!("Unknown method: {}", method))
            }
        };

        match result {
            Ok(result_value) => JsonRpcResponse {
                jsonrpc: Some("2.0".to_string()),
                id,
                result: Some(result_value),
                error: None,
            },
            Err(e) => {
                tracing::error!(
                    session_id = %self.session_id,
                    method = %method,
                    error = %e,
                    "ACP: request failed"
                );
                JsonRpcResponse {
                    jsonrpc: Some("2.0".to_string()),
                    id,
                    result: None,
                    error: Some(JsonRpcError::internal_error_with_message(e.to_string())),
                }
            }
        }
    }

    /// Run the ACP handler - reads JSON-RPC from stdin and writes responses to stdout.
    pub async fn run(&self) -> Result<()> {
        tracing::info!(
            session_id = %self.session_id,
            "ACP handler starting - reading JSON-RPC from stdin"
        );

        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut stdout_lock = stdout.lock();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        for line_result in &mut lines {
            let line = line_result.context("Failed to read line from stdin")?;

            // Skip empty lines and comments
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Parse JSON-RPC request
            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    tracing::error!(
                        session_id = %self.session_id,
                        input = %line,
                        error = %e,
                        "ACP: failed to parse JSON-RPC request"
                    );

                    let error_response = JsonRpcResponse {
                        jsonrpc: Some("2.0".to_string()),
                        id: None,
                        result: None,
                        error: Some(JsonRpcError {
                            code: JsonRpcError::PARSE_ERROR_CODE,
                            message: format!("Parse error: {}", e),
                            data: None,
                        }),
                    };

                    let response_json =
                        serde_json::to_string(&error_response).context("Failed to serialize error")?;
                    writeln!(stdout_lock, "{}", response_json)
                        .context("Failed to write error response")?;
                    stdout_lock.flush().context("Failed to flush stdout")?;
                    continue;
                }
            };

            // Process the request
            let response = self.process_request(request).await;

            // Serialize and send response
            let response_json =
                serde_json::to_string(&response).context("Failed to serialize response")?;

            writeln!(stdout_lock, "{}", response_json).context("Failed to write response")?;
            stdout_lock.flush().context("Failed to flush stdout")?;

            // Check if client requested shutdown
            if let Some(ref result) = response.result {
                if result.get("shutdown").and_then(|v| v.as_bool()).unwrap_or(false) {
                    tracing::info!(
                        session_id = %self.session_id,
                        "ACP: shutting down per client request"
                    );
                    break;
                }
            }
        }

        tracing::info!(
            session_id = %self.session_id,
            "ACP handler terminated"
        );

        Ok(())
    }
}

/// Run the ACP protocol handler with proper logging setup.
///
/// This is the main entry point for the `zeroclaw acp` command.
///
/// Note: Logging is already initialized by main.rs via `init_logging()`,
/// which writes to ~/.zeroclaw/logs/zeroclaw.log. This preserves stdout
/// for JSON-RPC responses.
pub async fn run_acp() -> Result<()> {
    // Load ZeroClaw configuration
    let config = Config::load_or_init().await?;

    let handler = AcpHandler::new(config);
    handler.run().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_response_serialization() {
        let response = JsonRpcResponse {
            jsonrpc: Some("2.0".to_string()),
            id: Some(json!("test-id")),
            result: Some(json!({"status": "ok"})),
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"result\""));
        assert!(json.contains("\"status\""));
    }

    #[test]
    fn test_build_prompt_with_zed_instructions() {
        let prompt = "Hello, world!";
        let enriched = build_prompt_with_zed_instructions(prompt);

        assert!(enriched.contains("Zed Editor ACP mode"));
        assert!(enriched.contains("[[ZED:open:"));
        assert!(enriched.contains(prompt));
    }

    #[test]
    fn test_json_rpc_error_codes() {
        assert_eq!(JsonRpcError::PARSE_ERROR_CODE, -32700);
        assert_eq!(JsonRpcError::INVALID_REQUEST_CODE, -32600);
        assert_eq!(JsonRpcError::METHOD_NOT_FOUND_CODE, -32601);
        assert_eq!(JsonRpcError::INVALID_PARAMS_CODE, -32602);
        assert_eq!(JsonRpcError::INTERNAL_ERROR_CODE, -32603);
    }

    #[test]
    fn test_json_rpc_error_creation() {
        let error = JsonRpcError::new(100, "Test error");
        assert_eq!(error.code, 100);
        assert_eq!(error.message, "Test error");
        assert!(error.data.is_none());
    }

    #[test]
    fn test_json_rpc_error_standard_errors() {
        let parse_error = JsonRpcError::parse_error();
        assert_eq!(parse_error.code, -32700);
        assert_eq!(parse_error.message, "Parse error");

        let invalid_req = JsonRpcError::invalid_request();
        assert_eq!(invalid_req.code, -32600);

        let method_not_found = JsonRpcError::method_not_found();
        assert_eq!(method_not_found.code, -32601);

        let invalid_params = JsonRpcError::invalid_params();
        assert_eq!(invalid_params.code, -32602);

        let internal = JsonRpcError::internal_error();
        assert_eq!(internal.code, -32603);

        let custom = JsonRpcError::internal_error_with_message("Custom error");
        assert_eq!(custom.code, -32603);
        assert_eq!(custom.message, "Custom error");
    }

    // TODO: Add integration tests with mock Config for initialize, text_append, etc.
}
