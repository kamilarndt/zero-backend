//! SSE (Server-Sent Events) type definitions for OpenAI compatibility.
//!
//! These types match the OpenAI chat.completion.chunk format for streaming responses.

use serde::Serialize;

/// SSE response chunk matching OpenAI format
#[derive(Debug, Serialize)]
pub struct SSEChunk {
    pub id: String,
    pub object: &'static str,  // "chat.completion.chunk"
    pub created: u64,
    pub model: String,
    pub choices: Vec<DeltaChoice>,
}

#[derive(Debug, Serialize)]
pub struct DeltaChoice {
    pub index: u32,
    pub delta: DeltaDelta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct DeltaDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Debug, Serialize)]
pub struct ToolCallDelta {
    pub index: u32,
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: &'static str,  // "function"
    pub function: ToolFunction,
}

#[derive(Debug, Serialize)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_chunk_serialization() {
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

        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("chat.completion.chunk"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_sse_chunk_with_tool_calls() {
        let chunk = SSEChunk {
            id: "test-id".to_string(),
            object: "chat.completion.chunk",
            created: 1234567890,
            model: "test-model".to_string(),
            choices: vec![DeltaChoice {
                index: 0,
                delta: DeltaDelta {
                    content: None,
                    tool_calls: Some(vec![ToolCallDelta {
                        index: 0,
                        id: "call_123".to_string(),
                        tool_type: "function",
                        function: ToolFunction {
                            name: "search".to_string(),
                            arguments: "{}".to_string(),
                        },
                    }]),
                },
                finish_reason: None,
            }],
        };

        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("tool_calls"));
        assert!(json.contains("search"));
    }

    #[test]
    fn test_finish_reason_none_when_empty() {
        let choice = DeltaChoice {
            index: 0,
            delta: DeltaDelta {
                content: Some("test".to_string()),
                tool_calls: None,
            },
            finish_reason: None,
        };

        let json = serde_json::to_string(&choice).unwrap();
        // finish_reason should be skipped when None
        assert!(!json.contains("finish_reason"));
    }

    #[test]
    fn test_finish_reason_included_when_set() {
        let choice = DeltaChoice {
            index: 0,
            delta: DeltaDelta {
                content: Some("test".to_string()),
                tool_calls: None,
            },
            finish_reason: Some("stop"),
        };

        let json = serde_json::to_string(&choice).unwrap();
        assert!(json.contains("finish_reason"));
        assert!(json.contains("stop"));
    }
}
