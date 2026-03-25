//! Auto-hydration of active tasks into system prompt.
//!
//! This module provides automatic injection of active tasks from the
//! SQLite task database into the system prompt, ensuring the agent
//! always has context about what it's working on.

use anyhow::Result;
use std::fmt::Write;

use super::prompt::{PromptContext, PromptSection};

const TASKS_DB_PATH: &str = ".zeroclaw/agent_tasks.db";

/// System prompt section that auto-hydrates active tasks
pub struct TasksSection;

impl PromptSection for TasksSection {
    fn name(&self) -> &str {
        "tasks"
    }

    fn build(&self, ctx: &PromptContext<'_>) -> Result<String> {
        // Try to load active tasks from SQLite database
        let db_path = ctx.workspace_dir.join(TASKS_DB_PATH);

        if !db_path.exists() {
            return Ok(String::new());
        }

        // Use the tasks module to list active tasks
        let conn = rusqlite::Connection::open(&db_path);
        let Ok(conn) = conn else {
            return Ok(String::new());
        };

        // Initialize schema if needed and list in-progress tasks
        let _ = crate::memory::tasks::init_tasks_table(&conn);

        let Ok(tasks) = crate::memory::tasks::list_tasks(
            &conn,
            Some(crate::memory::tasks::TaskStatus::InProgress),
        ) else {
            return Ok(String::new());
        };

        if tasks.is_empty() {
            return Ok(String::new());
        }

        let mut output = String::from("## [SYSTEM STATE: ACTIVE TASKS]\n\n");
        output.push_str("You are currently working on the following tasks:\n\n");

        for task in tasks {
            let _ = writeln!(
                output,
                "- **ID**: `{}` | **Status**: {} | **Title**: {}",
                task.id,
                task.status.as_str(),
                task.title
            );

            if let Some(parent_id) = &task.parent_id {
                let _ = writeln!(output, "  └─ Parent: `{parent_id}`");
            }

            if let Some(hand) = &task.assigned_hand {
                let _ = writeln!(output, "  └─ Assigned to: {hand}");
            }
        }

        output.push_str("\nUse this context to prioritize your work and provide status updates.\n");

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn tasks_section_returns_empty_when_db_missing() {
        let section = TasksSection;

        let tools: Vec<Box<dyn crate::tools::Tool>> = vec![];
        let ctx = PromptContext {
            workspace_dir: Path::new("/tmp"),
            model_name: "test-model",
            tools: &tools,
            skills: &[],
            skills_prompt_mode: crate::config::SkillsPromptInjectionMode::Full,
            identity_config: None,
            dispatcher_instructions: "",
        };

        let output = section.build(&ctx).unwrap();
        assert!(output.is_empty());
    }
}
