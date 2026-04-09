//! Chat Panel - Main conversation interface
//!
//! Displays chat messages with rich formatting including:
//! - Role-based coloring (user/assistant/system)
//! - Message timestamps
//! - Model attribution
//! - Word wrapping for long messages
//! - Scrollable message history

use crate::state::{AppState, Message, MessageRole};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// State specific to the chat panel
#[derive(Debug, Clone, Default)]
pub struct ChatPanelState {
    /// Vertical scroll offset
    pub scroll_offset: usize,

    /// Whether to show timestamps
    pub show_timestamps: bool,

    /// Whether to show model attribution
    pub show_model: bool,
}

/// Render the chat panel
pub fn render_chat_panel(frame: &mut Frame, area: Rect, app: &AppState, state: &ChatPanelState) {
    static EMPTY_MESSAGES: Vec<Message> = Vec::new();
    let messages = app
        .current_session()
        .map(|s| &s.messages)
        .unwrap_or(&EMPTY_MESSAGES);

    let mut text = Text::default();

    // Apply scroll offset (show most recent messages first)
    let visible_messages: Vec<_> = messages.iter().rev().skip(state.scroll_offset).collect();

    for msg in visible_messages.iter().rev() {
        let (prefix, style) = match msg.role {
            MessageRole::User => (
                "You: ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            MessageRole::Assistant => (
                "AI: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            MessageRole::System => (
                "System: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::ITALIC),
            ),
        };

        // Role prefix
        text.push_line(Line::styled(prefix, style));

        // Model attribution (if enabled and available)
        if state.show_model {
            if let Some(model) = &msg.model_used {
                text.push_line(Line::styled(
                    format!("  [via {}]", model),
                    Style::default().fg(Color::DarkGray).italic(),
                ));
            }
        }

        // Message content (word-wrapped)
        for line in textwrap::wrap(&msg.content, area.width as usize - 4) {
            let content_style = match msg.role {
                MessageRole::User => Style::default().fg(Color::White),
                MessageRole::Assistant => Style::default().fg(Color::White),
                MessageRole::System => Style::default().fg(Color::Yellow),
            };
            text.push_line(Line::styled(line.to_string(), content_style));
        }

        // Timestamp (if enabled)
        if state.show_timestamps {
            let time_str = msg.timestamp.format("%H:%M:%S").to_string();
            text.push_line(Line::styled(
                format!("  [{}]", time_str),
                Style::default().fg(Color::DarkGray),
            ));
        }

        // Spacing between messages
        text.push_line(Line::styled("", Style::default()));
    }

    // Empty state message
    if messages.is_empty() {
        text = Text::styled(
            "No messages yet.\n\nPress 'i' to enter insert mode and start typing.\nPress '?' for help.",
            Style::default().fg(Color::DarkGray).italic(),
        );
    }

    // Message count indicator
    let title = if messages.len() > 0 {
        format!(" Chat ({}) ", messages.len())
    } else {
        " Chat ".to_string()
    };

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title),
        )
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left)
        .scroll((0, 0));

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Session;
    use chrono::Utc;

    #[test]
    fn test_chat_panel_state_default() {
        let state = ChatPanelState::default();
        assert_eq!(state.scroll_offset, 0);
        assert!(state.show_timestamps);
        assert!(state.show_model);
    }

    #[test]
    fn test_chat_panel_with_messages() {
        let mut app = AppState::default();
        app.add_user_message("Hello, ZeroClaw!".to_string());
        app.add_assistant_message("Hi! How can I help?".to_string(), Some("gpt-4".to_string()));

        let state = ChatPanelState::default();

        // Should not panic
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))
                .unwrap();
        let _ = terminal.draw(|f| {
            render_chat_panel(f, f.area(), &app, &state);
        });
    }
}
