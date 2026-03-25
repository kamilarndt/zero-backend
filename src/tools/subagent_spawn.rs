use crate::agent::a2a::{AgentRole, A2APacket, A2AMessageBuilder, A2AHelper};
use crate::agent::hands::HandsDispatcher;
use crate::tools::traits::Tool;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Magic signal value to identify spawn messages in the system
pub const SPAWN_SIGNAL_MAGIC: &str = "__ZEROCLAW_SPAWN_SIGNAL__";

/// Request structure for spawning a new sub-agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnRequest {
    /// Unique request ID for tracking
    pub request_id: String,
    /// Role classification for the sub-agent
    pub role: AgentRole,
    /// Task description for the sub-agent
    pub task: String,
    /// Detailed instructions for task execution
    pub instructions: String,
    /// List of dependency task IDs
    pub dependencies: Vec<String>,
    /// Timeout in seconds before auto-termination
    pub timeout_seconds: u64,
}

impl SpawnRequest {
    /// Create a new spawn request with generated ID
    pub fn new(
        role: AgentRole,
        task: String,
        instructions: String,
        dependencies: Vec<String>,
        timeout_seconds: u64,
    ) -> Self {
        Self {
            request_id: A2AHelper::generate_task_id(),
            role,
            task,
            instructions,
            dependencies,
            timeout_seconds,
        }
    }

    /// Convert to A2APacket for transmission
    pub fn to_packet(self, source_id: String, target_id: String) -> A2APacket {
        let message = A2AMessageBuilder::task_assignment(
            self.request_id.clone(),
            format!("{}: {}", self.task, self.instructions),
            self.dependencies,
        );
        A2APacket::new(source_id, target_id, message)
    }
}

/// Tool for spawning background sub-agents (swarm workers)
///
/// This tool enables the main agent to delegate tasks to specialized
/// sub-agents that run concurrently with full isolation and coordination.
///
/// # Example Usage
/// ```json
/// {
///   "role": "executor",
///   "task": "Process and analyze the dataset",
///   "instructions": "Read data.csv, perform statistical analysis, save results",
///   "dependencies": [],
///   "timeout_seconds": 300
/// }
/// ```
pub struct SubagentSpawnTool {
    /// Channel sender for spawn requests
    pub spawn_tx: Option<Arc<mpsc::Sender<SpawnRequest>>>,
    /// Hands dispatcher for registering background agents
    pub hands_dispatcher: Option<Arc<HandsDispatcher>>,
}

impl SubagentSpawnTool {
    /// Create a new SubagentSpawnTool
    pub fn new() -> Self {
        Self {
            spawn_tx: None,
            hands_dispatcher: None,
        }
    }

    /// Set the spawn channel sender
    pub fn with_channel(mut self, sender: mpsc::Sender<SpawnRequest>) -> Self {
        self.spawn_tx = Some(Arc::new(sender));
        self
    }

    /// Set the hands dispatcher for registering background agents
    pub fn with_hands_dispatcher(mut self, dispatcher: Arc<HandsDispatcher>) -> Self {
        self.hands_dispatcher = Some(dispatcher);
        self
    }
}

impl Default for SubagentSpawnTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SubagentSpawnTool {
    fn name(&self) -> &str {
        "subagent_spawn"
    }

    fn description(&self) -> &str {
        r#"Spawns a background sub-agent for parallel task execution.

Supports three agent roles:
- planner: Decomposes complex tasks into subtasks with dependencies
- executor: Performs assigned tasks with full tool access
- reviewer: Validates results and provides quality assessment

The sub-agent runs in isolation with its own workspace and context.
Results are communicated back via the A2A (Agent-to-Agent) protocol."#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "role": {
                    "type": "string",
                    "enum": ["planner", "executor", "reviewer"],
                    "description": "Role classification for the sub-agent"
                },
                "task": {
                    "type": "string",
                    "description": "Brief task description (one line)"
                },
                "instructions": {
                    "type": "string",
                    "description": "Detailed instructions for task execution"
                },
                "dependencies": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "List of task IDs that must complete first (optional)"
                },
                "timeout_seconds": {
                    "type": "number",
                    "description": "Maximum execution time in seconds (default: 300)"
                }
            },
            "required": ["role", "task", "instructions"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<crate::tools::traits::ToolResult> {
        use crate::tools::traits::ToolResult;

        // Parse arguments
        let role_str = args.get("role")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required argument: role"))?;

        let role = AgentRole::from_str(role_str)
            .map_err(|e| anyhow::anyhow!("Invalid role: {}", e))?;

        let task = args.get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required argument: task"))?;

        let instructions = args.get("instructions")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required argument: instructions"))?;

        let dependencies: Vec<String> = args.get("dependencies")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let timeout_seconds = args.get("timeout_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(300);

        // Create spawn request
        let request = SpawnRequest::new(
            role.clone(),
            task.to_string(),
            instructions.to_string(),
            dependencies,
            timeout_seconds,
        );

        // Register the hand with the HandsDispatcher if available
        let hand_id = if let Some(dispatcher) = &self.hands_dispatcher {
            // Register the hand with the dispatcher
            let workspace_path = dispatcher.register_hand(request.request_id.clone()).await
                .map_err(|e| anyhow::anyhow!("Failed to register hand: {}", e))?;

            // Store the workspace path in the request for later use
            tracing::info!(
                "Registered hand {} at workspace: {:?}",
                request.request_id,
                workspace_path
            );

            Some(request.request_id.clone())
        } else {
            tracing::warn!("HandsDispatcher not configured - hand not registered");
            None
        };

        // Send to dispatcher channel if available
        if let Some(tx) = &self.spawn_tx {
            tx.send(request.clone()).await
                .map_err(|e| anyhow::anyhow!("Failed to queue spawn request: {}", e))?;
        }

        let output = if hand_id.is_some() || self.spawn_tx.is_some() {
            format!(
                "Sub-agent {} ({}) queued for parallel execution",
                request.request_id,
                role.as_str()
            )
        } else {
            format!(
                "Sub-agent {} ({}) created (dispatcher not configured)",
                request.request_id,
                role.as_str()
            )
        };

        Ok(ToolResult {
            success: true,
            output,
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_request_creation() {
        let request = SpawnRequest::new(
            AgentRole::Executor,
            "Test task".to_string(),
            "Test instructions".to_string(),
            vec![],
            300,
        );

        assert!(!request.request_id.is_empty());
        assert_eq!(request.role, AgentRole::Executor);
        assert_eq!(request.task, "Test task");
        assert_eq!(request.timeout_seconds, 300);
    }

    #[test]
    fn test_spawn_request_to_packet() {
        let request = SpawnRequest::new(
            AgentRole::Planner,
            "Plan task".to_string(),
            "Plan instructions".to_string(),
            vec!["dep1".to_string()],
            600,
        );

        let packet = request.to_packet("main".to_string(), "sub".to_string());

        assert_eq!(packet.source_id, "main");
        assert_eq!(packet.target_id, "sub");
        assert!(packet.validate().is_ok());
    }

    #[tokio::test]
    async fn test_subagent_spawn_tool() {
        let tool = SubagentSpawnTool::new();

        let args = json!({
            "role": "executor",
            "task": "Test task",
            "instructions": "Test instructions",
            "timeout_seconds": 300
        });

        let result = tool.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        assert!(tool_result.success);
        assert!(!tool_result.output.is_empty());
    }

    #[tokio::test]
    async fn test_subagent_spawn_with_dispatcher() {
        use crate::agent::hands::HandsDispatcher;

        let dispatcher = Arc::new(HandsDispatcher::new(5));
        let tool = SubagentSpawnTool::new().with_hands_dispatcher(dispatcher);

        let args = json!({
            "role": "executor",
            "task": "Test task with dispatcher",
            "instructions": "Test instructions",
            "timeout_seconds": 300
        });

        let result = tool.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        assert!(tool_result.success);
        assert!(tool_result.output.contains("queued for parallel execution"));
    }

    #[tokio::test]
    async fn test_subagent_spawn_tool_with_channel() {
        use tokio::sync::mpsc;

        let (tx, mut rx) = mpsc::channel(10);
        let tool = SubagentSpawnTool::new().with_channel(tx);

        let args = json!({
            "role": "planner",
            "task": "Test task with channel",
            "instructions": "Test instructions",
            "timeout_seconds": 300
        });

        let result = tool.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        assert!(tool_result.success);

        // Verify the request was sent through the channel
        let received = rx.try_recv();
        assert!(received.is_ok());
        let request = received.unwrap();
        assert_eq!(request.role, AgentRole::Planner);
        assert_eq!(request.task, "Test task with channel");
    }
}
