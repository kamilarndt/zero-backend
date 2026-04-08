//! Conversation Logger — JSONL logging for pattern analysis
//!
//! Logs every conversation turn to a JSONL file (one JSON object per line)
//! for later analysis of interaction patterns, emotional trajectories, and outcomes.

use crate::buddy::{EmotionalState, Situation};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Outcome of a conversation turn
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversationOutcome {
    /// Conversation is still ongoing
    Continued,
    /// Task or topic was completed
    Completed,
    /// User pivoted to a different topic/task
    Pivoted,
    /// User abandoned the conversation
    Abandoned,
}

/// A single recorded conversation turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub timestamp: DateTime<Utc>,
    pub user_input: String,
    pub buddy_response: String,
    pub emotion: EmotionalState,
    pub situation: Situation,
    pub outcome: ConversationOutcome,
}

/// Logger that appends conversation turns to a JSONL file
pub struct ConversationLogger {
    log_path: PathBuf,
}

impl ConversationLogger {
    /// Create a logger with the default path (~/.buddy/conversations.jsonl)
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let log_path = Path::new(&home).join(".buddy").join("conversations.jsonl");
        Self { log_path }
    }

    /// Create a logger with a custom file path
    pub fn with_path(path: PathBuf) -> Self {
        Self { log_path: path }
    }

    /// Append a conversation turn as a single JSON line
    pub fn log(&self, turn: &ConversationTurn) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.log_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory: {}", parent.display()))?;
        }

        let json = serde_json::to_string(turn)
            .context("failed to serialize conversation turn")?;

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .with_context(|| format!("failed to open log file: {}", self.log_path.display()))?;

        writeln!(file, "{}", json)
            .with_context(|| format!("failed to write to log file: {}", self.log_path.display()))?;

        Ok(())
    }

    /// Read all recorded conversation turns
    pub fn read_all(&self) -> Result<Vec<ConversationTurn>> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&self.log_path)
            .with_context(|| format!("failed to open log file: {}", self.log_path.display()))?;

        let reader = BufReader::new(file);
        let mut turns = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.with_context(|| format!("failed to read line {}", line_num + 1))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let turn: ConversationTurn = serde_json::from_str(trimmed)
                .with_context(|| format!("failed to parse line {}", line_num + 1))?;
            turns.push(turn);
        }

        Ok(turns)
    }

    /// Count the number of logged turns
    pub fn count(&self) -> Result<usize> {
        if !self.log_path.exists() {
            return Ok(0);
        }

        let file = fs::File::open(&self.log_path)
            .with_context(|| format!("failed to open log file: {}", self.log_path.display()))?;

        let reader = BufReader::new(file);
        let mut count = 0;

        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                count += 1;
            }
        }

        Ok(count)
    }
}

impl Default for ConversationLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_test_turn(situation: Situation, outcome: ConversationOutcome) -> ConversationTurn {
        ConversationTurn {
            timestamp: Utc::now(),
            user_input: "test input".into(),
            buddy_response: "test response".into(),
            emotion: EmotionalState::Neutral,
            situation,
            outcome,
        }
    }

    #[test]
    fn test_new_creates_logger() {
        let logger = ConversationLogger::new();
        assert!(logger.log_path.to_string_lossy().contains("conversations.jsonl"));
    }

    #[test]
    fn test_log_creates_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test_log.jsonl");
        let logger = ConversationLogger::with_path(path.clone());

        let turn = make_test_turn(Situation::TaskCompleted, ConversationOutcome::Completed);
        logger.log(&turn).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn test_log_and_read_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("roundtrip.jsonl");
        let logger = ConversationLogger::with_path(path);

        let turn1 = make_test_turn(Situation::TaskCompleted, ConversationOutcome::Completed);
        let turn2 = make_test_turn(Situation::ProgressMade, ConversationOutcome::Continued);

        logger.log(&turn1).unwrap();
        logger.log(&turn2).unwrap();

        let turns = logger.read_all().unwrap();
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].situation, Situation::TaskCompleted);
        assert_eq!(turns[1].situation, Situation::ProgressMade);
    }

    #[test]
    fn test_count_returns_correct_number() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("count.jsonl");
        let logger = ConversationLogger::with_path(path);

        assert_eq!(logger.count().unwrap(), 0);

        let turn = make_test_turn(Situation::NewDay, ConversationOutcome::Continued);
        logger.log(&turn).unwrap();
        assert_eq!(logger.count().unwrap(), 1);

        logger.log(&turn).unwrap();
        logger.log(&turn).unwrap();
        assert_eq!(logger.count().unwrap(), 3);
    }

    #[test]
    fn test_with_custom_path() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("subdir").join("custom.jsonl");
        let logger = ConversationLogger::with_path(path.clone());

        let turn = make_test_turn(Situation::UserEngaged, ConversationOutcome::Pivoted);
        logger.log(&turn).unwrap();

        assert!(path.exists());
        let turns = logger.read_all().unwrap();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].outcome, ConversationOutcome::Pivoted);
    }
}
