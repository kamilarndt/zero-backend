//! OpenAI streaming conversion layer.
//!
//! Converts AgentStreamChunk to SSE format for OpenAI compatibility.

use crate::agent::AgentStreamChunk;
use crate::gateway::openai_sse_types::{DeltaChoice, DeltaDelta, SSEChunk, ToolCallDelta, ToolFunction};
use uuid::Uuid;

/// Convert an AgentStreamChunk to OpenAI SSE format.
///
/// # Arguments
/// * `chunk` - The agent stream chunk to convert
/// * `model` - The model name to include in the SSE response
///
/// # Returns
/// An `SSEChunk` ready for serialization to SSE format
pub fn convert_chunk_to_sse(chunk: &AgentStreamChunk, model: &str) -> SSEChunk {
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let id = Uuid::new_v4().to_string();

    match chunk {
        AgentStreamChunk::Text(text) => SSEChunk {
            id,
            object: "chat.completion.chunk",
            created,
            model: model.to_string(),
            choices: vec![DeltaChoice {
                index: 0,
                delta: DeltaDelta {
                    content: Some(text.clone()),
                    tool_calls: None,
                },
                finish_reason: None,
            }],
        },

        AgentStreamChunk::ToolStart {
            tool_id,
            tool_name,
            args,
        } => {
            // OpenAI format: tool calls are emitted incrementally
            // First chunk: just the id and type
            SSEChunk {
                id,
                object: "chat.completion.chunk",
                created,
                model: model.to_string(),
                choices: vec![DeltaChoice {
                    index: 0,
                    delta: DeltaDelta {
                        content: None,
                        tool_calls: Some(vec![ToolCallDelta {
                            index: 0,
                            id: tool_id.clone(),
                            tool_type: "function",
                            function: ToolFunction {
                                name: tool_name.clone(),
                                arguments: String::new(), // Empty initially
                            },
                        }]),
                    },
                    finish_reason: None,
                }],
            }
        }

        AgentStreamChunk::ToolResult { tool_id: _, result } => {
            // Tool results are typically sent as additional assistant messages
            // For streaming, we include the result in the content
            SSEChunk {
                id,
                object: "chat.completion.chunk",
                created,
                model: model.to_string(),
                choices: vec![DeltaChoice {
                    index: 0,
                    delta: DeltaDelta {
                        content: Some(format!("\n\nTool result: {}\n\n", result)),
                        tool_calls: None,
                    },
                    finish_reason: None,
                }],
            }
        }

        AgentStreamChunk::ToolEnd => {
            // Send a chunk to indicate tool execution is complete
            SSEChunk {
                id,
                object: "chat.completion.chunk",
                created,
                model: model.to_string(),
                choices: vec![DeltaChoice {
                    index: 0,
                    delta: DeltaDelta {
                        content: None,
                        tool_calls: None,
                    },
                    finish_reason: None,
                }],
            }
        }

        AgentStreamChunk::Done => {
            // Final chunk with finish_reason
            SSEChunk {
                id,
                object: "chat.completion.chunk",
                created,
                model: model.to_string(),
                choices: vec![DeltaChoice {
                    index: 0,
                    delta: DeltaDelta {
                        content: Some(String::new()),
                        tool_calls: None,
                    },
                    finish_reason: Some("stop"),
                }],
            }
        }
    }
}

/// Format an SSEChunk as a Server-Sent Event line.
///
/// # Arguments
/// * `chunk` - The SSE chunk to format
///
/// # Returns
/// A string formatted as an SSE event (with "data: " prefix)
pub fn format_sse_event(chunk: &SSEChunk) -> String {
    let json = serde_json::to_string(chunk).unwrap();
    format!("data: {}\n\n", json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_text_chunk() {
        let chunk = AgentStreamChunk::Text("Hello, world!".to_string());
        let sse = convert_chunk_to_sse(&chunk, "test-model");

        assert_eq!(sse.object, "chat.completion.chunk");
        assert_eq!(sse.model, "test-model");
        assert_eq!(sse.choices.len(), 1);
        assert_eq!(sse.choices[0].index, 0);
        assert_eq!(sse.choices[0].delta.content, Some("Hello, world!".to_string()));
        assert!(sse.choices[0].delta.tool_calls.is_none());
        assert!(sse.choices[0].finish_reason.is_none());
    }

    #[test]
    fn test_convert_tool_start_chunk() {
        let chunk = AgentStreamChunk::ToolStart {
            tool_id: "call_123".to_string(),
            tool_name: "search".to_string(),
            args: serde_json::json!({"query": "test"}),
        };
        let sse = convert_chunk_to_sse(&chunk, "test-model");

        assert_eq!(sse.object, "chat.completion.chunk");
        assert_eq!(sse.choices.len(), 1);
        assert!(sse.choices[0].delta.content.is_none());
        assert!(sse.choices[0].delta.tool_calls.is_some());

        let tool_calls = sse.choices[0].delta.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_123");
        assert_eq!(tool_calls[0].tool_type, "function");
        assert_eq!(tool_calls[0].function.name, "search");
        assert_eq!(tool_calls[0].function.arguments, ""); // Empty initially
    }

    #[test]
    fn test_convert_tool_result_chunk() {
        let chunk = AgentStreamChunk::ToolResult {
            tool_id: "call_123".to_string(),
            result: "Found 5 results".to_string(),
        };
        let sse = convert_chunk_to_sse(&chunk, "test-model");

        assert_eq!(sse.choices.len(), 1);
        let content = sse.choices[0].delta.content.as_ref().unwrap();
        assert!(content.contains("Tool result"));
        assert!(content.contains("Found 5 results"));
    }

    #[test]
    fn test_convert_done_chunk() {
        let chunk = AgentStreamChunk::Done;
        let sse = convert_chunk_to_sse(&chunk, "test-model");

        assert_eq!(sse.choices.len(), 1);
        assert_eq!(sse.choices[0].delta.content, Some("".to_string()));
        assert_eq!(sse.choices[0].finish_reason, Some("stop"));
    }

    #[test]
    fn test_format_sse_event() {
        let chunk = SSEChunk {
            id: "test-id".to_string(),
            object: "chat.completion.chunk",
            created: 1234567890,
            model: "test-model".to_string(),
            choices: vec![DeltaChoice {
                index: 0,
                delta: DeltaDelta {
                    content: Some("Hello".to_string()),
                    tool_calls: None,
                },
                finish_reason: None,
            }],
        };

        let formatted = format_sse_event(&chunk);
        assert!(formatted.starts_with("data: "));
        assert!(formatted.ends_with("\n\n"));

        // Verify it's valid JSON after stripping prefix
        let json_str = formatted.strip_prefix("data: ").unwrap().trim();
        let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed["object"], "chat.completion.chunk");
        assert_eq!(parsed["choices"][0]["delta"]["content"], "Hello");
    }

    #[test]
    fn test_tool_end_empty_chunk() {
        let chunk = AgentStreamChunk::ToolEnd;
        let sse = convert_chunk_to_sse(&chunk, "test-model");

        assert_eq!(sse.choices.len(), 1);
        assert!(sse.choices[0].delta.content.is_none());
        assert!(sse.choices[0].delta.tool_calls.is_none());
        assert!(sse.choices[0].finish_reason.is_none());
    }
}
