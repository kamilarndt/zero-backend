//! Swarm Panel - Real-time agent swarm status
//!
//! Displays:
//! - Active agents with their roles and models
//! - Current task and progress for each agent
//! - Agent status (running/idle/done/failed)
//! - Swarm throughput metrics
//! - Tasks completed counter

use crate::app::AgentState;
use crate::state::subsystems::{AgentInfo, AgentStatus};
use crate::state::AppState;
use ratatui::{
    layout::{Alignment, Rect},
    style::Stylize,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// State specific to the swarm panel
#[derive(Debug, Clone, Default)]
pub struct SwarmPanelState {
    /// Whether panel is visible
    pub visible: bool,

    /// Last update timestamp
    pub last_update: Option<chrono::DateTime<chrono::Utc>>,

    /// Auto-refresh interval (seconds)
    pub refresh_interval_secs: u64,
}

/// Render the swarm panel
pub fn render_swarm_panel(frame: &mut Frame, area: Rect, app: &AppState, state: &SwarmPanelState) {
    if !state.visible {
        render_hidden_panel(frame, area, " Swarm (press 's' to toggle) ");
        return;
    }

    let mut text = Text::default();

    // Access agents from the swarm panel
    let swarm_agents = if let Some(swarm) = app.swarm_panel.try_lock() {
        swarm.agents.clone()
    } else {
        Vec::new()
    };

    if swarm_agents.is_empty() {
        text = Text::styled(
            "No active agents.\n\nAgent swarm will appear here when tasks are running.\n\nPress Ctrl+A to spawn a test agent.",
            Style::default().fg(Color::DarkGray).italic(),
        );
    } else {
        // Header stats
        text.push_line(Line::styled(
            format!("Active Agents: {}", swarm_agents.len()),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        text.push_line(Line::styled("", Style::default()));

        // List each agent
        for agent in &swarm_agents {
            let (status_color, status_symbol) = match agent.status {
                crate::state::subsystems::AgentStatus::Idle => (Color::DarkGray, "○"),
                crate::state::subsystems::AgentStatus::Running => (Color::Yellow, "●"),
                crate::state::subsystems::AgentStatus::Done => (Color::Green, "✓"),
                crate::state::subsystems::AgentStatus::Failed => (Color::Red, "✗"),
            };

            // Agent name and status
            text.push_line(Line::styled(
                format!("{} {} ({})", status_symbol, agent.name, agent.model),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ));

            // Progress bar
            let progress_width = 12;
            let filled = (agent.progress as usize * progress_width / 100).min(progress_width);
            let empty = progress_width - filled;
            let progress_bar = format!(
                "  [{}{}] {}%",
                "█".repeat(filled),
                "░".repeat(empty),
                agent.progress
            );

            text.push_line(Line::styled(
                progress_bar,
                Style::default().fg(status_color),
            ));

            // Agent current task
            if let Some(task) = &agent.current_task {
                text.push_line(Line::styled(
                    format!("  Task: {}", task),
                    Style::default().fg(Color::Gray),
                ));
            }

            // Agent role
            text.push_line(Line::styled(
                format!("  Role: {}", agent.role),
                Style::default().fg(Color::DarkGray).italic(),
            ));

            text.push_line(Line::styled("", Style::default()));
        }
    }

    // Footer info
    if let Some(last_update) = state.last_update {
        let age = chrono::Utc::now() - last_update;
        text.push_line(Line::styled(
            format!("Last update: {}s ago", age.num_seconds()),
            Style::default().fg(Color::DarkGray).italic(),
        ));
    }

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Agent Swarm "),
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

/// Convert app AgentState to subsystem AgentStatus
impl From<AgentState> for AgentStatus {
    fn from(state: AgentState) -> Self {
        match state {
            AgentState::Idle => AgentStatus::Idle,
            AgentState::Running => AgentStatus::Running,
            AgentState::Done => AgentStatus::Done,
            AgentState::Failed => AgentStatus::Failed,
        }
    }
}

/// Convert app AgentStatus to subsystem AgentInfo
impl From<&crate::app::AgentStatus> for AgentInfo {
    fn from(app_agent: &crate::app::AgentStatus) -> Self {
        AgentInfo {
            id: app_agent.id.clone(),
            name: app_agent.name.clone(),
            role: "executor".to_string(), // Default role for now
            model: app_agent.model.clone(),
            current_task: None, // TODO: Integrate with A2A task tracking
            progress: app_agent.progress,
            status: AgentStatus::from(app_agent.status),
            created_at: chrono::Utc::now(), // TODO: Track creation time
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swarm_panel_state_default() {
        let state = SwarmPanelState::default();
        assert!(!state.visible); // Hidden by default in 5-panel layout
        assert_eq!(state.refresh_interval_secs, 1);
    }

    #[test]
    fn test_agent_status_conversion() {
        let app_status = crate::app::AgentStatus {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Test".to_string(),
            model: "gpt-4".to_string(),
            progress: 75,
            status: AgentState::Running,
        };

        let agent_info = AgentInfo::from(&app_status);
        assert_eq!(agent_info.progress, 75);
        assert_eq!(agent_info.status, AgentStatus::Running);
    }
}
