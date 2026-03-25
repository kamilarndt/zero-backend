use async_trait::async_trait;
use parking_lot::Mutex;
use rusqlite::Connection;
use serde_json::json;
use std::sync::Arc;

use super::traits::{Tool, ToolResult};
use crate::memory::tasks::{AgentTask, TaskStatus};

/// Task planning tool for creating and managing agent tasks
pub struct TaskPlanTool {
    db: Arc<Mutex<Connection>>,
}

impl TaskPlanTool {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    fn handle_create(&self, args: serde_json::Value) -> ToolResult {
        let title = match args.get("title").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return ToolResult {
                success: false,
                output: String::new(),
                error: Some("Missing required field: title".to_string()),
            },
        };

        let status_str = args.get("status").and_then(|v| v.as_str()).unwrap_or("Todo");
        let status = match status_str {
            "Todo" => TaskStatus::Todo,
            "InProgress" => TaskStatus::InProgress,
            "Review" => TaskStatus::Review,
            "Done" => TaskStatus::Done,
            _ => TaskStatus::Todo,
        };

        let parent_id = args.get("parent_id").and_then(|v| v.as_str()).map(String::from);
        let assigned_hand = args.get("assigned_hand").and_then(|v| v.as_str()).map(String::from);

        let task = AgentTask::new(title, status, parent_id, assigned_hand);

        let conn = self.db.lock();
        match crate::memory::tasks::create_task(&conn, &task) {
            Ok(()) => ToolResult {
                success: true,
                output: format!("Created task {} with status {:?}", task.id, task.status),
                error: None,
            },
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to create task: {}", e)),
            },
        }
    }

    fn handle_update(&self, args: serde_json::Value) -> ToolResult {
        let task_id = match args.get("task_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult {
                success: false,
                output: String::new(),
                error: Some("Missing required field: task_id".to_string()),
            },
        };

        let status_str = args.get("status").and_then(|v| v.as_str()).unwrap_or("Todo");
        let status = match status_str {
            "Todo" => TaskStatus::Todo,
            "InProgress" => TaskStatus::InProgress,
            "Review" => TaskStatus::Review,
            "Done" => TaskStatus::Done,
            _ => TaskStatus::Todo,
        };

        let assigned_hand = args.get("assigned_hand").and_then(|v| v.as_str());

        let conn = self.db.lock();
        match crate::memory::tasks::update_task_status(&conn, task_id, status, assigned_hand) {
            Ok(()) => ToolResult {
                success: true,
                output: format!("Updated task {} to status {:?}", task_id, status),
                error: None,
            },
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to update task: {}", e)),
            },
        }
    }

    fn handle_list(&self, args: serde_json::Value) -> ToolResult {
        let status_filter = args.get("status").and_then(|v| v.as_str()).map(|s| match s {
            "Todo" => TaskStatus::Todo,
            "InProgress" => TaskStatus::InProgress,
            "Review" => TaskStatus::Review,
            "Done" => TaskStatus::Done,
            _ => TaskStatus::Todo,
        });

        let conn = self.db.lock();
        match crate::memory::tasks::list_tasks(&conn, status_filter) {
            Ok(tasks) => {
                let output = if tasks.is_empty() {
                    "No tasks found".to_string()
                } else {
                    tasks
                        .iter()
                        .map(|t| {
                            format!(
                                "- [{}] {}: {} (hand: {:?})",
                                t.status.as_str(),
                                t.id,
                                t.title,
                                t.assigned_hand
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                ToolResult {
                    success: true,
                    output,
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to list tasks: {}", e)),
            },
        }
    }

    fn handle_get(&self, args: serde_json::Value) -> ToolResult {
        let task_id = match args.get("task_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult {
                success: false,
                output: String::new(),
                error: Some("Missing required field: task_id".to_string()),
            },
        };

        let conn = self.db.lock();
        match crate::memory::tasks::get_task(&conn, task_id) {
            Ok(Some(task)) => ToolResult {
                success: true,
                output: format!(
                    "Task {}: {}\nStatus: {:?}\nParent: {:?}\nHand: {:?}\nCreated: {}\nUpdated: {}",
                    task.id, task.title, task.status, task.parent_id, task.assigned_hand, task.created_at, task.updated_at
                ),
                error: None,
            },
            Ok(None) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Task {} not found", task_id)),
            },
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to get task: {}", e)),
            },
        }
    }

    fn handle_delete(&self, args: serde_json::Value) -> ToolResult {
        let task_id = match args.get("task_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult {
                success: false,
                output: String::new(),
                error: Some("Missing required field: task_id".to_string()),
            },
        };

        let conn = self.db.lock();
        match crate::memory::tasks::delete_task(&conn, task_id) {
            Ok(()) => ToolResult {
                success: true,
                output: format!("Deleted task {}", task_id),
                error: None,
            },
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to delete task: {}", e)),
            },
        }
    }
}

#[async_trait]
impl Tool for TaskPlanTool {
    fn name(&self) -> &str {
        "task_plan"
    }

    fn description(&self) -> &str {
        "Create and manage agent tasks with status tracking and hand assignment. Supports subcommands: create, update, list, get, delete."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "update", "list", "get", "delete"],
                    "description": "The action to perform"
                },
                "title": {
                    "type": "string",
                    "description": "Task title (required for create)"
                },
                "status": {
                    "type": "string",
                    "enum": ["Todo", "InProgress", "Review", "Done"],
                    "description": "Task status"
                },
                "task_id": {
                    "type": "string",
                    "description": "Task ID (required for update, get, delete)"
                },
                "parent_id": {
                    "type": "string",
                    "description": "Parent task ID for hierarchical tasks"
                },
                "assigned_hand": {
                    "type": "string",
                    "description": "Hand/agent assigned to this task"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let action = match args.get("action").and_then(|v| v.as_str()) {
            Some(a) => a,
            None => return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Missing required field: action".to_string()),
            }),
        };

        let result = match action {
            "create" => self.handle_create(args),
            "update" => self.handle_update(args),
            "list" => self.handle_list(args),
            "get" => self.handle_get(args),
            "delete" => self.handle_delete(args),
            _ => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown action: {}", action)),
            },
        };

        Ok(result)
    }
}
