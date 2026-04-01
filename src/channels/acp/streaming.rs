//! True streaming implementation for ACP protocol.
//!
//! This module implements true streaming that consumes provider token streams directly,
//! with buffering to detect and extract Zed commands from the stream.

use anyhow::{Context, Result};
use futures_util::{stream, Stream, StreamExt};
use serde_json::json;
use std::pin::Pin;
use std::sync::Arc;

use crate::config::Config;
use crate::providers::{Provider, create_routed_provider};
use crate::providers::traits::{StreamChunk, StreamOptions};

use super::commands::{ZedCommand, parse_zed_command};

/// Maximum buffer size for Zed command detection (in bytes).
const MAX_COMMAND_BUFFER_SIZE: usize = 256;

/// State machine for Zed command buffering.
#[derive(Debug, Clone, PartialEq)]
enum BufferState {
    /// No potential command detected
    Idle,
    /// Detected "[[" but not yet confirmed as ZED command
    PotentialCommand,
    /// Detected "[[ZED:" and actively buffering until "]]"
    BufferingCommand,
}

/// Buffers stream chunks to detect and extract Zed commands.
///
/// This buffer processes incoming text chunks and:
/// 1. Immediately flushes non-command text
/// 2. Detects Zed command patterns (`[[ZED:...]]`)
/// 3. Handles commands split across multiple chunks
/// 4. Protects against buffer overflow attacks
pub struct ZedCommandBuffer {
    state: BufferState,
    buffer: String,
    commands: Vec<ZedCommand>,
    flushed_text: String,
}

impl ZedCommandBuffer {
    /// Create a new command buffer.
    pub fn new() -> Self {
        Self {
            state: BufferState::Idle,
            buffer: String::new(),
            commands: Vec::new(),
            flushed_text: String::new(),
        }
    }

    /// Process a chunk of text from the stream.
    ///
    /// Returns text that should be immediately flushed to the client.
    /// Commands are buffered and returned via `finalize()`.
    pub fn process_chunk(&mut self, chunk: &str) -> String {
        let mut to_flush = String::new();

        for ch in chunk.chars() {
            match self.state {
                BufferState::Idle => {
                    if ch == '[' {
                        self.state = BufferState::PotentialCommand;
                        self.buffer.push(ch);
                    } else {
                        to_flush.push(ch);
                    }
                }
                BufferState::PotentialCommand => {
                    self.buffer.push(ch);
                    if self.buffer == "[[" {
                        // Now we have a potential command
                        self.state = BufferState::BufferingCommand;
                    } else if !self.buffer.starts_with('[') {
                        // Not actually a command start, flush buffer
                        to_flush.push_str(&self.buffer);
                        self.buffer.clear();
                        self.state = BufferState::Idle;
                        // Re-process this character
                        if ch == '[' {
                            self.state = BufferState::PotentialCommand;
                            self.buffer.push(ch);
                        } else {
                            to_flush.push(ch);
                        }
                    } else if self.buffer.len() > MAX_COMMAND_BUFFER_SIZE {
                        // Buffer overflow, flush as text and reset
                        to_flush.push_str(&self.buffer);
                        self.buffer.clear();
                        self.state = BufferState::Idle;
                    }
                }
                BufferState::BufferingCommand => {
                    self.buffer.push(ch);

                    // Check for command completion
                    if self.buffer.ends_with("]]") {
                        // Try to parse as Zed command
                        let maybe_command = parse_zed_command(&self.buffer);

                        if let Some(cmd) = maybe_command {
                            self.commands.push(cmd);
                            // Command extracted, don't flush to output
                        } else {
                            // Not a valid Zed command, flush as text
                            to_flush.push_str(&self.buffer);
                        }

                        self.buffer.clear();
                        self.state = BufferState::Idle;
                    } else if self.buffer.len() > MAX_COMMAND_BUFFER_SIZE {
                        // Buffer overflow, flush as text and reset
                        to_flush.push_str(&self.buffer);
                        self.buffer.clear();
                        self.state = BufferState::Idle;
                    } else if self.buffer.len() >= 6 && !self.buffer.starts_with("[[ZED:") {
                        // Not a ZED command after all, flush buffer
                        to_flush.push_str(&self.buffer);
                        self.buffer.clear();
                        self.state = BufferState::Idle;
                    }
                }
            }
        }

        self.flushed_text.push_str(&to_flush);
        to_flush
    }

    /// Finalize the buffer and return any remaining content.
    ///
    /// Returns (all_flushed_text, all_commands)
    pub fn finalize(self) -> (String, Vec<ZedCommand>) {
        let mut final_text = self.flushed_text;

        // Any remaining buffer content is flushed as text
        if !self.buffer.is_empty() {
            final_text.push_str(&self.buffer);
        }

        (final_text, self.commands)
    }

    /// Finalize the buffer from a mutable reference.
    /// This consumes the buffer state but allows calling when behind a reference.
    pub fn finalize_ref(&mut self) -> (String, Vec<ZedCommand>) {
        let mut final_text = std::mem::take(&mut self.flushed_text);

        // Any remaining buffer content is flushed as text
        if !self.buffer.is_empty() {
            final_text.push_str(&std::mem::take(&mut self.buffer));
        }

        let commands = std::mem::take(&mut self.commands);
        (final_text, commands)
    }

    /// Get all commands collected so far (without finalizing).
    pub fn commands(&self) -> &[ZedCommand] {
        &self.commands
    }
}

impl Default for ZedCommandBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming processor that consumes provider streams.
///
/// Processes streaming chunks from a provider, buffering to detect
/// Zed commands while immediately forwarding text deltas.
pub struct StreamProcessor {
    request_id: String,
    buffer: ZedCommandBuffer,
    is_complete: bool,
}

impl StreamProcessor {
    /// Create a new stream processor.
    pub fn new(request_id: String) -> Self {
        Self {
            request_id,
            buffer: ZedCommandBuffer::new(),
            is_complete: false,
        }
    }

    /// Process a chunk from the provider stream.
    ///
    /// Returns a JSON string to send to the client, or None if nothing to send.
    pub fn process_chunk(&mut self, chunk: &StreamChunk) -> Option<String> {
        if chunk.is_final {
            self.is_complete = true;
            return None;
        }

        // Process through Zed command buffer
        let text_delta = self.buffer.process_chunk(&chunk.delta);

        if text_delta.is_empty() {
            None
        } else {
            Some(json!({
                "jsonrpc": "2.0",
                "method": "progress",
                "params": {
                    "requestId": self.request_id,
                    "type": "text/delta",
                    "text": text_delta
                }
            }).to_string())
        }
    }

    /// Finalize the stream and return the completion message.
    pub fn finalize(self) -> (String, Vec<ZedCommand>) {
        self.buffer.finalize()
    }

    /// Build the final completion message.
    pub fn build_final_message(self) -> String {
        let request_id = self.request_id.clone();
        let (full_text, zed_commands) = self.finalize();

        json!({
            "requestId": request_id,
            "status": "complete",
            "response": full_text,
            "zedCommands": zed_commands
        }).to_string()
    }

    /// Build the final completion message from a mutable reference.
    /// This is useful when the processor is behind a MutexGuard.
    pub fn build_final_message_ref(&mut self) -> String {
        let request_id = self.request_id.clone();
        let (full_text, zed_commands) = self.buffer.finalize_ref();

        // Reset state after finalization
        self.is_complete = true;

        json!({
            "requestId": request_id,
            "status": "complete",
            "response": full_text,
            "zedCommands": zed_commands
        }).to_string()
    }
}

/// Create a streaming-capable provider.
///
/// Returns an error if the provider doesn't support streaming.
pub async fn create_streaming_provider(config: &Config) -> Result<Arc<dyn Provider>> {
    let provider = create_routed_provider(
        &config.default_provider.as_deref().unwrap_or("openrouter"),
        config.api_key.as_deref(),
        config.api_url.as_deref(),
        &config.reliability,
        &config.model_routes,
        &config.default_model.as_deref().unwrap_or("gpt-4"),
    ).context("Failed to create provider")?;

    // Check if provider supports streaming
    if !provider.supports_streaming() {
        anyhow::bail!(
            "Provider '{}' does not support streaming",
            config.default_provider.as_deref().unwrap_or("unknown")
        );
    }

    Ok(Arc::from(provider))
}

/// True streaming implementation for ACP prompts.
///
/// This consumes the provider's token stream directly and processes it
/// through the Zed command buffer, emitting JSON-RPC progress notifications.
pub async fn process_prompt_true_streaming(
    config: &Config,
    prompt: &str,
    request_id: String,
) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
    // Create streaming provider
    let provider = create_streaming_provider(config).await?;

    // Get model from config
    let model = config.default_model.as_deref().unwrap_or("gpt-4");

    // Create stream processor
    let processor = Arc::new(tokio::sync::Mutex::new(StreamProcessor::new(request_id.clone())));
    let processor_clone = processor.clone();

    // Start the streaming chat
    let stream = provider.stream_chat_with_system(
        None,
        prompt,
        model,
        config.default_temperature,
        StreamOptions::new(true),
    );

    // Process the stream
    let processed_stream = stream.then(move |chunk_result| {
        let processor = processor.clone();
        async move {
            match chunk_result {
                Ok(chunk) => {
                    let mut proc = processor.lock().await;
                    if let Some(json_msg) = proc.process_chunk(&chunk) {
                        Ok(json_msg)
                    } else {
                        // Nothing to send for this chunk
                        Ok(String::new())
                    }
                }
                Err(e) => {
                    // Stream error
                    Err(anyhow::anyhow!("Stream error: {}", e))
                }
            }
        }
    });

    // Add final completion message
    let final_stream = processed_stream.chain(stream::once(async move {
        let mut proc = processor_clone.lock().await;
        Ok(proc.build_final_message_ref())
    }));

    // Filter out empty messages using filter_map
    let filtered_stream = final_stream.filter_map(|result| async move {
        match result {
            Ok(msg) if !msg.is_empty() => Some(Ok(msg)),
            Ok(_) => None, // Filter out empty messages
            Err(e) => Some(Err(e)), // Keep errors
        }
    });

    Ok(Box::pin(filtered_stream))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_no_command() {
        let mut buffer = ZedCommandBuffer::new();
        let output = buffer.process_chunk("Hello world");
        assert_eq!(output, "Hello world");
        assert!(buffer.commands().is_empty());
    }

    #[test]
    fn test_buffer_complete_command() {
        let mut buffer = ZedCommandBuffer::new();
        let output = buffer.process_chunk("Check this [[ZED:open:test.txt:42]] file");
        assert_eq!(output, "Check this  file");
        assert_eq!(buffer.commands().len(), 1);

        match &buffer.commands()[0] {
            ZedCommand::OpenFile { path, line, .. } => {
                assert_eq!(path, "test.txt");
                assert_eq!(*line, Some(42));
            }
            _ => panic!("Expected OpenFile command"),
        }
    }

    #[test]
    fn test_buffer_command_split_across_chunks() {
        let mut buffer = ZedCommandBuffer::new();

        // First chunk ends mid-command
        let output1 = buffer.process_chunk("Text [[ZED:op");
        assert_eq!(output1, "Text ");
        assert!(buffer.commands().is_empty());

        // Second chunk completes command
        let output2 = buffer.process_chunk("en:test.txt]] more");
        assert_eq!(output2, " more");
        assert_eq!(buffer.commands().len(), 1);
    }

    #[test]
    fn test_buffer_false_positive() {
        let mut buffer = ZedCommandBuffer::new();
        let output = buffer.process_chunk("Text with [[brackets]] but not command");
        assert_eq!(output, "Text with [[brackets]] but not command");
        assert!(buffer.commands().is_empty());
    }

    #[test]
    fn test_buffer_triple_bracket() {
        let mut buffer = ZedCommandBuffer::new();
        let output = buffer.process_chunk("Text with [[[triple]] brackets");
        // Should handle gracefully - the extra bracket might cause some buffering
        // but should eventually flush
        assert!(!output.is_empty());
    }

    #[test]
    fn test_buffer_ends_at_double_bracket() {
        let mut buffer = ZedCommandBuffer::new();

        // Chunk ends right at "[["
        let output1 = buffer.process_chunk("Text [[");
        assert_eq!(output1, "Text ");

        // Next chunk continues
        let output2 = buffer.process_chunk("ZED:open:test.txt]] done");
        assert!(output2.contains("done"));
        assert_eq!(buffer.commands().len(), 1);
    }

    #[test]
    fn test_buffer_ends_mid_command() {
        let mut buffer = ZedCommandBuffer::new();

        // Chunk ends mid-command
        let output1 = buffer.process_chunk("Text [[ZED:op");
        assert_eq!(output1, "Text ");
        assert!(buffer.commands().is_empty());

        // Next chunk has more content
        let output2 = buffer.process_chunk("en:test.txt");
        assert!(output2.is_empty());
        assert!(buffer.commands().is_empty());

        // Final chunk closes command
        let output3 = buffer.process_chunk("]] done");
        assert!(output3.contains("done"));
        assert_eq!(buffer.commands().len(), 1);
    }

    #[test]
    fn test_buffer_overflow_protection() {
        let mut buffer = ZedCommandBuffer::new();

        // Create a buffer that would overflow
        let long_text = "[[".repeat(200); // 400 characters
        let output = buffer.process_chunk(&long_text);

        // Should flush to prevent overflow
        assert!(!output.is_empty());
        assert!(buffer.commands().is_empty());
    }

    #[test]
    fn test_buffer_multiple_commands_in_stream() {
        let mut buffer = ZedCommandBuffer::new();
        let output = buffer.process_chunk(
            "Check [[ZED:open:file1.txt]] and [[ZED:open:file2.txt:10]] done"
        );

        assert_eq!(output, "Check  and  done");
        assert_eq!(buffer.commands().len(), 2);
    }

    #[test]
    fn test_buffer_finalize() {
        let mut buffer = ZedCommandBuffer::new();

        buffer.process_chunk("Text [[ZED:open:test.txt]] more");

        let (full_text, commands) = buffer.finalize();
        assert_eq!(full_text, "Text  more");
        assert_eq!(commands.len(), 1);
    }

    #[test]
    fn test_stream_processor_new() {
        let processor = StreamProcessor::new("test-id".to_string());
        assert!(!processor.is_complete);
        assert!(processor.buffer.commands().is_empty());
    }

    #[test]
    fn test_stream_processor_process_chunk() {
        let mut processor = StreamProcessor::new("test-id".to_string());

        let chunk = StreamChunk {
            delta: "Hello world".to_string(),
            is_final: false,
            token_count: 2,
        };

        let json_msg = processor.process_chunk(&chunk);
        assert!(json_msg.is_some());

        let msg = json_msg.unwrap();
        assert!(msg.contains("\"method\":\"progress\""));
        assert!(msg.contains("Hello world"));
    }

    #[test]
    fn test_stream_processor_final_chunk() {
        let mut processor = StreamProcessor::new("test-id".to_string());

        let chunk = StreamChunk {
            delta: String::new(),
            is_final: true,
            token_count: 0,
        };

        let json_msg = processor.process_chunk(&chunk);
        assert!(json_msg.is_none());
        assert!(processor.is_complete);
    }

    #[test]
    fn test_stream_processor_command_extraction() {
        let mut processor = StreamProcessor::new("test-id".to_string());

        let chunk = StreamChunk {
            delta: "Check [[ZED:open:test.txt:42]]".to_string(),
            is_final: false,
            token_count: 5,
        };

        let json_msg = processor.process_chunk(&chunk);
        assert!(json_msg.is_some());

        // Command should be extracted
        let (text, commands) = processor.finalize();
        assert!(!text.contains("[[ZED:"));
        assert_eq!(commands.len(), 1);
    }

    #[test]
    fn test_stream_processor_build_final_message() {
        let mut processor = StreamProcessor::new("test-123".to_string());

        let chunk = StreamChunk {
            delta: "Hello [[ZED:open:test.txt]] world".to_string(),
            is_final: false,
            token_count: 5,
        };

        processor.process_chunk(&chunk);

        let final_msg = processor.build_final_message();
        assert!(final_msg.contains("\"requestId\":\"test-123\""));
        assert!(final_msg.contains("\"status\":\"complete\""));
        assert!(final_msg.contains("\"response\""));
        assert!(final_msg.contains("\"zedCommands\""));
    }
}
