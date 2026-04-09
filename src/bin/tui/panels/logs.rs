//! Logs Panel - Toggleable system log tail
//!
//! Displays:
//! - Real-time system logs
//! - Log level filtering (ERROR/WARN/INFO/DEBUG)
//! - Color-coded log levels
//! - Module/source attribution
//! - Rolling buffer (last 200 lines)

use crate::state::subsystems::{LogLevel, LogLine};
use crate::state::AppState;
use ratatui::{
    layout::{Alignment, Rect},
    style::Stylize,
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// State specific to the logs panel
#[derive(Debug, Clone, Default)]
pub struct LogsPanelState {
    /// Whether panel is visible
    pub visible: bool,

    /// Current log level filter
    pub log_level: LogLevel,

    /// Log lines buffer
    pub log_lines: Vec<LogLine>,

    /// Maximum lines to keep
    pub max_lines: usize,

    /// Scroll offset
    pub scroll_offset: usize,
}

/// Re-export LogLevel for external use

/// Render the logs panel
pub fn render_logs_panel(frame: &mut Frame, area: Rect, _app: &AppState, state: &LogsPanelState) {
    if !state.visible {
        render_hidden_panel(frame, area, " Logs (press 'l' to toggle) ");
        return;
    }

    let mut text = Text::default();

    // Filter logs by level
    let filtered_logs: Vec<_> = state
        .log_lines
        .iter()
        .filter(|log| log.level <= state.log_level)
        .collect();

    if filtered_logs.is_empty() {
        text = Text::styled(
            "No logs to display.\n\nLogs will appear here as the system runs.",
            Style::default().fg(Color::DarkGray).italic(),
        );
    } else {
        // Apply scroll offset (show most recent first)
        let visible_logs: Vec<_> = filtered_logs
            .iter()
            .rev()
            .skip(state.scroll_offset)
            .collect();

        for log in visible_logs.iter().rev() {
            let level = log.level;
            let level_color = level.color();

            // Timestamp
            let time_str = log.timestamp.format("%H:%M:%S%.3f").to_string();
            text.push_line(Line::styled(
                format!("[{}]", time_str),
                Style::default().fg(Color::DarkGray).italic(),
            ));

            // Level
            text.push_line(Line::styled(
                format!(" {} ", level.as_str()),
                Style::default()
                    .fg(level_color)
                    .add_modifier(Modifier::BOLD),
            ));

            // Module (if present)
            if let Some(module) = &log.module {
                text.push_line(Line::styled(
                    format!("{}: ", module),
                    Style::default().fg(Color::Cyan).italic(),
                ));
            }

            // Message (word-wrapped)
            for line in textwrap::wrap(&log.message, area.width as usize - 4) {
                text.push_line(Line::styled(
                    format!("  {}", line),
                    Style::default().fg(Color::White),
                ));
            }

            text.push_line(Line::styled("", Style::default()));
        }
    }

    // Level filter indicator
    let _title = format!(
        " Logs [{}:{}] ",
        state.log_level.as_str(),
        filtered_logs.len()
    );

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" System Logs "),
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_logs_panel_state_default() {
        let state = LogsPanelState::default();
        assert!(!state.visible);
        assert_eq!(state.log_level, LogLevel::Info);
        assert_eq!(state.max_lines, 200);
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn test_log_level_colors() {
        assert_eq!(LogLevel::Error.color(), Color::Red);
        assert_eq!(LogLevel::Warn.color(), Color::Yellow);
        assert_eq!(LogLevel::Info.color(), Color::Cyan);
        assert_eq!(LogLevel::Debug.color(), Color::DarkGray);
        assert_eq!(LogLevel::Trace.color(), Color::Gray);
    }

    #[test]
    fn test_log_level_parsing() {
        assert_eq!(LogLevel::from_str("ERROR"), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_str("warn"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::from_str("invalid"), None);
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Trace);
    }

    #[test]
    fn test_log_rendering() {
        let state = LogsPanelState {
            visible: true,
            log_level: LogLevel::Info,
            log_lines: vec![LogLine {
                level: LogLevel::Info,
                message: "Test log message".to_string(),
                module: Some("test_module".to_string()),
                timestamp: Utc::now(),
            }],
            max_lines: 200,
            scroll_offset: 0,
        };

        // Should not panic
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))
                .unwrap();
        let _ = terminal.draw(|f| {
            render_logs_panel(f, f.area(), &AppState::default(), &state);
        });
    }
}
