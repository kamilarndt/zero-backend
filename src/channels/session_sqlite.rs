//! Chat session management backed by SQLite.
//!
//! Stores sessions in a dedicated `chat_sessions` table inside the same
//! `brain.db` used by [`SqliteMemory`].  Messages live in the existing
//! `memories` table with `category = 'conversation'` and matching `session_id`.

use anyhow::{Context, Result};
use chrono::Local;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

// ── SessionStore ──────────────────────────────────────────────────────────────

/// Lightweight handle that shares the same SQLite DB as `SqliteMemory`.
pub struct SessionStore {
    conn: Arc<Mutex<Connection>>,
}

impl SessionStore {
    /// Open (or create) the sessions table inside `brain.db`.
    pub fn new(workspace_dir: &Path) -> Result<Self> {
        let db_path = workspace_dir.join("memory").join("brain.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&db_path)
            .context("SessionStore: failed to open brain.db")?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;

             CREATE TABLE IF NOT EXISTS chat_sessions (
                 id          TEXT PRIMARY KEY,
                 title       TEXT NOT NULL DEFAULT 'New Chat',
                 created_at  TEXT NOT NULL,
                 updated_at  TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_sessions_updated
                 ON chat_sessions(updated_at DESC);",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Build from an existing connection (e.g. shared with `SqliteMemory`).
    pub fn from_connection(conn: Arc<Mutex<Connection>>) -> Result<Self> {
        {
            let c = conn.lock();
            c.execute_batch(
                "CREATE TABLE IF NOT EXISTS chat_sessions (
                     id          TEXT PRIMARY KEY,
                     title       TEXT NOT NULL DEFAULT 'New Chat',
                     created_at  TEXT NOT NULL,
                     updated_at  TEXT NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_sessions_updated
                     ON chat_sessions(updated_at DESC);",
            )?;
        }
        Ok(Self { conn })
    }

    // ── CRUD ──────────────────────────────────────────────────────────────

    /// List sessions, newest first, with message count.
    pub fn list(&self, limit: i64, offset: i64) -> Result<Vec<ChatSession>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.title, s.created_at, s.updated_at,
                    COALESCE(
                      (SELECT COUNT(*) FROM memories m
                       WHERE m.session_id = s.id
                         AND m.category = 'conversation'), 0) AS message_count
             FROM chat_sessions s
             ORDER BY s.updated_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(params![limit, offset], |row| {
            Ok(ChatSession {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                message_count: row.get(4)?,
            })
        })?;
        Ok(rows.filter_map(std::result::Result::ok).collect())
    }

    /// Create a new session and return it.
    pub fn create(&self, title: Option<&str>) -> Result<ChatSession> {
        let id = Uuid::new_v4().to_string();
        let now = Local::now().to_rfc3339();
        let title = title.unwrap_or("New Chat");

        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO chat_sessions (id, title, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, title, now, now],
        )?;

        Ok(ChatSession {
            id,
            title: title.to_string(),
            created_at: now.clone(),
            updated_at: now,
            message_count: 0,
        })
    }

    /// Get a single session by id.
    pub fn get(&self, id: &str) -> Result<Option<ChatSession>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.title, s.created_at, s.updated_at,
                    COALESCE(
                      (SELECT COUNT(*) FROM memories m
                       WHERE m.session_id = s.id
                         AND m.category = 'conversation'), 0)
             FROM chat_sessions s WHERE s.id = ?1",
        )?;
        let row = stmt.query_row(params![id], |row| {
            Ok(ChatSession {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                message_count: row.get(4)?,
            })
        });
        match row {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Touch `updated_at` after a new message.
    pub fn touch(&self, id: &str) -> Result<()> {
        let now = Local::now().to_rfc3339();
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE chat_sessions SET updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    /// Delete a session and all its messages.
    pub fn delete(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock();
        let tx = conn.unchecked_transaction()?;
        tx.execute("DELETE FROM memories WHERE session_id = ?1", params![id])?;
        let affected = tx.execute("DELETE FROM chat_sessions WHERE id = ?1", params![id])?;
        tx.commit()?;
        Ok(affected > 0)
    }

    /// List messages for a session (paginated, newest last).
    pub fn messages(
        &self,
        session_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ChatMessage>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, key, content, created_at
             FROM memories
             WHERE session_id = ?1 AND category = 'conversation'
             ORDER BY created_at ASC
             LIMIT ?2 OFFSET ?3",
        )?;
        let sid = session_id.to_string();
        let rows = stmt.query_map(params![sid, limit, offset], |row| {
            let key: String = row.get(1)?;
            let role = if key.contains(":user") {
                "user"
            } else if key.contains(":assistant") {
                "assistant"
            } else {
                "system"
            };
            Ok(ChatMessage {
                id: row.get(0)?,
                session_id: session_id.to_string(),
                role: role.to_string(),
                content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        Ok(rows.filter_map(std::result::Result::ok).collect())
    }
}
