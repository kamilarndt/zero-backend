//! ZeroClaw Native Skills Engine v2.0
//!
//! SQLite (brain.db) + Qdrant (skills_index) + Vector Search

use crate::memory::embeddings::EmbeddingProvider;
use crate::memory::traits::{Memory, MemoryCategory};
use anyhow::{Context, Result};
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Configuration for the Skills Engine
#[derive(Debug, Clone)]
pub struct SkillsConfig {
    pub workspace_dir: PathBuf,
    pub qdrant_url: String,
    pub qdrant_collection: String,
    pub search_threshold: f64,
    pub search_limit: usize,
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            workspace_dir: PathBuf::from(".zeroclaw/workspace"),
            qdrant_url: "http://localhost:6333".to_string(),
            qdrant_collection: "skills_index".to_string(),
            search_threshold: 0.82,
            search_limit: 5,
        }
    }
}

/// A skill stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: Option<i64>,
    pub name: String,
    pub description: String,
    pub content: String,
    pub version: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip)]
    pub tools: Vec<super::SkillTool>,
    pub is_active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl Skill {
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

/// Result from vector search with score
#[derive(Debug, Clone)]
pub struct SkillSearchResult {
    pub skill: Skill,
    pub score: f64,
}

/// Payload stored in Qdrant for each skill
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SkillQdrantPayload {
    skill_id: i64,
    name: String,
    description: String,
    tags: String,
}

/// Core Skills Engine - manages SQLite storage + Qdrant vector index
pub struct SkillsEngine {
    db: Arc<Mutex<Connection>>,
    qdrant: Arc<dyn Memory>,
    embedder: Arc<dyn EmbeddingProvider>,
    config: SkillsConfig,
    initialized: Arc<std::sync::atomic::AtomicBool>,
}

impl SkillsEngine {
    /// Create a new SkillsEngine with lazy initialization
    pub fn new(
        workspace_dir: &Path,
        qdrant: Arc<dyn Memory>,
        embedder: Arc<dyn EmbeddingProvider>,
    ) -> Result<Self> {
        let db_path = workspace_dir.join("brain.db");
        let conn =
            Connection::open(&db_path).context("Failed to open brain.db for SkillsEngine")?;

        let config = SkillsConfig {
            workspace_dir: workspace_dir.to_path_buf(),
            ..Default::default()
        };

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            qdrant,
            embedder,
            config,
            initialized: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Ensure database schema and Qdrant collection are initialized
    pub async fn ensure_initialized(&self) -> Result<()> {
        if self.initialized.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }

        // Initialize SQLite schema
        self.init_db_schema()?;

        // Initialize Qdrant collection
        self.init_qdrant_collection().await?;

        self.initialized
            .store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// Initialize SQLite schema for agent_skills table
    fn init_db_schema(&self) -> Result<()> {
        let conn = self.db.lock();
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS agent_skills (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT NOT NULL,
                content TEXT NOT NULL,
                version TEXT DEFAULT '1.0.0',
                author TEXT,
                tags TEXT,
                tools TEXT,
                is_active BOOLEAN DEFAULT 1,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            [],
        )
        .context("Failed to create agent_skills table")?;

        // Create indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_skills_name ON agent_skills(name)",
            [],
        )
        .context("Failed to create idx_skills_name")?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_skills_active ON agent_skills(is_active)",
            [],
        )
        .context("Failed to create idx_skills_active")?;

        Ok(())
    }

    /// Initialize Qdrant collection for skill vectors
    async fn init_qdrant_collection(&self) -> Result<()> {
        // Note: QdrantMemory handles collection creation lazily
        // We just need to ensure our collection name is used
        tracing::info!(
            "SkillsEngine using Qdrant collection: {}",
            self.config.qdrant_collection
        );
        Ok(())
    }

    /// Store a new skill or update existing one
    pub async fn store_skill(&self, skill: &Skill) -> Result<i64> {
        self.ensure_initialized().await?;

        let tags_json = serde_json::to_string(&skill.tags)?;
        let tools_json = serde_json::to_string(&skill.tools)?;
        let now = Utc::now().to_rfc3339();

        let conn = self.db.lock();
        let skill_id: i64 = if let Some(id) = skill.id {
            // Update existing skill
            conn.execute(
                r#"
                UPDATE agent_skills
                SET name = ?1, description = ?2, content = ?3,
                    version = ?4, author = ?5, tags = ?6, tools = ?7,
                    is_active = ?8, updated_at = ?9
                WHERE id = ?10
                "#,
                params![
                    &skill.name,
                    &skill.description,
                    &skill.content,
                    &skill.version,
                    &skill.author,
                    &tags_json,
                    &tools_json,
                    skill.is_active,
                    &now,
                    id,
                ],
            )
            .context("Failed to update skill")?;
            id
        } else {
            // Insert new skill
            conn.execute(
                r#"
                INSERT INTO agent_skills (name, description, content, version, author, tags, tools, is_active)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    &skill.name,
                    &skill.description,
                    &skill.content,
                    &skill.version,
                    &skill.author,
                    &tags_json,
                    &tools_json,
                    skill.is_active,
                ],
            ).context("Failed to insert skill")?;
            conn.last_insert_rowid()
        };

        // Vectorize and upsert to Qdrant
        self.upsert_skill_to_qdrant(skill_id, skill).await?;

        Ok(skill_id)
    }

    /// Upsert skill vector to Qdrant
    async fn upsert_skill_to_qdrant(&self, skill_id: i64, skill: &Skill) -> Result<()> {
        // Create a unique key for Qdrant
        let key = format!("skill:{}", skill.name);

        // Store in Qdrant with metadata
        self.qdrant
            .store(
                &key,
                &skill.description,
                MemoryCategory::Custom("skill".to_string()),
                None,
            )
            .await
            .context("Failed to upsert skill to Qdrant")?;

        tracing::debug!(
            "Upserted skill '{}' to Qdrant (id: {})",
            skill.name,
            skill_id
        );
        Ok(())
    }

    /// Get a skill by ID
    pub async fn get_skill(&self, id: i64) -> Result<Option<Skill>> {
        self.ensure_initialized().await?;

        let conn = self.db.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, content, version, author, tags, tools, is_active, created_at, updated_at
             FROM agent_skills WHERE id = ?"
        )?;

        let skill = stmt
            .query_row(params![id], |row| {
                Ok(Skill {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    content: row.get(3)?,
                    version: row.get(4)?,
                    author: row.get(5)?,
                    tags: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                    tools: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                    is_active: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .optional()?;

        Ok(skill)
    }

    /// Get a skill by name
    pub async fn get_skill_by_name(&self, name: &str) -> Result<Option<Skill>> {
        self.ensure_initialized().await?;

        let conn = self.db.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, content, version, author, tags, tools, is_active, created_at, updated_at
             FROM agent_skills WHERE name = ?"
        )?;

        let skill = stmt
            .query_row(params![name], |row| {
                Ok(Skill {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    content: row.get(3)?,
                    version: row.get(4)?,
                    author: row.get(5)?,
                    tags: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                    tools: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                    is_active: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .optional()?;

        Ok(skill)
    }

    /// List all skills
    pub async fn list_skills(&self, active_only: bool) -> Result<Vec<Skill>> {
        self.ensure_initialized().await?;

        let conn = self.db.lock();
        let query = if active_only {
            "SELECT id, name, description, content, version, author, tags, tools, is_active, created_at, updated_at
             FROM agent_skills WHERE is_active = 1 ORDER BY created_at DESC"
        } else {
            "SELECT id, name, description, content, version, author, tags, tools, is_active, created_at, updated_at
             FROM agent_skills ORDER BY created_at DESC"
        };

        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map([], |row| {
            Ok(Skill {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                description: row.get(2)?,
                content: row.get(3)?,
                version: row.get(4)?,
                author: row.get(5)?,
                tags: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                tools: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                is_active: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?;

        let mut skills = Vec::new();
        for row in rows {
            skills.push(row?);
        }
        Ok(skills)
    }

    /// Search skills by vector similarity
    pub async fn search_skills(
        &self,
        query: &str,
        threshold: f64,
    ) -> Result<Vec<SkillSearchResult>> {
        self.ensure_initialized().await?;

        // Search Qdrant for matching skills
        let entries = self
            .qdrant
            .recall(query, self.config.search_limit, None)
            .await?;

        let mut results = Vec::new();
        for entry in entries {
            // Extract skill name from key (format: "skill:name")
            let skill_name = entry.key.strip_prefix("skill:").unwrap_or(&entry.key);

            if let Some(skill) = self.get_skill_by_name(skill_name).await? {
                if let Some(score) = entry.score {
                    if score >= threshold {
                        results.push(SkillSearchResult { skill, score });
                    }
                }
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }

    /// Delete a skill
    pub async fn delete_skill(&self, id: i64) -> Result<bool> {
        self.ensure_initialized().await?;

        // Get skill name before deletion for Qdrant cleanup
        let skill = self.get_skill(id).await?;
        let skill_name = skill.as_ref().map(|s| s.name.clone());

        let conn = self.db.lock();
        let rows_affected = conn.execute("DELETE FROM agent_skills WHERE id = ?", params![id])?;

        if rows_affected > 0 {
            // Remove from Qdrant
            if let Some(name) = skill_name {
                let key = format!("skill:{}", name);
                let _ = self.qdrant.forget(&key).await;
            }
        }

        Ok(rows_affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_config_has_defaults() {
        let cfg = SkillsConfig::default();
        assert_eq!(cfg.qdrant_collection, "skills_index");
        assert_eq!(cfg.search_threshold, 0.82);
        assert_eq!(cfg.search_limit, 5);
    }

    #[test]
    fn skill_serializes_correctly() {
        let skill = Skill {
            id: None,
            name: "test".to_string(),
            description: "test desc".to_string(),
            content: "# Test\n\nContent".to_string(),
            version: "1.0.0".to_string(),
            author: Some("test".to_string()),
            tags: vec!["test".to_string(), "demo".to_string()],
            tools: vec![],
            is_active: true,
            created_at: None,
            updated_at: None,
        };

        let json = serde_json::to_string(&skill).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("demo"));
    }
}
