//! emit_event tool - sends events to dashboard via SSE broadcast
//!
//! Agents use this tool to push real-time updates to the web dashboard.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::tools::traits::Tool;

#[derive(Debug, Deserialize)]
struct EmitEventParams {
    /// Event type identifier (e.g. "widget_update", "log_entry")
    event_type: String,
    /// Optional target widget ID — becomes the 'from' field
    #[serde(default)]
    target_widget: Option<String>,
    /// Arbitrary event payload
    payload: serde_json::Value,
}

/// Tool for emitting events to the dashboard SSE stream
pub struct EmitEventTool;

#[async_trait]
impl Tool for EmitEventTool {
    fn name(&self) -> &str {
        "emit_event"
    }

    fn description(&self) -> &str {
        "Emit an event to the web dashboard via SSE broadcast channel. Use for real-time updates."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["event_type", "payload"],
            "properties": {
                "event_type": {
                    "type": "string",
                    "description": "Event type identifier (e.g., 'widget_update', 'log_entry', 'agent_action')"
                },
                "target_widget": {
                    "type": "string",
                    "description": "Optional target widget ID (becomes 'from' field in the event)"
                },
                "payload": {
                    "type": "object",
                    "description": "Event data payload (any JSON)"
                }
            }
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> anyhow::Result<crate::tools::traits::ToolResult> {
        let p: EmitEventParams =
            serde_json::from_value(args).map_err(|e| anyhow::anyhow!("Invalid params: {}", e))?;

        // Construct event with timestamp
        let event = json!({
            "type": p.event_type,
            "from": p.target_widget,
            "to": null,
            "data": p.payload,
            "timestamp": chrono::Utc::now().timestamp_millis(),
        });

        // Send via HTTP POST to the events endpoint
        // This is safer than direct broadcast access and works across contexts
        let client = reqwest::Client::new();
        let resp = client
            .post("http://127.0.0.1:42618/v1/events")
            .json(&event)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        let success = resp.status().is_success();

        Ok(crate::tools::traits::ToolResult {
            success,
            output: if success {
                format!("Event '{}' emitted successfully", p.event_type)
            } else {
                format!("Failed to emit event: {}", resp.status())
            },
            error: if success {
                None
            } else {
                Some("HTTP error".into())
            },
        })
    }
}
