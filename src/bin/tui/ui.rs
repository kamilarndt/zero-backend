//! UI rendering for ZeroClaw TUI
//!
//! This module provides the rendering logic using ratatui for drawing
//! the terminal interface with the 5-panel dashboard layout.

use super::state::{
    AgentState, AppState, InputMode,
    Message, MessageRole,
};
use chrono::Utc;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Stylize,
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

/// Render the main UI with 5-panel layout
pub fn render(frame: &mut Frame, app: &AppState) {
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
            Constraint::Length(3), // Header/tabs
            Constraint::Min(12),   // Main area (at least 12 lines for 5-panel)
            Constraint::Length(3), // Input
            Constraint::Length(1), // Status bar
        ])
        .split(size);

    // Main area: horizontal split (50% chat, 50% panels)
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

    // Render each section
    render_header(frame, app, chunks[0]);
    render_chat(frame, app, main_chunks[0]);
    render_panels(frame, app, panels_chunks.to_vec());
    render_input(frame, app, chunks[2]);
    render_status(frame, app, chunks[3]);
}

/// Render the 4-panel dashboard (swarm, cost, memory, logs)
fn render_panels(frame: &mut Frame, app: &AppState, areas: Vec<Rect>) {
    // Render each panel based on visibility
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
                let state = if let Some(cost_panel) = app.cost_panel.try_lock() {
                    CostPanelState {
                        visible: true,
                        cost_history: cost_panel.cost_history.clone(),
                        max_history: 100,
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
                // Panel not visible, show placeholder
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

/// Render chat messages area
fn render_chat(frame: &mut Frame, app: &AppState, area: Rect) {
    static EMPTY_MESSAGES: Vec<Message> = Vec::new();
    let messages = app
        .current_session()
        .map(|s| &s.messages)
        .unwrap_or(&EMPTY_MESSAGES);

    let mut text = Text::default();

    // Apply scroll offset
    let visible_messages: Vec<_> = messages.iter().rev().skip(app.chat_scroll).collect();

    for msg in visible_messages.iter().rev() {
        let (prefix, style) = match msg.role {
            MessageRole::User => ("User: ", Style::default().fg(Color::Green)),
            MessageRole::Assistant => ("Assistant: ", Style::default().fg(Color::Blue)),
            MessageRole::System => ("System: ", Style::default().fg(Color::Yellow)),
        };

        // Add role prefix
        text.push_line(Line::styled(prefix, style));

        // Add message content (word-wrapped)
        for line in textwrap::wrap(&msg.content, area.width as usize - 4) {
            text.push_line(Line::styled(
                line.to_string(),
                Style::default().fg(Color::White),
            ));
        }

        // Add timestamp
        let time_str = msg.timestamp.format("%H:%M:%S").to_string();
        text.push_line(Line::styled(
            format!("  [{}]", time_str),
            Style::default().fg(Color::DarkGray),
        ));

        text.push_line(Line::styled("", Style::default()));
    }

    // Empty state
    if messages.is_empty() {
        text = Text::styled(
            "No messages yet. Press 'i' to start typing.",
            Style::default().fg(Color::DarkGray).italic(),
        );
    }

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Chat "),
        )
        .wrap(Wrap { trim: false })
        .scroll((0, 0));

    frame.render_widget(paragraph, area);
}

/// Render agents panel
fn render_agents(frame: &mut Frame, app: &AppState, area: Rect) {
    let mut text = Text::default();

    if app.active_agents.is_empty() {
        text = Text::styled(
            "No active agents.\n\nPress Ctrl+A to spawn.",
            Style::default().fg(Color::DarkGray).italic(),
        );
    } else {
        for agent in &app.active_agents {
            let status_color = match agent.status {
                AgentState::Idle => Color::DarkGray,
                AgentState::Running => Color::Yellow,
                AgentState::Done => Color::Green,
                AgentState::Failed => Color::Red,
            };

            let status_symbol = match agent.status {
                AgentState::Idle => "○",
                AgentState::Running => "●",
                AgentState::Done => "✓",
                AgentState::Failed => "✗",
            };

            // Progress bar
            let progress_width = 10;
            let filled = (agent.progress as usize * progress_width / 100).min(progress_width);
            let empty = progress_width - filled;
            let progress_bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

            text.push_line(Line::styled(
                format!("{} {}", status_symbol, agent.name),
                Style::default().fg(status_color),
            ));
            text.push_line(Line::styled(
                format!("  {} {}", progress_bar, agent.model),
                Style::default().fg(Color::Gray),
            ));
            text.push_line(Line::styled("", Style::default()));
        }
    }

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Agents "),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Render input box
fn render_input(frame: &mut Frame, app: &AppState, area: Rect) {
    let input_style = if app.input_mode == InputMode::Insert {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let mode_indicator = match app.input_mode {
        InputMode::Normal => Span::styled("NORMAL", Style::default().fg(Color::Green)),
        InputMode::Insert => Span::styled("INSERT", Style::default().fg(Color::Cyan)),
        InputMode::Command => Span::styled("COMMAND", Style::default().fg(Color::Yellow)),
    };

    let prefix = if app.input_mode == InputMode::Insert {
        "> "
    } else if app.input_mode == InputMode::Command {
        ":"
    } else {
        ""
    };

    let mut input_text = Text::default();
    input_text.push_line(Line::from(vec![
        mode_indicator,
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(prefix, input_style),
        Span::styled(&app.input_buffer, input_style),
    ]));

    // Help hint in normal mode
    if app.input_mode == InputMode::Normal && app.input_buffer.is_empty() {
        input_text.push_line(Line::styled(
            "Press '?' for help | 'i' to type",
            Style::default().fg(Color::DarkGray).italic(),
        ));
    }

    let paragraph = Paragraph::new(input_text)
        .block(Block::default().borders(Borders::ALL).border_style(
            if app.input_mode == InputMode::Insert {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Render status bar
fn render_status(frame: &mut Frame, app: &AppState, area: Rect) {
    let provider = &app.router_status.active_provider;
    let quota = app.router_status.quota_used_percent;
    let selected_agent = app.selected_agent();

    let status_text = vec![
        Span::styled(
            format!("Agent: {}", selected_agent),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Provider: {}", provider),
            Style::default().fg(Color::Green),
        ),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Quota: {:.0}%", quota),
            Style::default().fg(if quota > 80.0 {
                Color::Red
            } else {
                Color::Green
            }),
        ),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Sessions: {}", app.sessions.len()),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!(
                "Messages: {}",
                app.current_session().map(|s| s.messages.len()).unwrap_or(0)
            ),
            Style::default().fg(Color::Magenta),
        ),
    ];

    let paragraph = Paragraph::new(Line::from(status_text))
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));

    frame.render_widget(paragraph, area);
}

/// Render help overlay
pub fn render_help(frame: &mut Frame) {
    let area = frame.area();

    let help_text = super::events::get_help_text();

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Help - Press Esc or q to close "),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
