//! Example tools demonstrating the new registration system.
//!
//! This module shows how tools can now register themselves automatically
//! using the register_tool! macro, eliminating manual registration in mod.rs.

use super::registry::register_tool;
use super::traits::{Tool, ToolResult};
use crate::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Example: Simple echo tool
///
/// This demonstrates the minimal registration pattern:
/// - Define the tool struct
/// - Implement the Tool trait
/// - Use register_tool! macro at module level
pub struct EchoTool;

impl EchoTool {
    pub fn new(_security: Arc<SecurityPolicy>) -> Self {
        Self
    }
}

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echo back the input text"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The text to echo back"
                }
            },
            "required": ["text"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'text' parameter"))?;

        Ok(ToolResult {
            success: true,
            output: text.to_string(),
            error: None,
        })
    }
}

// Auto-register this tool with the global registry
register_tool! {
    name: "echo",
    description: "Echo back the input text",
    parameters: {
        "type": "object",
        "properties": {
            "text": { "type": "string", "description": "The text to echo back" }
        },
        "required": ["text"]
    },
    factory: |security: Arc<SecurityPolicy>| -> Arc<dyn Tool> {
        Arc::new(EchoTool::new(security))
    }
}

/// Example: Tool with dependencies
///
/// This shows how to register tools that need additional context
/// beyond just SecurityPolicy. The factory can capture any needed state.
pub struct TimeTool {
    _security: Arc<SecurityPolicy>,
}

impl TimeTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { _security: security }
    }
}

#[async_trait]
impl Tool for TimeTool {
    fn name(&self) -> &str {
        "get_time"
    }

    fn description(&self) -> &str {
        "Get the current system time"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        use chrono::Local;

        let now = Local::now();
        Ok(ToolResult {
            success: true,
            output: now.to_rfc3339(),
            error: None,
        })
    }
}

// Auto-register this tool
register_tool! {
    name: "get_time",
    description: "Get the current system time",
    parameters: {
        "type": "object",
        "properties": {},
        "required": []
    },
    factory: |security: Arc<SecurityPolicy>| -> Arc<dyn Tool> {
        Arc::new(TimeTool::new(security))
    }
}

/// Example: Using the SchemaBuilder for cleaner schema definitions
///
/// This demonstrates using SchemaBuilder instead of manual json! macros.
pub struct GreetTool;

impl GreetTool {
    pub fn new(_security: Arc<SecurityPolicy>) -> Self {
        Self
    }
}

#[async_trait]
impl Tool for GreetTool {
    fn name(&self) -> &str {
        "greet"
    }

    fn description(&self) -> &str {
        "Greet a person by name"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        super::schema_builder::SchemaBuilder::new()
            .string_property("name")
            .description("name", "The name of the person to greet")
            .boolean_property("formal")
            .description("formal", "Use formal greeting style")
            .default("formal", json!(false))
            .required("name")
            .build()
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' parameter"))?;

        let formal = args
            .get("formal")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let greeting = if formal {
            format!("Good day, {}. How do you do?", name)
        } else {
            format!("Hey {}, what's up?", name)
        };

        Ok(ToolResult {
            success: true,
            output: greeting,
            error: None,
        })
    }
}

// Auto-register this tool
register_tool! {
    name: "greet",
    description: "Greet a person by name",
    parameters: {
        "type": "object",
        "properties": {
            "name": { "type": "string", "description": "The name of the person to greet" },
            "formal": { "type": "boolean", "description": "Use formal greeting style", "default": false }
        },
        "required": ["name"]
    },
    factory: |security: Arc<SecurityPolicy>| -> Arc<dyn Tool> {
        Arc::new(GreetTool::new(security))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_tool() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = EchoTool::new(security);

        assert_eq!(tool.name(), "echo");
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_time_tool() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = TimeTool::new(security);

        assert_eq!(tool.name(), "get_time");
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_greet_tool_schema() {
        let tool = GreetTool;
        let schema = tool.parameters_schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["name"].is_object());
        assert!(schema["properties"]["formal"].is_object());
        assert_eq!(schema["required"].as_array().unwrap().len(), 1);
    }
}
