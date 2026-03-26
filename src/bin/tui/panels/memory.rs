//! Memory Panel - Memory system inspector
//!
//! Displays:
//! - Active memory backend (SQLite/Qdrant/etc)
//! - Total memories stored
//! - Storage size in human-readable format
//! - Recent memory operations (store/recall/search/delete)
//! - Memory category breakdown

use crate::state::AppState;
use crate::state::subsystems::{MemoryOpType, MemoryOperation};
use ratatui::{
    style::Stylize,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// State specific to the memory panel
#[derive(Debug, Clone, Default)]
pub struct MemoryPanelState {
    /// Whether panel is visible
    pub visible: bool,

    /// Active memory backend
    pub backend: String,

    /// Total memories count
    pub total_memories: usize,

    /// Storage size in bytes
    pub storage_bytes: u64,

    /// Recent memory operations
    pub recent_operations: Vec<MemoryOperation>,
}

/// Re-export types from subsystems for external use

/// Render the memory panel
pub fn render_memory_panel(
    frame: &mut Frame,
    area: Rect,
    _app: &AppState,
    state: &MemoryPanelState,
) {
    if !state.visible {
        render_hidden_panel(frame, area, " Memory (press 'm' to toggle) ");
        return;
    }

    let mut text = Text::default();

    // Backend info
    text.push_line(Line::styled(
        format!("Backend: {}", state.backend),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));
    text.push_line(Line::styled("", Style::default()));

    // Statistics
    text.push_line(Line::styled(
        format!("Total Memories: {}", state.total_memories),
        Style::default().fg(Color::Green),
    ));

    let size_human = bytes_to_human(state.storage_bytes);
    text.push_line(Line::styled(
        format!("Storage Size: {}", size_human),
        Style::default().fg(Color::Yellow),
    ));
    text.push_line(Line::styled("", Style::default()));

    // Recent operations
    if state.recent_operations.is_empty() {
        text.push_line(Line::styled(
            "No recent operations.",
            Style::default().fg(Color::DarkGray).italic(),
        ));
    } else {
        text.push_line(Line::styled(
            "Recent Operations:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        text.push_line(Line::styled("", Style::default()));

        // Show last 10 operations
        for op in state.recent_operations.iter().rev().take(10) {
            let status_symbol = if op.success { "✓" } else { "✗" };
            let status_color = if op.success { Color::Green } else { Color::Red };

            let op_color = op.op_type.clone().color();

            text.push_line(Line::from(vec![
                Span::styled(status_symbol, Style::default().fg(status_color)),
                Span::styled(" ", Style::default()),
                Span::styled(
                    op.op_type.as_str(),
                    Style::default().fg(op_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ", Style::default()),
                Span::styled(
                    op.key.as_deref().unwrap_or("(none)"),
                    Style::default().fg(Color::White).italic(),
                ),
            ]));
        }
    }

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta))
                .title(" Memory Inspector "),
        )
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

/// Render a placeholder for hidden panel
fn render_hidden_panel(frame: &mut Frame, area: Rect, title: &str) {
    let paragraph = Paragraph::new("Panel hidden. Press toggle key to show.")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title),
        )
        .style(Style::default().fg(Color::DarkGray).italic());

    frame.render_widget(paragraph, area);
}

/// Convert bytes to human-readable format
fn bytes_to_human(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_memory_panel_state_default() {
        let state = MemoryPanelState::default();
        assert!(!state.visible);
        assert_eq!(state.total_memories, 0);
        assert_eq!(state.storage_bytes, 0);
    }

    #[test]
    fn test_memory_op_type_colors() {
        assert_eq!(MemoryOpType::Store.color(), Color::Green);
        assert_eq!(MemoryOpType::Recall.color(), Color::Cyan);
        assert_eq!(MemoryOpType::Search.color(), Color::Yellow);
        assert_eq!(MemoryOpType::Delete.color(), Color::Red);
        assert_eq!(MemoryOpType::Clear.color(), Color::Magenta);
    }

    #[test]
    fn test_bytes_to_human() {
        assert_eq!(bytes_to_human(0), "0 B");
        assert_eq!(bytes_to_human(1024), "1.00 KB");
        assert_eq!(bytes_to_human(1024 * 1024), "1.00 MB");
        assert_eq!(bytes_to_human(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_memory_op_rendering() {
        let state = MemoryPanelState {
            visible: true,
            backend: "sqlite".to_string(),
            total_memories: 42,
            storage_bytes: 1024 * 1024,
            recent_operations: vec![MemoryOperation {
                op_type: MemoryOpType::Store,
                key: Some("test_key".to_string()),
                success: true,
                timestamp: Utc::now(),
            }],
        };

        // Should not panic
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))
                .unwrap();
        let _ = terminal.draw(|f| {
            render_memory_panel(f, f.area(), &AppState::default(), &state);
        });
    }
}
