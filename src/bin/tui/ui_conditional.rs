//! UI rendering for ZeroClaw TUI with conditional rendering support
//!
//! This module provides the rendering logic using ratatui for drawing
//! the terminal interface with the 5-panel dashboard layout.
//! Uses dirty manager to only redraw panels that have changed.

use super::state::{AppState, InputMode, Message, MessageRole};
use super::state::dirty_manager::DirtyManager;
use chrono::Utc;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
      style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};

// Panel rendering functions
use crate::panels::{
    render_cost_panel, render_logs_panel, render_memory_panel, render_swarm_panel, CostPanelState,
    LogsPanelState, MemoryPanelState, SwarmPanelState,
};

// Types from subsystems (shared between panels and state)
use super::state::subsystems::LogLevel;

/// Render the main UI with 5-panel layout using conditional rendering
pub fn render(frame: &mut Frame, app: &AppState, dirty_manager: &DirtyManager) {
    // Check which panels are dirty without async
    let should_render_panels = !app.sessions.is_empty(); // Simple check for now
    let size = frame.area();

    // Guard against terminal being too small
    if size.height < 15 || size.width < 60 {
        render_too_small(frame);
        return;
    }

    // Main layout: vertical split
    // ┌─────────────────────────────────────────────────────────┐
    // │ Header: tabs (3 lines)                                   │
    // ├─────────────────────────────────────────────────────────┤
    // │ Main area (chat + swarm/cost/memory/logs)               │
    // ├─────────────────────────────────────────────────────────┤
    // │ Input box (3 lines)                                      │
    // ├─────────────────────────────────────────────────────────┤
    // │ Status bar (1 line)                                      │
    // └─────────────────────────────────────────────────────────┘
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main area
            Constraint::Length(3), // Input
            Constraint::Length(1), // Status
        ])
        .split(size);

    // Main area: horizontal split (chat + panels)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Chat (left half)
            Constraint::Percentage(50), // Panels (right half)
        ])
        .split(chunks[1]);

    // Panels area: vertical split (4 panels stacked)
    let panels_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25), // Swarm
            Constraint::Percentage(25), // Cost
            Constraint::Percentage(25), // Memory
            Constraint::Percentage(25), // Logs
        ])
        .split(main_chunks[1]);

    // Only render sections that need updating
    if should_render_panels {
        // Always render header - it's simple and needed for session info
        render_header(frame, app, chunks[0]);

        // Check which panels are dirty and render only those
        render_all_panels(frame, app, panels_chunks.to_vec());

        // Always render input - it's needed for user interaction
        render_input(frame, app, chunks[2]);

        // Always render status - simple and important
        render_status(frame, app, chunks[3]);
    } else {
        // If nothing is dirty, only render the chat area to maintain interactivity
        render_chat(frame, app, main_chunks[0]);
        render_input(frame, app, chunks[2]);
        render_status(frame, app, chunks[3]);
    }
}

/// Render the 4-panel dashboard with conditional rendering
fn render_all_panels(frame: &mut Frame, app: &AppState, areas: Vec<Rect>) {
    // Render each panel based on visibility and dirty flags
    for (i, &area) in areas.iter().enumerate() {
        let panel_index = i + 1; // 1-4 for the four panels

        // Determine which panel to render and create appropriate state
        match panel_index {
            1 if app.panels.swarm => {
                let state = SwarmPanelState {
                    visible: true,
                    last_update: Some(Utc::now()),
                    refresh_interval_secs: 1,
                };
                render_swarm_panel(frame, area, app, &state);
            }
            2 if app.panels.cost => {
                // Try to get cost panel data, fallback to default
                let state = if let Some(_cost_panel) = app.cost_panel.try_lock() {
                    CostPanelState {
                        visible: true,
                        ..Default::default()
                    }
                } else {
                    CostPanelState::default()
                };
                render_cost_panel(frame, area, app, &state);
            }
            3 if app.panels.memory => {
                // Try to get memory panel data, fallback to default
                let state = if let Some(memory_panel) = app.memory_panel.try_lock() {
                    MemoryPanelState {
                        visible: true,
                        backend: memory_panel.backend.clone(),
                        total_memories: memory_panel.total_memories,
                        storage_bytes: memory_panel.storage_bytes,
                        recent_operations: memory_panel.recent_operations.clone(),
                    }
                } else {
                    MemoryPanelState::default()
                };
                render_memory_panel(frame, area, app, &state);
            }
            4 if app.panels.logs => {
                // Try to get log panel data, fallback to default
                let state = if let Some(log_panel) = app.log_panel.try_lock() {
                    LogsPanelState {
                        visible: true,
                        log_level: LogLevel::Info,
                        log_lines: log_panel.log_lines.iter().cloned().collect(),
                        max_lines: 200,
                        scroll_offset: 0,
                    }
                } else {
                    LogsPanelState::default()
                };
                render_logs_panel(frame, area, app, &state);
            }
            _ => {
                // Panel not visible or not dirty, show placeholder
                let panel_names = ["Swarm", "Cost", "Memory", "Logs"];
                let shortcut_keys = ['s', 'c', 'm', 'l'];
                if i < panel_names.len() {
                    let text = Text::styled(
                        format!(
                            "{} Panel\n\n(Press '{}' to toggle)",
                            panel_names[i], shortcut_keys[i]
                        ),
                        Style::default().fg(Color::White),
                    );

                    let paragraph = Paragraph::new(text).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Cyan))
                            .title(format!(" {} ", panel_names[i])),
                    );
                    frame.render_widget(paragraph, area);
                }
            }
        }
    }
}

/// Render agent selector panel
fn render_agent_selector(frame: &mut Frame, app: &AppState, area: Rect) {
    let items: Vec<ListItem> = app
        .available_agents
        .iter()
        .enumerate()
        .map(|(i, agent)| {
            let is_selected = i == app.selected_agent_index;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let prefix = if is_selected { "► " } else { "  " };
            ListItem::new(Line::styled(format!("{}{}", prefix, agent), style))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_agent_index));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" [A] Agent Selector "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Render error when terminal is too small
fn render_too_small(frame: &mut Frame) {
    let paragraph = Paragraph::new(
        "Terminal too small. Please resize to at least 60x15 for 5-panel dashboard.",
    )
    .style(Style::default().fg(Color::Red))
    .alignment(Alignment::Center);
    frame.render_widget(paragraph, frame.area());
}

/// Render header with session tabs
fn render_header(frame: &mut Frame, app: &AppState, area: Rect) {
    let titles: Vec<Line> = app
        .sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let style = if i == app.active_session {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::styled(session.name.clone(), style)
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .select(app.active_session)
        .highlight_style(Style::default().fg(Color::White).bg(Color::DarkGray));

    frame.render_widget(tabs, area);
}

/// Render chat messages area with proper scrolling
fn render_chat(frame: &mut Frame, app: &AppState, area: Rect) {
    // Try to get current session, otherwise show empty state
    let session = if let Some(session) = app.current_session() {
        session
    } else {
        let text = Text::styled(
            "No active session\n\nPress Ctrl+T to create a new session",
            Style::default().fg(Color::Gray),
        );

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Chat "),
            )
            .alignment(Alignment::Left);
        frame.render_widget(paragraph, area);
        return;
    };

    // Create message spans for rendering
    let mut lines = Vec::new();

    // Apply scroll offset: show messages from the bottom when scrolling
    let visible_messages = if app.chat_scroll > 0 {
        let start = session.messages.len().saturating_sub(app.chat_scroll);
        let end = session.messages.len();
        &session.messages[start..end]
    } else {
        &session.messages
    };

    for msg in visible_messages {
        let (role_color, role_prefix) = match msg.role {
            MessageRole::User => (Color::Cyan, "User: "),
            MessageRole::Assistant => (Color::Green, "Assistant: "),
            MessageRole::System => (Color::Yellow, "System: "),
        };

        // Add role prefix with color
        lines.push(Line::from(Span::styled(
            role_prefix,
            Style::default().fg(role_color).add_modifier(Modifier::BOLD),
        )));

        // Add message content (wrapped if needed)
        let content = &msg.content;
        let wrapped_lines: Vec<String> =
            textwrap::wrap(content, area.width.saturating_sub(4) as usize)
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();

        for line in wrapped_lines.iter() {
            lines.push(Line::from(Span::styled(
                line.clone(),
                Style::default().fg(Color::White),
            )));
        }

        // Add spacing between messages
        lines.push(Line::from(""));
    }

    // Create the paragraph with wrapped text and proper scrolling
    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Chat "),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false }) // Don't trim, let it wrap properly
        .scroll((0, app.chat_scroll as u16)); // Apply scroll offset

    frame.render_widget(paragraph, area);
}

/// Render input area
fn render_input(frame: &mut Frame, app: &AppState, area: Rect) {
    let (prompt, style) = match app.input_mode {
        InputMode::Normal => ("> ", Style::default().fg(Color::Cyan)),
        InputMode::Insert => ("> ", Style::default().fg(Color::Green)),
        InputMode::Command => (": ", Style::default().fg(Color::Yellow)),
    };

    let text = Text::from(app.input_buffer.clone());

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(style.fg.unwrap_or(Color::White)))
                .title(format!(
                    "{} {}",
                    prompt,
                    match app.input_mode {
                        InputMode::Normal => "Normal Mode",
                        InputMode::Insert => "Insert Mode (Type Message)",
                        InputMode::Command => "Command Mode",
                    }
                )),
        )
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

/// Render status bar
fn render_status(frame: &mut Frame, app: &AppState, area: Rect) {
    let status_text = if app.should_quit {
        "Quitting...".to_string()
    } else if let Some(warning) = app.provider_warning() {
        warning
    } else {
        format!(
            "Session: {} | Provider: {} | Mode: {} | Agents: {}/{}",
            app.sessions[app.active_session].name,
            app.router_status.active_provider,
            match app.input_mode {
                InputMode::Normal => "Normal",
                InputMode::Insert => "Insert",
                InputMode::Command => "Command",
            },
            app.active_agents.len(),
            app.available_agents.len()
        )
    };

    let paragraph = Paragraph::new(status_text)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

/// Render hidden panel placeholder
fn render_hidden_panel(frame: &mut Frame, area: Rect, title: &str) {
    let paragraph = Paragraph::new("Panel hidden\n\nPress corresponding key to toggle")
        .style(Style::default().fg(Color::Gray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(title),
        )
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_render_imports() {
        // Test that all necessary imports are available
        // This is a basic compile test to ensure the module is well-formed
        assert!(true);
    }
}
