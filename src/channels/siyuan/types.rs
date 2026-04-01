//! SiYuan API request and response types.
//!
//! This module defines all data structures for communicating with
//! the SiYuan REST API (http://localhost:6806).

use serde::{Deserialize, Serialize};

/// Base SiYuan API response wrapper.
/// All SiYuan API responses follow this structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiyuanResponse<T> {
    /// Zero indicates success, non-zero indicates an error.
    pub code: i32,
    /// Empty string on success, error message on failure.
    pub msg: String,
    /// Response data. Can be an object, array, or null depending on the endpoint.
    pub data: Option<T>,
}

impl<T> SiyuanResponse<T> {
    /// Returns true if the response indicates success (code == 0).
    pub fn is_success(&self) -> bool {
        self.code == 0
    }

    /// Converts the response into a Result, returning an error if code != 0.
    pub fn into_result(self) -> anyhow::Result<T> {
        if self.is_success() {
            self.data.ok_or_else(|| anyhow::anyhow!("Success response missing data"))
        } else {
            Err(anyhow::anyhow!("SiYuan API error: {} (code {})", self.msg, self.code))
        }
    }
}

/// Empty response type for endpoints that return null on success.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyResponse;

/// Notebook information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notebook {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub sort: i32,
    pub closed: bool,
}

/// Block type identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockType {
    /// Document block
    Document,
    /// Heading block (h1-h6)
    Heading,
    /// Paragraph block
    Paragraph,
    /// Code block
    Code,
    /// List item
    ListItem,
    /// Table
    Table,
    /// Super block
    SuperBlock,
    /// Blockquote
    Blockquote,
    /// Callout
    Callout,
    /// HTML block
    Html,
    /// Math block
    Math,
}

/// Block information summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub block_type: String,
    pub subtype: Option<String>,
}

/// Block operation result from insert/update/delete operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockOperation {
    pub action: String,
    pub id: String,
    #[serde(rename = "parentID")]
    pub parent_id: Option<String>,
    #[serde(rename = "previousID")]
    pub previous_id: Option<String>,
    pub data: Option<String>,
}

/// Block operation response with undo/redo support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockOperations {
    #[serde(rename = "doOperations")]
    pub do_operations: Vec<BlockOperation>,
    #[serde(rename = "undoOperations")]
    pub undo_operations: Option<Vec<BlockOperation>>,
}

/// File information in SiYuan workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub is_dir: bool,
    pub is_symlink: bool,
    pub name: String,
    pub updated: u64,
}

/// Asset upload result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetUploadResult {
    pub err_files: Vec<String>,
    pub succ_map: std::collections::HashMap<String, String>,
}

/// Export result with file path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub path: String,
}

/// Template render result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateResult {
    pub content: String,
    pub path: String,
}