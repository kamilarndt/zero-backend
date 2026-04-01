//! Data structures for the knowledge base.

use serde::{Deserialize, Serialize};

// ── Documents ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateDocument {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDocument {
    pub title: Option<String>,
}

// ── Blocks ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BlockType {
    Paragraph,
    Heading,
}

impl std::fmt::Display for BlockType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockType::Paragraph => write!(f, "paragraph"),
            BlockType::Heading => write!(f, "heading"),
        }
    }
}

impl std::str::FromStr for BlockType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "paragraph" => Ok(BlockType::Paragraph),
            "heading" => Ok(BlockType::Heading),
            _ => anyhow::bail!("unknown block type: {s}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    pub document_id: String,
    pub block_type: BlockType,
    /// JSON content — e.g. `{"text": "Hello"}` for paragraph, `{"level": 1, "text": "Title"}` for heading.
    pub content: serde_json::Value,
    pub position: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateBlock {
    pub block_type: BlockType,
    pub content: serde_json::Value,
    pub position: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBlock {
    pub content: Option<serde_json::Value>,
    pub position: Option<i64>,
}

// ── Composite response ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DocumentWithBlocks {
    #[serde(flatten)]
    pub document: Document,
    pub blocks: Vec<Block>,
}
