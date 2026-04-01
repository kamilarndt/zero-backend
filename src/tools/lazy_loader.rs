//! Lazy tool loading and dynamic selection system.
//!
//! This module implements token-efficient tool management by:
//! 1. Providing lightweight inventory (name + 1-sentence summary)
//! 2. Dynamically selecting relevant tools based on user intent
//! 3. Only injecting full JSON schemas for selected tools
//!
//! # Architecture
//!
//! - [`ToolInventory`]: Lightweight metadata for all registered tools
//! - [`ToolSelector`]: Heuristic-based selector that chooses relevant tools
//! - [`get_tool_inventory()`]: Returns lightweight tool metadata
//! - [`select_relevant_tools()`]: Filters tools based on user query

use crate::tools::Tool;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// Lightweight metadata for a tool (token-efficient)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Tool name (e.g., "file_read")
    pub name: String,

    /// One-sentence description (no verbose docs)
    pub short_description: String,

    /// Keyword hints for selection (e.g., ["file", "read", "cat"])
    pub keywords: Vec<String>,

    /// Category for grouping (e.g., "filesystem", "memory", "web")
    pub category: ToolCategory,
}

/// Tool categories for hierarchical selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    /// File system operations (read, write, edit, glob)
    Filesystem,
    /// Memory operations (store, recall, forget)
    Memory,
    /// Web operations (search, fetch, browser)
    Web,
    /// System operations (shell, cron, schedule)
    System,
    /// Communication (email, pushover, telegram, etc.)
    Communication,
    /// Development (git, task planning)
    Development,
    /// Vision/media (screenshot, image info, pdf)
    Media,
    /// Delegation (subagent, delegate to other models)
    Delegation,
    /// Other/miscellaneous tools
    Other,
}

/// Lightweight inventory of all available tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInventory {
    /// Map of tool name -> metadata
    pub tools: HashMap<String, ToolMetadata>,

    /// Categorized groups for efficient selection
    pub by_category: HashMap<ToolCategory, Vec<String>>,
}

impl ToolInventory {
    /// Create inventory from a list of tool instances
    pub fn from_tools(tools: &[Box<dyn Tool>]) -> Self {
        let mut tools_map = HashMap::new();
        let mut by_category: HashMap<ToolCategory, Vec<String>> = HashMap::new();

        for tool in tools {
            let metadata = Self::extract_metadata(tool);
            let name = metadata.name.clone();

            // Add to tools map
            tools_map.insert(name.clone(), metadata.clone());

            // Add to category index
            by_category
                .entry(metadata.category.clone())
                .or_insert_with(Vec::new)
                .push(name);
        }

        Self {
            tools: tools_map,
            by_category,
        }
    }

    /// Extract metadata from a tool instance
    fn extract_metadata(tool: &Box<dyn Tool>) -> ToolMetadata {
        let name = tool.name().to_string();
        let description = tool.description().to_string();

        // Create short description (first sentence only)
        let short_description = description
            .split('.')
            .next()
            .unwrap_or(&description)
            .trim()
            .to_string();

        // Extract keywords from description and name
        let keywords = Self::extract_keywords(&name, &description);

        // Categorize based on name and keywords
        let category = Self::categorize_tool(&name, &keywords);

        ToolMetadata {
            name,
            short_description,
            keywords,
            category,
        }
    }

    /// Extract searchable keywords from tool metadata
    fn extract_keywords(name: &str, description: &str) -> Vec<String> {
        let mut keywords = Vec::new();

        // Add name components (split on _)
        for part in name.split('_') {
            if !part.is_empty() {
                keywords.push(part.to_string());
            }
        }

        // Add common words from description (simple heuristic)
        let common_words = [
            "file", "read", "write", "edit", "search", "shell", "command",
            "memory", "store", "recall", "forget", "web", "fetch", "browser",
            "git", "schedule", "cron", "email", "notification", "screenshot",
            "image", "pdf", "delegate", "agent", "task", "plan", "http",
            "content", "glob", "directory", "folder", "list", "delete",
        ];

        let description_lower = description.to_lowercase();
        for word in common_words {
            if description_lower.contains(word) {
                keywords.push(word.to_string());
            }
        }

        // Remove duplicates
        keywords.sort();
        keywords.dedup();
        keywords
    }

    /// Categorize a tool based on name and keywords
    fn categorize_tool(name: &str, _keywords: &[String]) -> ToolCategory {
        let name_lower = name.to_lowercase();

        match name_lower.as_str() {
            n if n.contains("file") || n.contains("glob") || n.contains("content_search") => {
                ToolCategory::Filesystem
            }
            n if n.contains("memory") || n.contains("siyuan") || n.contains("qdrant") => {
                ToolCategory::Memory
            }
            n if n.contains("web") || n.contains("browser") || n.contains("fetch") || n.contains("search") => {
                ToolCategory::Web
            }
            n if n.contains("shell") || n.contains("cron") || n.contains("schedule") => {
                ToolCategory::System
            }
            n if n.contains("email") || n.contains("pushover") || n.contains("telegram")
                || n.contains("slack") || n.contains("discord") => ToolCategory::Communication,
            n if n.contains("git") || n.contains("task_plan") => ToolCategory::Development,
            n if n.contains("screenshot") || n.contains("image") || n.contains("pdf") => {
                ToolCategory::Media
            }
            n if n.contains("delegate") || n.contains("subagent") => ToolCategory::Delegation,
            _ => ToolCategory::Other,
        }
    }

    /// Get tool count
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

/// Tool selector for dynamic tool selection
pub struct ToolSelector {
    /// Maximum number of tools to select (default: 10)
    max_tools: usize,

    /// Always-include tools (regardless of query)
    always_include: HashSet<String>,

    /// Minimum keyword match score threshold (0.0 - 1.0)
    min_score: f64,
}

impl Default for ToolSelector {
    fn default() -> Self {
        Self {
            max_tools: 10,
            always_include: Self::default_always_include(),
            min_score: 0.2,
        }
    }
}

impl ToolSelector {
    /// Create a new selector with custom parameters
    pub fn new(max_tools: usize, min_score: f64) -> Self {
        Self {
            max_tools,
            always_include: Self::default_always_include(),
            min_score,
        }
    }

    /// Default tools to always include (high-use tools)
    fn default_always_include() -> HashSet<String> {
        ["shell", "file_read", "file_write", "file_edit"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Add a tool to the always-include list
    pub fn always_include(mut self, tool_name: &str) -> Self {
        self.always_include.insert(tool_name.to_string());
        self
    }

    /// Select relevant tools based on user query
    pub fn select(&self, inventory: &ToolInventory, query: &str) -> Vec<String> {
        let query_lower = query.to_lowercase();

        // Check if query is simple (no tools needed)
        if self.is_simple_query(&query_lower) {
            return Vec::new();
        }

        let mut scored_tools: Vec<(String, f64)> = Vec::new();

        // Score each tool
        for (name, metadata) in &inventory.tools {
            let score = self.score_tool(metadata, &query_lower);

            // Always include specific tools
            if self.always_include.contains(name) {
                scored_tools.push((name.clone(), 1.0));
            } else if score >= self.min_score {
                scored_tools.push((name.clone(), score));
            }
        }

        // Sort by score (descending)
        scored_tools.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Take top N tools
        scored_tools
            .into_iter()
            .take(self.max_tools)
            .map(|(name, _)| name)
            .collect()
    }

    /// Check if query is simple (doesn't need tools)
    fn is_simple_query(&self, query: &str) -> bool {
        let simple_patterns = [
            "hello", "hi ", "hey ", "status", "help", "what is",
            "explain", "describe", "tell me", "how do", "how to",
            "why", "when", "who", "where", "which", "can you",
            "could you", "would you", "thanks", "thank you", "bye",
        ];

        let query_trimmed = query.trim();

        // Check for simple greetings/status
        for pattern in &simple_patterns {
            if query_trimmed.starts_with(pattern) && query_trimmed.len() < 100 {
                return true;
            }
        }

        // Check for math expressions
        if query_trimmed.len() < 50
            && query_trimmed
                .chars()
                .all(|c| c.is_ascii_digit() || c.is_ascii_punctuation() || c.is_whitespace())
        {
            return true;
        }

        false
    }

    /// Score a tool's relevance to the query (0.0 - 1.0)
    fn score_tool(&self, metadata: &ToolMetadata, query: &str) -> f64 {
        let mut score = 0.0;

        // Exact name match = highest score
        if query.contains(&metadata.name.to_lowercase().replace('_', "")) {
            score += 0.8;
        }

        // Keyword matches
        let query_words: HashSet<&str> = query.split_whitespace().collect();
        let keyword_matches: usize = metadata
            .keywords
            .iter()
            .filter(|k| query_words.contains(&k.as_str()))
            .count();

        if !metadata.keywords.is_empty() {
            score += (keyword_matches as f64) / (metadata.keywords.len() as f64) * 0.5;
        }

        // Category bonus (if query mentions category)
        let category_keywords = self.get_category_keywords(&metadata.category);
        let category_matches: usize = category_keywords
            .iter()
            .filter(|k| query.contains(*k))
            .count();

        if !category_keywords.is_empty() {
            score += (category_matches as f64) / (category_keywords.len() as f64) * 0.2;
        }

        // Cap at 1.0
        score.min(1.0)
    }

    /// Get keywords for a category
    fn get_category_keywords(&self, category: &ToolCategory) -> Vec<&'static str> {
        match category {
            ToolCategory::Filesystem => vec!["file", "folder", "directory", "read", "write"],
            ToolCategory::Memory => vec!["memory", "remember", "recall", "store", "knowledge"],
            ToolCategory::Web => vec!["web", "internet", "browser", "search", "fetch", "url"],
            ToolCategory::System => vec!["system", "command", "shell", "execute", "run"],
            ToolCategory::Communication => vec!["send", "notify", "message", "email", "alert"],
            ToolCategory::Development => vec!["git", "code", "develop", "task", "plan"],
            ToolCategory::Media => vec!["image", "picture", "screenshot", "pdf", "media"],
            ToolCategory::Delegation => vec!["delegate", "agent", "helper", "assistant"],
            ToolCategory::Other => vec![],
        }
    }
}

/// Get lightweight inventory (names + short descriptions only)
///
/// This function is token-efficient and should be used for initial tool discovery.
pub fn get_tool_inventory(tools: &[Box<dyn Tool>]) -> ToolInventory {
    ToolInventory::from_tools(tools)
}

/// Select relevant tool names for a given query
///
/// Returns a list of tool names that should have their full schemas included
/// in the LLM context. This reduces token usage by only loading schemas for
/// relevant tools.
pub fn select_relevant_tool_names(
    tools: &[Box<dyn Tool>],
    query: &str,
    max_tools: Option<usize>,
) -> Vec<String> {
    let inventory = get_tool_inventory(tools);

    let selector = match max_tools {
        Some(n) => ToolSelector::new(n, 0.2),
        None => ToolSelector::default(),
    };

    selector.select(&inventory, query)
}

/// Build token-efficient tool instructions (only names + short descriptions)
///
/// This should replace `build_tool_instructions` in the agent loop for
/// non-tool-using providers or for initial tool discovery.
pub fn build_lightweight_tool_instructions(inventory: &ToolInventory) -> String {
    let mut instructions = String::new();
    instructions.push_str("\n## Available Tools\n\n");
    instructions.push_str("You have access to the following tools:\n\n");

    // Group by category for better organization
    for category in &[
        ToolCategory::Filesystem,
        ToolCategory::Memory,
        ToolCategory::Web,
        ToolCategory::System,
        ToolCategory::Communication,
        ToolCategory::Development,
        ToolCategory::Media,
        ToolCategory::Delegation,
        ToolCategory::Other,
    ] {
        if let Some(tool_names) = inventory.by_category.get(category) {
            if !tool_names.is_empty() {
                let _ = writeln!(instructions, "### {:?}\n", category);
                for name in tool_names {
                    if let Some(metadata) = inventory.tools.get(name) {
                        let _ = writeln!(
                            instructions,
                            "- **{}**: {}",
                            metadata.name, metadata.short_description
                        );
                    }
                }
                instructions.push('\n');
            }
        }
    }

    instructions.push_str(
        "To use a tool, specify its name and required parameters. \
         Full parameter schemas will be provided when you request a specific tool.\n",
    );

    instructions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::traits::ToolResult;
    use async_trait::async_trait;

    // Mock tool for testing
    struct MockTool {
        name: String,
        description: String,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
            Ok(ToolResult {
                success: true,
                output: "mock".to_string(),
                error: None,
            })
        }
    }

    #[test]
    fn test_tool_inventory_creation() {
        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(MockTool {
                name: "file_read".to_string(),
                description: "Read a file from the filesystem. This tool allows reading file contents.".to_string(),
            }),
            Box::new(MockTool {
                name: "memory_store".to_string(),
                description: "Store information in memory for later retrieval.".to_string(),
            }),
        ];

        let inventory = get_tool_inventory(&tools);

        assert_eq!(inventory.len(), 2);
        assert!(inventory.tools.contains_key("file_read"));
        assert!(inventory.tools.contains_key("memory_store"));

        // Check file_read metadata
        let file_read_meta = &inventory.tools["file_read"];
        assert_eq!(
            file_read_meta.short_description,
            "Read a file from the filesystem"
        );
        assert_eq!(file_read_meta.category, ToolCategory::Filesystem);
    }

    #[test]
    fn test_simple_query_detection() {
        let selector = ToolSelector::default();

        assert!(selector.is_simple_query("hello"));
        assert!(selector.is_simple_query("what is 2+2"));
        assert!(selector.is_simple_query("status"));
        assert!(!selector.is_simple_query("read file foo.txt"));
        assert!(!selector.is_simple_query("search the web for rust"));
    }

    #[test]
    fn test_tool_selection() {
        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(MockTool {
                name: "file_read".to_string(),
                description: "Read a file from the filesystem.".to_string(),
            }),
            Box::new(MockTool {
                name: "web_search".to_string(),
                description: "Search the web for information.".to_string(),
            }),
            Box::new(MockTool {
                name: "memory_store".to_string(),
                description: "Store information in memory.".to_string(),
            }),
        ];

        let inventory = get_tool_inventory(&tools);
        let selector = ToolSelector::default();

        // Query about files should select file_read
        let selected = selector.select(&inventory, "read the file foo.txt");
        assert!(selected.contains(&"file_read".to_string()));
        assert!(!selected.contains(&"web_search".to_string()));

        // Query about web should select web_search
        let selected = selector.select(&inventory, "search the web for rust");
        assert!(selected.contains(&"web_search".to_string()));

        // Simple query should select no tools
        let selected = selector.select(&inventory, "hello");
        assert!(selected.is_empty());
    }

    #[test]
    fn test_lightweight_instructions() {
        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(MockTool {
                name: "file_read".to_string(),
                description: "Read a file from the filesystem.".to_string(),
            }),
        ];

        let inventory = get_tool_inventory(&tools);
        let instructions = build_lightweight_tool_instructions(&inventory);

        assert!(instructions.contains("file_read"));
        assert!(instructions.contains("Filesystem"));
        // Should NOT contain full JSON schema
        assert!(!instructions.contains("\"type\":"));
    }
}
