//! Native SQLite task system for ZeroClaw agents.
//!
//! This module provides a persistent task tracking system with hierarchical
//! parent-child relationships, status tracking, and hand assignment.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Task status in the workflow
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Todo,
    InProgress,
    Review,
    Done,
}

impl std::str::FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Todo" => Ok(TaskStatus::Todo),
            "InProgress" => Ok(TaskStatus::InProgress),
            "Review" => Ok(TaskStatus::Review),
            "Done" => Ok(TaskStatus::Done),
            _ => Err(format!("Unknown task status: {}", s)),
        }
    }
}

impl TaskStatus {
    /// Parse from string (for database reads) - fallback to Todo for unknown values
    pub fn from_str_fallback(s: &str) -> Self {
        s.parse().unwrap_or(TaskStatus::Todo)
    }

    /// Convert to string (for database writes)
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Todo => "Todo",
            TaskStatus::InProgress => "InProgress",
            TaskStatus::Review => "Review",
            TaskStatus::Done => "Done",
        }
    }
}

/// A task in the agent's workflow
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentTask {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub parent_id: Option<String>,
    pub assigned_hand: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl AgentTask {
    /// Create a new task with a generated ID and timestamps
    pub fn new(
        title: String,
        status: TaskStatus,
        parent_id: Option<String>,
        assigned_hand: Option<String>,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id: Uuid::new_v4().to_string(),
            title,
            status,
            parent_id,
            assigned_hand,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create from database row
    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get(0)?,
            title: row.get(1)?,
            status: TaskStatus::from_str_fallback(&row.get::<_, String>(2)?),
            parent_id: row.get(3)?,
            assigned_hand: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    }
}

/// Initialize the agent_tasks table schema
pub fn init_tasks_table(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS agent_tasks (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('Todo', 'InProgress', 'Review', 'Done')),
            parent_id TEXT,
            assigned_hand TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY (parent_id) REFERENCES agent_tasks(id)
        );
        CREATE INDEX IF NOT EXISTS idx_tasks_status ON agent_tasks(status);
        CREATE INDEX IF NOT EXISTS idx_tasks_parent ON agent_tasks(parent_id);
        CREATE INDEX IF NOT EXISTS idx_tasks_hand ON agent_tasks(assigned_hand);",
    )?;
    Ok(())
}

/// Create a new task in the database
pub fn create_task(conn: &Connection, task: &AgentTask) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO agent_tasks (id, title, status, parent_id, assigned_hand, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            &task.id,
            &task.title,
            task.status.as_str(),
            &task.parent_id,
            &task.assigned_hand,
            task.created_at,
            task.updated_at,
        ],
    )?;
    Ok(())
}

/// Update task status and optionally hand assignment
pub fn update_task_status(
    conn: &Connection,
    task_id: &str,
    status: TaskStatus,
    assigned_hand: Option<&str>,
) -> Result<(), rusqlite::Error> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if let Some(hand) = assigned_hand {
        conn.execute(
            "UPDATE agent_tasks SET status = ?1, assigned_hand = ?2, updated_at = ?3 WHERE id = ?4",
            params![status.as_str(), hand, now, task_id],
        )?;
    } else {
        conn.execute(
            "UPDATE agent_tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status.as_str(), now, task_id],
        )?;
    }
    Ok(())
}

/// List tasks with optional status filter
pub fn list_tasks(
    conn: &Connection,
    status: Option<TaskStatus>,
) -> Result<Vec<AgentTask>, rusqlite::Error> {
    let mut stmt = if let Some(_filter_status) = status {
        conn.prepare(
            "SELECT id, title, status, parent_id, assigned_hand, created_at, updated_at
             FROM agent_tasks WHERE status = ?1 ORDER BY created_at DESC",
        )?
    } else {
        conn.prepare(
            "SELECT id, title, status, parent_id, assigned_hand, created_at, updated_at
             FROM agent_tasks ORDER BY created_at DESC",
        )?
    };

    let tasks = if let Some(filter_status) = status {
        stmt.query_map([filter_status.as_str()], AgentTask::from_row)?
    } else {
        stmt.query_map([], AgentTask::from_row)?
    };

    tasks.collect()
}

/// Get a specific task by ID
pub fn get_task(conn: &Connection, task_id: &str) -> Result<Option<AgentTask>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, parent_id, assigned_hand, created_at, updated_at
         FROM agent_tasks WHERE id = ?1",
    )?;

    let mut rows = stmt.query_map([task_id], AgentTask::from_row)?;
    rows.next().transpose()
}

/// Delete a task by ID
pub fn delete_task(conn: &Connection, task_id: &str) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM agent_tasks WHERE id = ?1", [task_id])?;
    Ok(())
}

/// Get child tasks of a parent task
pub fn get_child_tasks(
    conn: &Connection,
    parent_id: &str,
) -> Result<Vec<AgentTask>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, parent_id, assigned_hand, created_at, updated_at
         FROM agent_tasks WHERE parent_id = ?1 ORDER BY created_at DESC",
    )?;

    let tasks = stmt.query_map([parent_id], AgentTask::from_row)?;
    tasks.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_tasks_table(&conn).unwrap();
        conn
    }

    #[test]
    fn test_create_and_get_task() {
        let conn = setup_test_db();
        let task = AgentTask::new("Test task".into(), TaskStatus::Todo, None, None);

        create_task(&conn, &task).unwrap();
        let retrieved = get_task(&conn, &task.id).unwrap().unwrap();

        assert_eq!(retrieved.id, task.id);
        assert_eq!(retrieved.title, "Test task");
        assert_eq!(retrieved.status, TaskStatus::Todo);
    }

    #[test]
    fn test_list_tasks_with_filter() {
        let conn = setup_test_db();

        let task1 = AgentTask::new("Task 1".into(), TaskStatus::Todo, None, None);
        let task2 = AgentTask::new("Task 2".into(), TaskStatus::Done, None, None);

        create_task(&conn, &task1).unwrap();
        create_task(&conn, &task2).unwrap();

        let todo_tasks = list_tasks(&conn, Some(TaskStatus::Todo)).unwrap();
        assert_eq!(todo_tasks.len(), 1);
        assert_eq!(todo_tasks[0].title, "Task 1");

        let all_tasks = list_tasks(&conn, None).unwrap();
        assert_eq!(all_tasks.len(), 2);
    }

    #[test]
    fn test_update_task_status() {
        let conn = setup_test_db();
        let task = AgentTask::new("Test task".into(), TaskStatus::Todo, None, None);

        create_task(&conn, &task).unwrap();
        update_task_status(&conn, &task.id, TaskStatus::InProgress, Some("Hand1")).unwrap();

        let updated = get_task(&conn, &task.id).unwrap().unwrap();
        assert_eq!(updated.status, TaskStatus::InProgress);
        assert_eq!(updated.assigned_hand, Some("Hand1".to_string()));
    }

    #[test]
    fn test_parent_child_relationships() {
        let conn = setup_test_db();
        let parent = AgentTask::new("Parent".into(), TaskStatus::Todo, None, None);
        create_task(&conn, &parent).unwrap();

        let child = AgentTask::new(
            "Child".into(),
            TaskStatus::Todo,
            Some(parent.id.clone()),
            None,
        );
        create_task(&conn, &child).unwrap();

        let children = get_child_tasks(&conn, &parent.id).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].title, "Child");
    }
}
