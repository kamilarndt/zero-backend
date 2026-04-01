//! Zed command parsing for ACP protocol.
//!
//! This module extracts Zed Editor command parsing from the main ACP module.
//! It handles parsing of special command syntax like `[[ZED:open:path/to/file:42]]`

use serde::{Deserialize, Serialize};

/// Zed Editor UI command that the agent can request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ZedCommand {
    /// Open a file in Zed, optionally at a specific line
    OpenFile {
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        line: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        column: Option<u32>,
    },
    /// Scroll to a specific line in the current file
    ScrollTo {
        line: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        column: Option<u32>,
    },
    /// Highlight a range of text
    Highlight {
        start: Position,
        end: Position,
    },
    /// Show a notification/info message
    Notify {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        level: Option<NotifyLevel>,
    },
}

/// Position in a file (line and column).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

/// Notification level for Zed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotifyLevel {
    Info,
    Warning,
    Error,
}

impl ZedCommand {
    /// Create an openFile command.
    pub fn open_file(path: impl Into<String>) -> Self {
        Self::OpenFile {
            path: path.into(),
            line: None,
            column: None,
        }
    }

    /// Create an openFile command with line number.
    pub fn open_file_at_line(path: impl Into<String>, line: u32) -> Self {
        Self::OpenFile {
            path: path.into(),
            line: Some(line),
            column: None,
        }
    }

    /// Create a scrollTo command.
    pub fn scroll_to(line: u32) -> Self {
        Self::ScrollTo { line, column: None }
    }

    /// Create a highlight command.
    pub fn highlight(start_line: u32, end_line: u32) -> Self {
        Self::Highlight {
            start: Position { line: start_line, column: 0 },
            end: Position { line: end_line, column: 0 },
        }
    }
}

/// Parse a single Zed command from text.
/// Returns None if the line is not a command.
pub fn parse_zed_command(text: &str) -> Option<ZedCommand> {
    let text = text.trim();

    // Check for ZED command marker
    if !text.starts_with("[[ZED:") || !text.ends_with("]]") {
        return None;
    }

    let inner = &text[6..text.len() - 2]; // Strip [[ZED: and ]]

    let parts: Vec<&str> = inner.split(':').collect();

    match parts.first() {
        Some(&"open") => {
            // open:path or open:path:line or open:path:line:col
            let path = parts.get(1)?;
            let line = parts.get(2).and_then(|s| s.parse::<u32>().ok());
            let column = parts.get(3).and_then(|s| s.parse::<u32>().ok());

            Some(ZedCommand::OpenFile {
                path: path.to_string(),
                line,
                column,
            })
        }
        Some(&"scroll") => {
            // scroll:line or scroll:line:col
            let line = parts.get(1).and_then(|s| s.parse::<u32>().ok())?;
            let column = parts.get(2).and_then(|s| s.parse::<u32>().ok());

            Some(ZedCommand::ScrollTo { line, column })
        }
        Some(&"highlight") => {
            // highlight:start_line:end_line
            let start_line = parts.get(1).and_then(|s| s.parse::<u32>().ok())?;
            let end_line = parts.get(2).and_then(|s| s.parse::<u32>().ok())?;

            Some(ZedCommand::Highlight {
                start: Position { line: start_line, column: 0 },
                end: Position { line: end_line, column: 0 },
            })
        }
        Some(&"info") | Some(&"notify") => {
            let message = parts.get(1).unwrap_or(&"").to_string();
            Some(ZedCommand::Notify {
                message,
                level: Some(NotifyLevel::Info),
            })
        }
        Some(&"warn") | Some(&"warning") => {
            let message = parts.get(1).unwrap_or(&"").to_string();
            Some(ZedCommand::Notify {
                message,
                level: Some(NotifyLevel::Warning),
            })
        }
        Some(&"error") => {
            let message = parts.get(1).unwrap_or(&"").to_string();
            Some(ZedCommand::Notify {
                message,
                level: Some(NotifyLevel::Error),
            })
        }
        _ => None,
    }
}

/// Parse Zed commands from agent response text.
///
/// The agent can embed commands using special syntax:
/// - `[[ZED:open:path/to/file]]`
/// - `[[ZED:open:path/to/file:42]]` (with line number)
/// - `[[ZED:scroll:100]]`
/// - `[[ZED:highlight:10:20]]`
/// - `[[ZED:info:message]]`, `[[ZED:warn:message]]`, `[[ZED:error:message]]`
///
/// Returns (cleaned_text, commands)
pub fn parse_zed_commands(response: &str) -> (String, Vec<ZedCommand>) {
    let mut commands = Vec::new();
    let mut cleaned_lines = Vec::new();

    for line in response.lines() {
        let trimmed = line.trim();

        if let Some(command) = parse_zed_command(trimmed) {
            commands.push(command);
        } else {
            cleaned_lines.push(line);
        }
    }

    let cleaned = cleaned_lines.join("\n");
    (cleaned, commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_zed_open_file() {
        let cmd = parse_zed_command("[[ZED:open:Cargo.toml]]").unwrap();
        match cmd {
            ZedCommand::OpenFile { path, line, column } => {
                assert_eq!(path, "Cargo.toml");
                assert_eq!(line, None);
                assert_eq!(column, None);
            }
            _ => panic!("Expected OpenFile command"),
        }
    }

    #[test]
    fn test_parse_zed_open_file_with_line() {
        let cmd = parse_zed_command("[[ZED:open:src/main.rs:42]]").unwrap();
        match cmd {
            ZedCommand::OpenFile { path, line, column } => {
                assert_eq!(path, "src/main.rs");
                assert_eq!(line, Some(42));
                assert_eq!(column, None);
            }
            _ => panic!("Expected OpenFile command"),
        }
    }

    #[test]
    fn test_parse_zed_scroll() {
        let cmd = parse_zed_command("[[ZED:scroll:100]]").unwrap();
        match cmd {
            ZedCommand::ScrollTo { line, column } => {
                assert_eq!(line, 100);
                assert_eq!(column, None);
            }
            _ => panic!("Expected ScrollTo command"),
        }
    }

    #[test]
    fn test_parse_zed_commands_from_response() {
        let response = "I'll open the file for you.\n\
            [[ZED:open:src/main.rs:42]]\n\
            The bug is on line 42.";

        let (cleaned, commands) = parse_zed_commands(response);

        assert!(!cleaned.contains("[[ZED:"));
        assert!(!cleaned.contains("[["));
        assert_eq!(commands.len(), 1);

        match &commands[0] {
            ZedCommand::OpenFile { path, line, .. } => {
                assert_eq!(path, "src/main.rs");
                assert_eq!(*line, Some(42));
            }
            _ => panic!("Expected OpenFile command"),
        }
    }

    #[test]
    fn test_parse_zed_multiple_commands() {
        let response = "Checking the file...\n\
            [[ZED:open:Cargo.toml]]\n\
            [[ZED:scroll:10]]\n\
            [[ZED:highlight:5:15]]\n\
            Done!";

        let (cleaned, commands) = parse_zed_commands(response);

        assert_eq!(commands.len(), 3);
        assert!(cleaned.contains("Checking the file"));
        assert!(cleaned.contains("Done!"));
    }

    #[test]
    fn test_zed_command_serialize() {
        let cmd = ZedCommand::open_file_at_line("src/main.rs", 42);
        let json = serde_json::to_string(&cmd).unwrap();

        assert!(json.contains("\"type\":\"openFile\""));
        assert!(json.contains("\"path\":\"src/main.rs\""));
        assert!(json.contains("\"line\":42"));
    }

    #[test]
    fn test_parse_zed_notify_info() {
        let cmd = parse_zed_command("[[ZED:info:This is a message]]").unwrap();
        match cmd {
            ZedCommand::Notify { message, level } => {
                assert_eq!(message, "This is a message");
                assert_eq!(level, Some(NotifyLevel::Info));
            }
            _ => panic!("Expected Notify command"),
        }
    }

    #[test]
    fn test_parse_zed_notify_warning() {
        let cmd = parse_zed_command("[[ZED:warn:Warning message]]").unwrap();
        match cmd {
            ZedCommand::Notify { message, level } => {
                assert_eq!(message, "Warning message");
                assert_eq!(level, Some(NotifyLevel::Warning));
            }
            _ => panic!("Expected Notify command"),
        }
    }

    #[test]
    fn test_parse_zed_notify_error() {
        let cmd = parse_zed_command("[[ZED:error:Error occurred]]").unwrap();
        match cmd {
            ZedCommand::Notify { message, level } => {
                assert_eq!(message, "Error occurred");
                assert_eq!(level, Some(NotifyLevel::Error));
            }
            _ => panic!("Expected Notify command"),
        }
    }

    #[test]
    fn test_parse_zed_highlight() {
        let cmd = parse_zed_command("[[ZED:highlight:10:20]]").unwrap();
        match cmd {
            ZedCommand::Highlight { start, end } => {
                assert_eq!(start.line, 10);
                assert_eq!(end.line, 20);
            }
            _ => panic!("Expected Highlight command"),
        }
    }

    #[test]
    fn test_parse_zed_invalid_command() {
        // Not a ZED command
        assert!(parse_zed_command("Some regular text").is_none());
        // Missing closing brackets
        assert!(parse_zed_command("[[ZED:open:test.txt").is_none());
        // Wrong prefix
        assert!(parse_zed_command("[[NOTZED:open:test.txt]]").is_none());
    }

    #[test]
    fn test_parse_zed_open_file_with_column() {
        let cmd = parse_zed_command("[[ZED:open:src/main.rs:42:10]]").unwrap();
        match cmd {
            ZedCommand::OpenFile { path, line, column } => {
                assert_eq!(path, "src/main.rs");
                assert_eq!(line, Some(42));
                assert_eq!(column, Some(10));
            }
            _ => panic!("Expected OpenFile command"),
        }
    }

    #[test]
    fn test_parse_zed_scroll_with_column() {
        let cmd = parse_zed_command("[[ZED:scroll:100:5]]").unwrap();
        match cmd {
            ZedCommand::ScrollTo { line, column } => {
                assert_eq!(line, 100);
                assert_eq!(column, Some(5));
            }
            _ => panic!("Expected ScrollTo command"),
        }
    }
}
