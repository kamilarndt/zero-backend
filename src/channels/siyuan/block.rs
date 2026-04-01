//! SiYuan block operations.
//!
//! This module provides functions for manipulating blocks in SiYuan:
//! - Insert, prepend, append blocks
//! - Update, delete, move blocks
//! - Fold, unfold blocks
//! - Get block kramdown and child blocks

use super::client::SiyuanClient;
use super::types::{BlockInfo, BlockOperation, BlockOperations, EmptyResponse};
use serde::Serialize;

/// Data type for block content.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockDataType {
    Markdown,
    Dom,
}

impl Default for BlockDataType {
    fn default() -> Self {
        Self::Markdown
    }
}

/// Request to insert a block.
#[derive(Debug, Clone, Serialize)]
pub struct InsertBlockRequest {
    #[serde(rename = "dataType")]
    pub data_type: String,
    pub data: String,
    #[serde(rename = "nextID")]
    pub next_id: Option<String>,
    #[serde(rename = "previousID")]
    pub previous_id: Option<String>,
    #[serde(rename = "parentID")]
    pub parent_id: Option<String>,
}

/// Request to prepend a block to a parent.
#[derive(Debug, Clone, Serialize)]
pub struct PrependBlockRequest {
    pub data: String,
    #[serde(rename = "dataType")]
    pub data_type: String,
    #[serde(rename = "parentID")]
    pub parent_id: String,
}

/// Request to append a block to a parent.
#[derive(Debug, Clone, Serialize)]
pub struct AppendBlockRequest {
    pub data: String,
    #[serde(rename = "dataType")]
    pub data_type: String,
    #[serde(rename = "parentID")]
    pub parent_id: String,
}

/// Request to update a block.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateBlockRequest {
    pub id: String,
    #[serde(rename = "dataType")]
    pub data_type: String,
    pub data: String,
}

/// Request to delete a block.
#[derive(Debug, Clone, Serialize)]
pub struct DeleteBlockRequest {
    pub id: String,
}

/// Request to move a block.
#[derive(Debug, Clone, Serialize)]
pub struct MoveBlockRequest {
    pub id: String,
    #[serde(rename = "previousID")]
    pub previous_id: Option<String>,
    #[serde(rename = "parentID")]
    pub parent_id: Option<String>,
}

/// Request to fold a block.
#[derive(Debug, Clone, Serialize)]
pub struct FoldBlockRequest {
    pub id: String,
}

/// Request to unfold a block.
#[derive(Debug, Clone, Serialize)]
pub struct UnfoldBlockRequest {
    pub id: String,
}

/// Request to get a block's kramdown content.
#[derive(Debug, Clone, Serialize)]
pub struct GetBlockKramdownRequest {
    pub id: String,
}

/// Response containing block kramdown.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BlockKramdownResponse {
    pub id: String,
    pub kramdown: String,
}

/// Request to get child blocks.
#[derive(Debug, Clone, Serialize)]
pub struct GetChildBlocksRequest {
    pub id: String,
}

/// Request to transfer block references.
#[derive(Debug, Clone, Serialize)]
pub struct TransferBlockRefRequest {
    #[serde(rename = "fromID")]
    pub from_id: String,
    #[serde(rename = "toID")]
    pub to_id: String,
    #[serde(rename = "refIDs")]
    pub ref_ids: Option<Vec<String>>,
}

impl SiyuanClient {
    /// Insert a block at a specific position.
    ///
    /// # Arguments
    /// * `data` - The content to insert (markdown or DOM format).
    /// * `data_type` - Either "markdown" or "dom".
    /// * `next_id` - Insert before this block ID (highest priority).
    /// * `previous_id` - Insert after this block ID (medium priority).
    /// * `parent_id` - Append to this parent block ID (lowest priority).
    ///
    /// # Returns
    /// The block operations result containing the ID of the created block.
    ///
    /// # Note
    /// At least one of `next_id`, `previous_id`, or `parent_id` must be provided.
    /// Priority: next_id > previous_id > parent_id.
    pub async fn insert_block(
        &self,
        data: &str,
        data_type: &str,
        next_id: Option<&str>,
        previous_id: Option<&str>,
        parent_id: Option<&str>,
    ) -> anyhow::Result<BlockOperations> {
        self.post_data(
            "/api/block/insertBlock",
            &InsertBlockRequest {
                data_type: data_type.to_string(),
                data: data.to_string(),
                next_id: next_id.map(|s| s.to_string()),
                previous_id: previous_id.map(|s| s.to_string()),
                parent_id: parent_id.map(|s| s.to_string()),
            },
        )
        .await
    }

    /// Prepend a block as the first child of a parent.
    ///
    /// # Arguments
    /// * `data` - The content to prepend (markdown or DOM format).
    /// * `data_type` - Either "markdown" or "dom".
    /// * `parent_id` - The ID of the parent block.
    ///
    /// # Returns
    /// The block operations result containing the ID of the created block.
    pub async fn prepend_block(
        &self,
        data: &str,
        data_type: &str,
        parent_id: &str,
    ) -> anyhow::Result<BlockOperations> {
        self.post_data(
            "/api/block/prependBlock",
            &PrependBlockRequest {
                data: data.to_string(),
                data_type: data_type.to_string(),
                parent_id: parent_id.to_string(),
            },
        )
        .await
    }

    /// Append a block as the last child of a parent.
    ///
    /// # Arguments
    /// * `data` - The content to append (markdown or DOM format).
    /// * `data_type` - Either "markdown" or "dom".
    /// * `parent_id` - The ID of the parent block.
    ///
    /// # Returns
    /// The block operations result containing the ID of the created block.
    pub async fn append_block(
        &self,
        data: &str,
        data_type: &str,
        parent_id: &str,
    ) -> anyhow::Result<BlockOperations> {
        self.post_data(
            "/api/block/appendBlock",
            &AppendBlockRequest {
                data: data.to_string(),
                data_type: data_type.to_string(),
                parent_id: parent_id.to_string(),
            },
        )
        .await
    }

    /// Update a block's content.
    ///
    /// # Arguments
    /// * `block_id` - The ID of the block to update.
    /// * `data` - The new content (markdown or DOM format).
    /// * `data_type` - Either "markdown" or "dom".
    pub async fn update_block(
        &self,
        block_id: &str,
        data: &str,
        data_type: &str,
    ) -> anyhow::Result<BlockOperations> {
        self.post_data(
            "/api/block/updateBlock",
            &UpdateBlockRequest {
                id: block_id.to_string(),
                data_type: data_type.to_string(),
                data: data.to_string(),
            },
        )
        .await
    }

    /// Delete a block.
    ///
    /// # Arguments
    /// * `block_id` - The ID of the block to delete.
    ///
    /// # Warning
    /// This operation is irreversible. Child blocks will also be deleted.
    pub async fn delete_block(&self, block_id: &str) -> anyhow::Result<BlockOperations> {
        self.post_data(
            "/api/block/deleteBlock",
            &DeleteBlockRequest {
                id: block_id.to_string(),
            },
        )
        .await
    }

    /// Move a block to a new location.
    ///
    /// # Arguments
    /// * `block_id` - The ID of the block to move.
    /// * `previous_id` - The ID of the block to insert after.
    /// * `parent_id` - The ID of the new parent block.
    ///
    /// # Note
    /// If both `previous_id` and `parent_id` are provided, `previous_id` takes precedence.
    pub async fn move_block(
        &self,
        block_id: &str,
        previous_id: Option<&str>,
        parent_id: Option<&str>,
    ) -> anyhow::Result<BlockOperations> {
        self.post_data(
            "/api/block/moveBlock",
            &MoveBlockRequest {
                id: block_id.to_string(),
                previous_id: previous_id.map(|s| s.to_string()),
                parent_id: parent_id.map(|s| s.to_string()),
            },
        )
        .await
    }

    /// Fold (collapse) a block.
    ///
    /// # Arguments
    /// * `block_id` - The ID of the block to fold.
    pub async fn fold_block(&self, block_id: &str) -> anyhow::Result<()> {
        self.post_data::<_, EmptyResponse>(
            "/api/block/foldBlock",
            &FoldBlockRequest {
                id: block_id.to_string(),
            },
        )
        .await
    }

    /// Unfold (expand) a block.
    ///
    /// # Arguments
    /// * `block_id` - The ID of the block to unfold.
    pub async fn unfold_block(&self, block_id: &str) -> anyhow::Result<()> {
        self.post_data::<_, EmptyResponse>(
            "/api/block/unfoldBlock",
            &UnfoldBlockRequest {
                id: block_id.to_string(),
            },
        )
        .await
    }

    /// Get a block's kramdown content.
    ///
    /// # Arguments
    /// * `block_id` - The ID of the block.
    ///
    /// # Returns
    /// The kramdown content including the block ID and content.
    pub async fn get_block_kramdown(&self, block_id: &str) -> anyhow::Result<BlockKramdownResponse> {
        self.post_data(
            "/api/block/getBlockKramdown",
            &GetBlockKramdownRequest {
                id: block_id.to_string(),
            },
        )
        .await
    }

    /// Get child blocks of a parent block.
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent block.
    ///
    /// # Returns
    /// A vector of child block information including type and subtype.
    ///
    /// # Note
    /// Blocks under headings are also counted as child blocks.
    pub async fn get_child_blocks(&self, parent_id: &str) -> anyhow::Result<Vec<BlockInfo>> {
        self.post_data(
            "/api/block/getChildBlocks",
            &GetChildBlocksRequest {
                id: parent_id.to_string(),
            },
        )
        .await
    }

    /// Transfer block references from one definition to another.
    ///
    /// # Arguments
    /// * `from_id` - The ID of the current definition block.
    /// * `to_id` - The ID of the target definition block.
    /// * `ref_ids` - Optional list of specific reference block IDs to transfer.
    ///              If None, all reference blocks will be transferred.
    pub async fn transfer_block_ref(
        &self,
        from_id: &str,
        to_id: &str,
        ref_ids: Option<Vec<String>>,
    ) -> anyhow::Result<()> {
        self.post_data::<_, EmptyResponse>(
            "/api/block/transferBlockRef",
            &TransferBlockRefRequest {
                from_id: from_id.to_string(),
                to_id: to_id.to_string(),
                ref_ids,
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_block_request_serialization() {
        let req = InsertBlockRequest {
            data_type: "markdown".into(),
            data: "**bold**".into(),
            next_id: None,
            previous_id: Some("prev-id".into()),
            parent_id: None,
        };
        let result = serde_json::to_string(&req);
        assert!(result.is_ok());
    }
}