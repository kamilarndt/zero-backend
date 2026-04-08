//! Tool for updating World Monitor dashboard widgets.
//!
//! Sends widget data to the gateway API for SSE broadcast to connected browsers.

use super::traits::{Tool, ToolResult};
use crate::security::{policy::ToolOperation, SecurityPolicy};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Widget update tool for agents to push data to the dashboard
pub struct UpdateDashboardWidgetTool {
    security: Arc<SecurityPolicy>,
    gateway_url: String,
}

impl UpdateDashboardWidgetTool {
    pub fn new(security: Arc<SecurityPolicy>, gateway_port: u16) -> Self {
        Self {
            security,
            gateway_url: format!("http://localhost:{}/v1/widgets", gateway_port),
        }
    }
}

#[async_trait]
impl Tool for UpdateDashboardWidgetTool {
    fn name(&self) -> &str {
        "update_dashboard_widget"
    }

    fn description(&self) -> &str {
        "Update a widget on the World Monitor dashboard. Send real-time data to browsers viewing the dashboard."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "widget_id": {
                    "type": "string",
                    "description": "Widget identifier (e.g., 'systemStatus', 'agentLog', 'taskQueue')"
                },
                "data": {
                    "description": "Widget data - can be any JSON object: number, string, array, or nested object",
                    "oneOf": [
                        {"type": "object"},
                        {"type": "array"},
                        {"type": "string"},
                        {"type": "number"}
                    ]
                }
            },
            "required": ["widget_id", "data"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // Security check
        if let Err(error) = self
            .security
            .enforce_tool_operation(ToolOperation::Act, self.name())
        {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error),
            });
        }

        let widget_id = args
            .get("widget_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'widget_id' parameter"))?;

        let data = args
            .get("data")
            .ok_or_else(|| anyhow::anyhow!("Missing 'data' parameter"))?;

        let url = format!("{}/{}", self.gateway_url, widget_id);

        // Send POST request to gateway
        let client = reqwest::Client::new();
        let payload = json!({ "data": data });

        match client
            .post(&url)
            .json(&payload)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => Ok(ToolResult {
                success: true,
                output: format!("Widget '{}' updated successfully", widget_id),
                error: None,
            }),
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Gateway returned {}: {}", status, body)),
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to reach gateway: {}", e)),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_and_schema() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = UpdateDashboardWidgetTool::new(security, 42617);
        assert_eq!(tool.name(), "update_dashboard_widget");
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["widget_id"].is_object());
        assert!(schema["properties"]["data"].is_object());
    }

    #[test]
    fn gateway_url_format() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = UpdateDashboardWidgetTool::new(security, 42617);
        assert_eq!(tool.gateway_url, "http://localhost:42617/v1/widgets");
    }
}
