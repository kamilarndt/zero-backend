//! Session management extensions for ZeroClaw TUI
//!
//! This module provides additional session operations beyond the basic
//! methods in app.rs, including session persistence and export.

use super::app::{AppState, Message, MessageRole};
use anyhow::Result;
use chrono::Utc;
use std::fs;
use std::path::Path;

impl AppState {
    /// Export current session to markdown
    pub fn export_session_markdown(&self, path: &Path) -> Result<()> {
        if let Some(session) = self.current_session() {
            let mut content = format!("# {}\n\n", session.name);
            content.push_str(&format!("Created: {}\n\n", session.created_at.to_rfc3339()));
            content.push_str("---\n\n");

            for msg in &session.messages {
                match msg.role {
                    MessageRole::User => {
                        content.push_str(&format!("## User\n{}\n\n", msg.content));
                    }
                    MessageRole::Assistant => {
                        let model_note = msg
                            .model_used
                            .as_ref()
                            .map(|m| format!(" (using {})", m))
                            .unwrap_or_default();
                        content
                            .push_str(&format!("## Assistant{}\n{}\n\n", model_note, msg.content));
                    }
                    MessageRole::System => {
                        content.push_str(&format!("## System\n{}\n\n", msg.content));
                    }
                }
            }

            fs::write(path, content)?;
        }
        Ok(())
    }

    /// Import messages from markdown (basic format)
    pub fn import_session_markdown(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();

        let mut messages = Vec::new();
        let mut current_role: Option<MessageRole> = None;
        let mut current_content = String::new();

        for line in lines {
            if line.starts_with("## User") {
                if let Some(role) = current_role.take() {
                    messages.push(Message {
                        role,
                        content: current_content.trim().to_string(),
                        model_used: None,
                        timestamp: Utc::now(),
                    });
                    current_content.clear();
                }
                current_role = Some(MessageRole::User);
            } else if line.starts_with("## Assistant") {
                if let Some(role) = current_role.take() {
                    messages.push(Message {
                        role,
                        content: current_content.trim().to_string(),
                        model_used: None,
                        timestamp: Utc::now(),
                    });
                    current_content.clear();
                }
                current_role = Some(MessageRole::Assistant);
            } else if line.starts_with("## System") {
                if let Some(role) = current_role.take() {
                    messages.push(Message {
                        role,
                        content: current_content.trim().to_string(),
                        model_used: None,
                        timestamp: Utc::now(),
                    });
                    current_content.clear();
                }
                current_role = Some(MessageRole::System);
            } else if !line.starts_with('#') && !line.starts_with("---") {
                if current_role.is_some() {
                    if !current_content.is_empty() {
                        current_content.push('\n');
                    }
                    current_content.push_str(line);
                }
            }
        }

        // Add last message
        if let Some(role) = current_role {
            messages.push(Message {
                role,
                content: current_content.trim().to_string(),
                model_used: None,
                timestamp: Utc::now(),
            });
        }

        if let Some(session) = self.current_session_mut() {
            session.messages.extend(messages);
        }

        Ok(())
    }

    /// Get session statistics
    pub fn session_stats(&self) -> SessionStats {
        if let Some(session) = self.current_session() {
            let user_messages = session
                .messages
                .iter()
                .filter(|m| m.role == MessageRole::User)
                .count();

            let assistant_messages = session
                .messages
                .iter()
                .filter(|m| m.role == MessageRole::Assistant)
                .count();

            let total_chars = session
                .messages
                .iter()
                .map(|m| m.content.len())
                .sum::<usize>();

            SessionStats {
                total_messages: session.messages.len(),
                user_messages,
                assistant_messages,
                total_characters: total_chars,
            }
        } else {
            SessionStats::default()
        }
    }
}

/// Session statistics
#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub total_messages: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub total_characters: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_export_session_markdown() {
        let mut app = AppState::default();
        app.add_user_message("Hello, ZeroClaw!".to_string());
        app.add_assistant_message("Hello! How can I help?".to_string(), None);

        let temp_file = NamedTempFile::new().unwrap();
        app.export_session_markdown(temp_file.path()).unwrap();

        let content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("Hello, ZeroClaw!"));
        assert!(content.contains("Hello! How can I help?"));
        assert!(content.contains("# Session 1"));
    }

    #[test]
    fn test_session_stats() {
        let mut app = AppState::default();
        app.add_user_message("Test 1".to_string());
        app.add_assistant_message("Response 1".to_string(), None);
        app.add_user_message("Test 2".to_string());

        let stats = app.session_stats();
        assert_eq!(stats.total_messages, 3);
        assert_eq!(stats.user_messages, 2);
        assert_eq!(stats.assistant_messages, 1);
    }
}
