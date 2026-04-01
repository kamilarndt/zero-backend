//! Siyuan integration tools for local knowledge base interaction.
//!
//! Siyuan is a local block-based knowledge base running on localhost:6806.
//! This module provides tools for querying and writing to Siyuan.

use super::traits::{Tool, ToolResult};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

/// Base URL for Siyuan API
const SIYUAN_BASE_URL: &str = "http://localhost:6806";

/// Siyuan API client with automatic authentication
#[derive(Debug, Clone)]
pub struct SiyuanClient {
    client: Client,
    api_token: String,
}

impl SiyuanClient {
    /// Create a new Siyuan client by reading the API token from environment
    pub fn from_env() -> anyhow::Result<Self> {
        let api_token = std::env::var("SIYUAN_API_TOKEN")
            .unwrap_or_else(|_| "".to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self { client, api_token })
    }

    /// Create a new Siyuan client with explicit token
    pub fn new(api_token: String) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self { client, api_token })
    }

    /// Send a POST request to the Siyuan API
    async fn post<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        payload: &T,
    ) -> anyhow::Result<R> {
        let url = format!("{}{}", SIYUAN_BASE_URL, endpoint);

        let mut request = self.client.post(&url).json(payload);

        // Add Authorization header if token is present
        if !self.api_token.is_empty() {
            request = request.header("Authorization", format!("Token {}", self.api_token));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Siyuan API error: {} - {}", status, error_text);
        }

        let result = response.json().await?;
        Ok(result)
    }

    /// Execute a SQL query on Siyuan
    pub async fn query_sql(&self, stmt: &str) -> anyhow::Result<SiyuanQueryResponse> {
        let payload = json!({
            "stmt": stmt
        });

        self.post("/api/query/sql", &payload).await
    }

    /// Insert a markdown block as a child of a parent block
    pub async fn insert_block(
        &self,
        content: &str,
        parent_id: &str,
    ) -> anyhow::Result<SiyuanInsertResponse> {
        let payload = json!({
            "dataType": "markdown",
            "data": content,
            "parentID": parent_id
        });

        self.post("/api/block/insertBlock", &payload).await
    }
}

/// Siyuan query API response
#[derive(Debug, Serialize, Deserialize)]
pub struct SiyuanQueryResponse {
    pub code: i32,
    pub msg: String,
    pub data: Option<serde_json::Value>,
}

/// Siyuan insert block API response
#[derive(Debug, Serialize, Deserialize)]
pub struct SiyuanInsertResponse {
    pub code: i32,
    pub msg: String,
    pub data: Option<serde_json::Value>,
}

/// Tool for querying Siyuan via SQL
#[derive(Debug, Clone)]
pub struct SiyuanQueryTool {
    client: SiyuanClient,
}

impl SiyuanQueryTool {
    /// Create a new Siyuan query tool
    pub fn new() -> anyhow::Result<Self> {
        let client = SiyuanClient::from_env()?;
        Ok(Self { client })
    }

    /// Create a new Siyuan query tool with explicit token
    pub fn with_token(api_token: String) -> anyhow::Result<Self> {
        let client = SiyuanClient::new(api_token)?;
        Ok(Self { client })
    }
}

#[async_trait]
impl Tool for SiyuanQueryTool {
    fn name(&self) -> &str {
        "siyuan_query"
    }

    fn description(&self) -> &str {
        "Query the Siyuan knowledge base using SQL. Siyuan is the local knowledge brain running on localhost:6806. \
         **IMPORTANT:** Always use siyuan_query for ALL Siyuan operations - do NOT try to manually debug Siyuan using curl/bash or edit SiyuanPro/index.js. \
         Examples: 'SELECT * FROM blocks WHERE type=\"d\"' (list documents), 'SELECT id, content FROM blocks WHERE content LIKE \"%keyword%\"' (search), 'SELECT * FROM notebooks' (list notebooks)."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "SQL query to execute on Siyuan database (e.g., SELECT * FROM blocks WHERE content LIKE '%keyword%')"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;

        match self.client.query_sql(query).await {
            Ok(response) => {
                if response.code == 0 {
                    let output = serde_json::to_string_pretty(&response.data)
                        .unwrap_or_else(|_| "Success (invalid JSON)".to_string());
                    Ok(ToolResult {
                        success: true,
                        output,
                        error: None,
                    })
                } else {
                    Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(response.msg),
                    })
                }
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Siyuan query failed: {}", e)),
            }),
        }
    }
}

/// Tool for writing markdown blocks to Siyuan
#[derive(Debug, Clone)]
pub struct SiyuanWriteTool {
    client: SiyuanClient,
}

impl SiyuanWriteTool {
    /// Create a new Siyuan write tool
    pub fn new() -> anyhow::Result<Self> {
        let client = SiyuanClient::from_env()?;
        Ok(Self { client })
    }

    /// Create a new Siyuan write tool with explicit token
    pub fn with_token(api_token: String) -> anyhow::Result<Self> {
        let client = SiyuanClient::new(api_token)?;
        Ok(Self { client })
    }
}

#[async_trait]
impl Tool for SiyuanWriteTool {
    fn name(&self) -> &str {
        "siyuan_write"
    }

    fn description(&self) -> &str {
        "Write a markdown block to the Siyuan knowledge base. **IMPORTANT:** Always use siyuan_write for ALL Siyuan write operations - do NOT try to manually debug Siyuan using curl/bash or edit SiyuanPro/index.js. \
         Use this to document completed tasks, summaries, or notes. Always format content as proper markdown before writing."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "Markdown content to write (e.g., '# Task Summary\\n\\nCompleted the implementation...')"
                },
                "parent_id": {
                    "type": "string",
                    "description": "Target block ID to append content to (the parent block)"
                }
            },
            "required": ["content", "parent_id"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' parameter"))?;

        let parent_id = args
            .get("parent_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'parent_id' parameter"))?;

        match self.client.insert_block(content, parent_id).await {
            Ok(response) => {
                if response.code == 0 {
                    Ok(ToolResult {
                        success: true,
                        output: format!("Written to Siyuan under parent block: {}", parent_id),
                        error: None,
                    })
                } else {
                    Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(response.msg),
                    })
                }
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Siyuan write failed: {}", e)),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_siyuan_client_creation() {
        let client = SiyuanClient::new("test_token".to_string());
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.api_token, "test_token");
    }

    #[test]
    fn test_siyuan_query_tool_name() {
        let tool = SiyuanQueryTool::new().unwrap();
        assert_eq!(tool.name(), "siyuan_query");
    }

    #[test]
    fn test_siyuan_write_tool_name() {
        let tool = SiyuanWriteTool::new().unwrap();
        assert_eq!(tool.name(), "siyuan_write");
    }

    #[test]
    fn test_siyuan_query_schema() {
        let tool = SiyuanQueryTool::new().unwrap();
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
        assert_eq!(schema["required"].as_array().unwrap()[0], "query");
    }

    #[test]
    fn test_siyuan_write_schema() {
        let tool = SiyuanWriteTool::new().unwrap();
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["content"].is_object());
        assert!(schema["properties"]["parent_id"].is_object());
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("content")));
        assert!(required.contains(&serde_json::json!("parent_id")));
    }

    #[test]
    fn test_siyuan_query_response_deserialize() {
        let json = r#"{"code":0,"msg":"success","data":[{"id":"123","content":"test"}]}"#;
        let response: SiyuanQueryResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.code, 0);
        assert_eq!(response.msg, "success");
        assert!(response.data.is_some());
    }

    #[test]
    fn test_siyuan_insert_response_deserialize() {
        let json = r#"{"code":0,"msg":"success","data":{"id":"123"}}"#;
        let response: SiyuanInsertResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.code, 0);
        assert_eq!(response.msg, "success");
        assert!(response.data.is_some());
    }
}
