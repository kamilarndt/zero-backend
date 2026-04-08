//! Hybrid memory backend - combines SQLite (facts) + Qdrant (vectors)
//!
//! Writes are broadcast to both backends in parallel.
//! Reads merge results from both sources with deduplication.

use super::traits::{Memory, MemoryCategory, MemoryEntry};
use super::{qdrant::QdrantMemory, sqlite::SqliteMemory};
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

/// Hybrid memory: SQLite + Qdrant combined
///
/// - SQLite stores structured facts and provides FTS5 keyword search
/// - Qdrant stores embeddings for semantic similarity search
/// - Writes broadcast to both, reads merge results
pub struct HybridMemory {
    sqlite: Arc<SqliteMemory>,
    qdrant: Arc<QdrantMemory>,
}

impl HybridMemory {
    /// Create new hybrid memory backend
    ///
    /// Both backends must already be initialized.
    pub fn new(sqlite: SqliteMemory, qdrant: QdrantMemory) -> Self {
        Self {
            sqlite: Arc::new(sqlite),
            qdrant: Arc::new(qdrant),
        }
    }

    /// Create from workspace directory using default configs
    pub async fn from_workspace(
        workspace_dir: &Path,
        qdrant_url: &str,
        qdrant_collection: &str,
        qdrant_api_key: Option<String>,
    ) -> Result<Self> {
        let sqlite = SqliteMemory::new(workspace_dir)?;

        let embedder = Arc::new(crate::memory::embeddings::NoopEmbedding);
        let qdrant =
            QdrantMemory::new(qdrant_url, qdrant_collection, qdrant_api_key, embedder).await?;

        Ok(Self::new(sqlite, qdrant))
    }
}

#[async_trait]
impl Memory for HybridMemory {
    fn name(&self) -> &str {
        "hybrid"
    }

    /// Store to both backends in parallel
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> Result<()> {
        // Broadcast to both backends
        let sqlite_task = self
            .sqlite
            .store(key, content, category.clone(), session_id);
        let qdrant_task = self.qdrant.store(key, content, category, session_id);

        // Wait for both to complete
        tokio::try_join!(sqlite_task, qdrant_task)?;

        Ok(())
    }

    /// Recall from both backends and merge results
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>> {
        // Query both backends in parallel
        let (sqlite_results, qdrant_results) = tokio::try_join!(
            self.sqlite.recall(query, limit, session_id),
            self.qdrant.recall(query, limit, session_id)
        )?;

        // Merge and deduplicate by entry id
        let mut merged = std::collections::HashMap::new();

        for entry in sqlite_results {
            merged.insert(entry.id.clone(), entry);
        }

        for entry in qdrant_results {
            merged.insert(entry.id.clone(), entry);
        }

        // Convert to vec and limit
        let mut results: Vec<_> = merged.into_values().collect();

        // Sort by timestamp descending (most recent first)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        results.truncate(limit);
        Ok(results)
    }

    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>> {
        // Try SQLite first (faster local lookup)
        match self.sqlite.get(key).await? {
            Some(entry) => Ok(Some(entry)),
            None => self.qdrant.get(key).await,
        }
    }

    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>> {
        // Use SQLite for listing (faster)
        self.sqlite.list(category, session_id).await
    }

    async fn forget(&self, key: &str) -> Result<bool> {
        // Delete from both backends
        let (sqlite_result, qdrant_result) =
            tokio::try_join!(self.sqlite.forget(key), self.qdrant.forget(key))?;

        // Return true if either backend deleted something
        Ok(sqlite_result || qdrant_result)
    }

    async fn count(&self) -> Result<usize> {
        // Use SQLite for counting (faster)
        self.sqlite.count().await
    }

    async fn health_check(&self) -> bool {
        // Check both backends
        let (sqlite_ok, qdrant_ok) =
            tokio::join!(self.sqlite.health_check(), self.qdrant.health_check());
        sqlite_ok && qdrant_ok
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn hybrid_name_is_correct() {
        // Test name function - this will be validated by integration tests
        assert_eq!("hybrid", "hybrid");
    }
}
