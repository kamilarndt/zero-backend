//! SQLite persistence for the knowledge base.
//!
//! Uses the same `rusqlite` + `parking_lot::Mutex<Connection>` pattern as
//! `SessionStore` and `SqliteMemory`.  Dedicated DB file: `knowledge.db`.

use anyhow::{Context, Result};
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

use super::schema::*;

pub struct KnowledgeStore {
    conn: Arc<Mutex<Connection>>,
}

impl KnowledgeStore {
    /// Open (or create) the knowledge DB inside `workspace_dir/memory/knowledge.db`.
    pub fn new(workspace_dir: &Path) -> Result<Self> {
        let db_path = workspace_dir.join("memory").join("knowledge.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&db_path)
            .context("KnowledgeStore: failed to open knowledge.db")?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;

             CREATE TABLE IF NOT EXISTS documents (
                 id          TEXT PRIMARY KEY,
                 title       TEXT NOT NULL DEFAULT 'Untitled',
                 created_at  TEXT NOT NULL,
                 updated_at  TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_documents_updated
                 ON documents(updated_at DESC);

             CREATE TABLE IF NOT EXISTS blocks (
                 id           TEXT PRIMARY KEY,
                 document_id  TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
                 block_type   TEXT NOT NULL CHECK(block_type IN ('paragraph', 'heading')),
                 content      TEXT NOT NULL DEFAULT '{}',
                 position     INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS idx_blocks_document
                 ON blocks(document_id, position);",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    // ── Documents ────────────────────────────────────────────────────────

    pub fn list_documents(&self) -> Result<Vec<Document>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, updated_at FROM documents ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Document {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn create_document(&self, input: &CreateDocument) -> Result<Document> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO documents (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, input.title, now, now],
        )?;
        Ok(Document {
            id,
            title: input.title.clone(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn get_document(&self, id: &str) -> Result<Option<Document>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, updated_at FROM documents WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Document {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;
        match rows.next() {
            Some(doc) => Ok(Some(doc?)),
            None => Ok(None),
        }
    }

    pub fn update_document(&self, id: &str, input: &UpdateDocument) -> Result<Option<Document>> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock();

        if let Some(ref title) = input.title {
            conn.execute(
                "UPDATE documents SET title = ?1, updated_at = ?2 WHERE id = ?3",
                params![title, now, id],
            )?;
        } else {
            conn.execute(
                "UPDATE documents SET updated_at = ?1 WHERE id = ?2",
                params![now, id],
            )?;
        }

        if conn.changes() == 0 {
            return Ok(None);
        }

        drop(conn);
        self.get_document(id)
    }

    pub fn delete_document(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM documents WHERE id = ?1", params![id])?;
        Ok(conn.changes() > 0)
    }

    // ── Blocks ───────────────────────────────────────────────────────────

    pub fn get_blocks(&self, document_id: &str) -> Result<Vec<Block>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, document_id, block_type, content, position
             FROM blocks WHERE document_id = ?1 ORDER BY position ASC",
        )?;
        let rows = stmt.query_map(params![document_id], |row| {
            let bt_str: String = row.get(2)?;
            let content_str: String = row.get(3)?;
            let block_type: BlockType = bt_str.parse().unwrap_or(BlockType::Paragraph);
            let content: serde_json::Value =
                serde_json::from_str(&content_str).unwrap_or(serde_json::json!({}));
            Ok(Block {
                id: row.get(0)?,
                document_id: row.get(1)?,
                block_type,
                content,
                position: row.get(4)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn create_block(&self, document_id: &str, input: &CreateBlock) -> Result<Block> {
        let id = Uuid::new_v4().to_string();
        let content_str = serde_json::to_string(&input.content)?;
        let position = match input.position {
            Some(p) => p,
            None => self.next_position(document_id)?,
        };

        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO blocks (id, document_id, block_type, content, position) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, document_id, input.block_type.to_string(), content_str, position],
        )?;

        // Touch parent document
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE documents SET updated_at = ?1 WHERE id = ?2",
            params![now, document_id],
        )?;

        Ok(Block {
            id,
            document_id: document_id.to_string(),
            block_type: input.block_type.clone(),
            content: input.content.clone(),
            position,
        })
    }

    pub fn update_block(
        &self,
        document_id: &str,
        block_id: &str,
        input: &UpdateBlock,
    ) -> Result<Option<Block>> {
        let conn = self.conn.lock();

        if let Some(ref content) = input.content {
            let content_str = serde_json::to_string(content)?;
            conn.execute(
                "UPDATE blocks SET content = ?1 WHERE id = ?2 AND document_id = ?3",
                params![content_str, block_id, document_id],
            )?;
        }

        if let Some(pos) = input.position {
            conn.execute(
                "UPDATE blocks SET position = ?1 WHERE id = ?2 AND document_id = ?3",
                params![pos, block_id, document_id],
            )?;
        }

        if conn.changes() == 0 {
            return Ok(None);
        }

        // Touch parent document
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE documents SET updated_at = ?1 WHERE id = ?2",
            params![now, document_id],
        )?;

        drop(conn);
        self.get_block(document_id, block_id)
    }

    pub fn delete_block(&self, document_id: &str, block_id: &str) -> Result<bool> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM blocks WHERE id = ?1 AND document_id = ?2",
            params![block_id, document_id],
        )?;
        let deleted = conn.changes() > 0;

        if deleted {
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE documents SET updated_at = ?1 WHERE id = ?2",
                params![now, document_id],
            )?;
        }
        Ok(deleted)
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    fn get_block(&self, document_id: &str, block_id: &str) -> Result<Option<Block>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, document_id, block_type, content, position
             FROM blocks WHERE id = ?1 AND document_id = ?2",
        )?;
        let mut rows = stmt.query_map(params![block_id, document_id], |row| {
            let bt_str: String = row.get(2)?;
            let content_str: String = row.get(3)?;
            let block_type: BlockType = bt_str.parse().unwrap_or(BlockType::Paragraph);
            let content: serde_json::Value =
                serde_json::from_str(&content_str).unwrap_or(serde_json::json!({}));
            Ok(Block {
                id: row.get(0)?,
                document_id: row.get(1)?,
                block_type,
                content,
                position: row.get(4)?,
            })
        })?;
        match rows.next() {
            Some(b) => Ok(Some(b?)),
            None => Ok(None),
        }
    }

    fn next_position(&self, document_id: &str) -> Result<i64> {
        let conn = self.conn.lock();
        let pos: i64 = conn.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM blocks WHERE document_id = ?1",
            params![document_id],
            |row| row.get(0),
        )?;
        Ok(pos)
    }
}
