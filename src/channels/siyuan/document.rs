//! SiYuan document operations.
//!
//! This module provides functions for managing documents in SiYuan:
//! - Create documents with markdown content
//! - Rename, remove, move documents
//! - Get document paths (human-readable and storage paths)

use super::client::SiyuanClient;
use super::types::EmptyResponse;
use serde::Serialize;

/// Request to create a document with markdown content.
#[derive(Debug, Clone, Serialize)]
pub struct CreateDocRequest {
    pub notebook: String,
    pub path: String,
    pub markdown: String,
}

/// Request to rename a document by path.
#[derive(Debug, Clone, Serialize)]
pub struct RenameDocRequest {
    pub notebook: String,
    pub path: String,
    pub title: String,
}

/// Request to rename a document by ID.
#[derive(Debug, Clone, Serialize)]
pub struct RenameDocByIDRequest {
    pub id: String,
    pub title: String,
}

/// Request to remove a document by path.
#[derive(Debug, Clone, Serialize)]
pub struct RemoveDocRequest {
    pub notebook: String,
    pub path: String,
}

/// Request to remove a document by ID.
#[derive(Debug, Clone, Serialize)]
pub struct RemoveDocByIDRequest {
    pub id: String,
}

/// Request to move documents.
#[derive(Debug, Clone, Serialize)]
pub struct MoveDocsRequest {
    #[serde(rename = "fromPaths")]
    pub from_paths: Vec<String>,
    #[serde(rename = "toNotebook")]
    pub to_notebook: String,
    #[serde(rename = "toPath")]
    pub to_path: String,
}

/// Request to move documents by ID.
#[derive(Debug, Clone, Serialize)]
pub struct MoveDocsByIDRequest {
    #[serde(rename = "fromIDs")]
    pub from_ids: Vec<String>,
    #[serde(rename = "toID")]
    pub to_id: String,
}

/// Request to get human-readable path from storage path.
#[derive(Debug, Clone, Serialize)]
pub struct GetHPathByPathRequest {
    pub notebook: String,
    pub path: String,
}

/// Request to get human-readable path from block ID.
#[derive(Debug, Clone, Serialize)]
pub struct GetHPathByIDRequest {
    pub id: String,
}

/// Request to get storage path from block ID.
#[derive(Debug, Clone, Serialize)]
pub struct GetPathByIDRequest {
    pub id: String,
}

/// Response containing storage path information.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PathByIDResponse {
    pub notebook: String,
    pub path: String,
}

/// Request to get block IDs from human-readable path.
#[derive(Debug, Clone, Serialize)]
pub struct GetIDsByHPathRequest {
    pub path: String,
    pub notebook: String,
}

impl SiyuanClient {
    /// Create a new document with markdown content.
    ///
    /// # Arguments
    /// * `notebook_id` - The ID of the notebook to create the document in.
    /// * `path` - The document path (e.g., "/foo/bar").
    /// * `markdown` - The GFM markdown content for the document.
    ///
    /// # Returns
    /// The ID of the created document.
    ///
    /// # Note
    /// If the same path is used repeatedly, the existing document is NOT overwritten.
    pub async fn create_doc(
        &self,
        notebook_id: &str,
        path: &str,
        markdown: &str,
    ) -> anyhow::Result<String> {
        self.post_data(
            "/api/filetree/createDocWithMd",
            &CreateDocRequest {
                notebook: notebook_id.to_string(),
                path: path.to_string(),
                markdown: markdown.to_string(),
            },
        )
        .await
    }

    /// Rename a document by its path.
    ///
    /// # Arguments
    /// * `notebook_id` - The ID of the notebook containing the document.
    /// * `path` - The current storage path of the document.
    /// * `new_title` - The new title for the document.
    pub async fn rename_doc(
        &self,
        notebook_id: &str,
        path: &str,
        new_title: &str,
    ) -> anyhow::Result<()> {
        self.post_data::<_, EmptyResponse>(
            "/api/filetree/renameDoc",
            &RenameDocRequest {
                notebook: notebook_id.to_string(),
                path: path.to_string(),
                title: new_title.to_string(),
            },
        )
        .await
    }

    /// Rename a document by its ID.
    ///
    /// # Arguments
    /// * `doc_id` - The ID of the document to rename.
    /// * `new_title` - The new title for the document.
    pub async fn rename_doc_by_id(&self, doc_id: &str, new_title: &str) -> anyhow::Result<()> {
        self.post_data::<_, EmptyResponse>(
            "/api/filetree/renameDocByID",
            &RenameDocByIDRequest {
                id: doc_id.to_string(),
                title: new_title.to_string(),
            },
        )
        .await
    }

    /// Remove a document by its path.
    ///
    /// # Arguments
    /// * `notebook_id` - The ID of the notebook containing the document.
    /// * `path` - The storage path of the document to remove.
    pub async fn remove_doc(&self, notebook_id: &str, path: &str) -> anyhow::Result<()> {
        self.post_data::<_, EmptyResponse>(
            "/api/filetree/removeDoc",
            &RemoveDocRequest {
                notebook: notebook_id.to_string(),
                path: path.to_string(),
            },
        )
        .await
    }

    /// Remove a document by its ID.
    ///
    /// # Arguments
    /// * `doc_id` - The ID of the document to remove.
    pub async fn remove_doc_by_id(&self, doc_id: &str) -> anyhow::Result<()> {
        self.post_data::<_, EmptyResponse>(
            "/api/filetree/removeDocByID",
            &RemoveDocByIDRequest { id: doc_id.to_string() },
        )
        .await
    }

    /// Move documents to a new location.
    ///
    /// # Arguments
    /// * `from_paths` - List of source storage paths to move.
    /// * `to_notebook_id` - The ID of the target notebook.
    /// * `to_path` - The target path in the destination notebook.
    pub async fn move_docs(
        &self,
        from_paths: &[&str],
        to_notebook_id: &str,
        to_path: &str,
    ) -> anyhow::Result<()> {
        self.post_data::<_, EmptyResponse>(
            "/api/filetree/moveDocs",
            &MoveDocsRequest {
                from_paths: from_paths.iter().map(|s| s.to_string()).collect(),
                to_notebook: to_notebook_id.to_string(),
                to_path: to_path.to_string(),
            },
        )
        .await
    }

    /// Move documents by ID to a new location.
    ///
    /// # Arguments
    /// * `from_ids` - List of source document IDs to move.
    /// * `to_id` - The target parent doc ID or notebook ID.
    pub async fn move_docs_by_id(&self, from_ids: &[&str], to_id: &str) -> anyhow::Result<()> {
        self.post_data::<_, EmptyResponse>(
            "/api/filetree/moveDocsByID",
            &MoveDocsByIDRequest {
                from_ids: from_ids.iter().map(|s| s.to_string()).collect(),
                to_id: to_id.to_string(),
            },
        )
        .await
    }

    /// Get human-readable path from storage path.
    ///
    /// # Arguments
    /// * `notebook_id` - The ID of the notebook.
    /// * `path` - The storage path.
    ///
    /// # Returns
    /// The human-readable path (e.g., "/foo/bar").
    pub async fn get_hpath_by_path(
        &self,
        notebook_id: &str,
        path: &str,
    ) -> anyhow::Result<String> {
        self.post_data(
            "/api/filetree/getHPathByPath",
            &GetHPathByPathRequest {
                notebook: notebook_id.to_string(),
                path: path.to_string(),
            },
        )
        .await
    }

    /// Get human-readable path from block ID.
    ///
    /// # Arguments
    /// * `block_id` - The block ID.
    ///
    /// # Returns
    /// The human-readable path.
    pub async fn get_hpath_by_id(&self, block_id: &str) -> anyhow::Result<String> {
        self.post_data(
            "/api/filetree/getHPathByID",
            &GetHPathByIDRequest {
                id: block_id.to_string(),
            },
        )
        .await
    }

    /// Get storage path from block ID.
    ///
    /// # Arguments
    /// * `block_id` - The block ID.
    ///
    /// # Returns
    /// The storage path information including notebook and path.
    pub async fn get_path_by_id(&self, block_id: &str) -> anyhow::Result<PathByIDResponse> {
        self.post_data(
            "/api/filetree/getPathByID",
            &GetPathByIDRequest {
                id: block_id.to_string(),
            },
        )
        .await
    }

    /// Get block IDs from human-readable path.
    ///
    /// # Arguments
    /// * `path` - The human-readable path.
    /// * `notebook_id` - The ID of the notebook.
    ///
    /// # Returns
    /// List of block IDs at the given path.
    pub async fn get_ids_by_hpath(
        &self,
        path: &str,
        notebook_id: &str,
    ) -> anyhow::Result<Vec<String>> {
        self.post_data(
            "/api/filetree/getIDsByHPath",
            &GetIDsByHPathRequest {
                path: path.to_string(),
                notebook: notebook_id.to_string(),
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_doc_request_serialization() {
        let req = CreateDocRequest {
            notebook: "test-nb".into(),
            path: "/test/doc".into(),
            markdown: "# Test".into(),
        };
        let result = serde_json::to_string(&req);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("test-nb"));
    }
}