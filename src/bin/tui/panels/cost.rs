//! Cost Panel - Cost tracking dashboard with sparklines
//!
//! Displays:
//! - Session cost (USD)
//! - Daily and monthly spending
//! - Budget limits and percentage used
//! - Cost history sparkline
//! - Per-model cost breakdown

use crate::bin::tui::app::AppState;
use crate::bin::tui::state::subsystems::CostDataPoint;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// State specific to the cost panel
#[derive(Debug, Clone, Default)]
pub struct CostPanelState {
    /// Whether panel is visible
    pub visible: bool,

    /// Cost history data points (for sparkline)
    pub cost_history: Vec<CostDataPoint>,

    /// Maximum history to keep
    pub max_history: usize,
}

/// Render the cost panel
pub fn render_cost_panel(frame: &mut Frame, area: Rect, app: &AppState, state: &CostPanelState) {
    if !state.visible {
        render_hidden_panel(frame, area, " Cost (press 'c' to toggle) ");
        return;
    }

    let mut text = Text::default();

    // Access cost data from the cost panel
    let cost_data = if let Ok(cost_panel) = app.cost_panel.try_lock() {
        cost_panel.clone()
    } else {
        crate::state::CostPanel {
            session_cost: 0.0,
            daily_cost: 0.0,
            daily_limit: 10.0,
            monthly_cost: 0.0,
            monthly_limit: 100.0,
            cost_history: Vec::new(),
        }
    };

    // Get current router status for display
    let provider = &app.router_status.active_provider;
    let quota_percent = app.router_status.quota_used_percent;

    // Header: Session Cost
    let cost_color = if cost_data.session_cost > cost_data.daily_limit * 0.8 {
        Color::Red
    } else if cost_data.session_cost > cost_data.daily_limit * 0.5 {
        Color::Yellow
    } else {
        Color::Green
    };

    text.push_line(Line::styled(
        format!("Session Cost: ${:.4}", cost_data.session_cost),
        Style::default().fg(cost_color).add_modifier(Modifier::BOLD),
    ));
    text.push_line(Line::styled("", Style::default()));

    // Daily spending
    let daily_color = if cost_data.daily_cost > cost_data.daily_limit * 0.8 {
        Color::Red
    } else {
        Color::Cyan
    };

    text.push_line(Line::styled(
        format!(
            "Daily: ${:.4} / ${:.4} ({:.0}%)",
            cost_data.daily_cost,
            cost_data.daily_limit,
            (cost_data.daily_cost / cost_data.daily_limit * 100.0)
        ),
        Style::default().fg(daily_color),
    ));
    text.push_line(Line::styled("", Style::default()));

    // Monthly spending
    text.push_line(Line::styled(
        format!(
            "Monthly: ${:.4} / ${:.4}",
            cost_data.monthly_cost, cost_data.monthly_limit
        ),
        Style::default().fg(Color::Blue),
    ));
    text.push_line(Line::styled("", Style::default()));

    // Daily spending
    let daily_color = if daily_cost > daily_limit * 0.8 {
        Color::Red
    } else {
        Color::Cyan
    };
    text.push_line(Line::styled(
        format!("Today:  ${:.4} / ${:.2}", daily_cost, daily_limit),
        Style::default().fg(daily_color),
    ));

    // Daily progress bar
    let daily_percent = (daily_cost / daily_limit * 100.0).min(100.0);
    let bar_width = 20;
    let filled = (daily_percent as usize * bar_width / 100).min(bar_width);
    let empty = bar_width - filled;
    text.push_line(Line::styled(
        format!(
            "  [{}{}] {:.0}%",
            "█".repeat(filled),
            "░".repeat(empty),
            daily_percent
        ),
        Style::default().fg(daily_color),
    ));
    text.push_line(Line::styled("", Style::default()));

    // Monthly spending
    let monthly_color = if monthly_cost > monthly_limit * 0.8 {
        Color::Red
    } else {
        Color::Cyan
    };
    text.push_line(Line::styled(
        format!("Month:  ${:.2} / ${:.2}", monthly_cost, monthly_limit),
        Style::default().fg(monthly_color),
    ));

    // Monthly progress bar
    let monthly_percent = (monthly_cost / monthly_limit * 100.0).min(100.0);
    let filled = (monthly_percent as usize * bar_width / 100).min(bar_width);
    let empty = bar_width - filled;
    text.push_line(Line::styled(
        format!(
            "  [{}{}] {:.0}%",
            "█".repeat(filled),
            "░".repeat(empty),
            monthly_percent
        ),
        Style::default().fg(monthly_color),
    ));
    text.push_line(Line::styled("", Style::default()));

    // Provider info
    text.push_line(Line::styled(
        format!("Provider: {}", provider),
        Style::default().fg(Color::Yellow).italic(),
    ));

    // Sparkline (placeholder - will be real data from CostTracker)
    if state.cost_history.len() > 1 {
        text.push_line(Line::styled("", Style::default()));
        text.push_line(Line::styled(
            "Cost History:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

        let sparkline = render_sparkline(&state.cost_history, area.width as usize - 4);
        text.push_line(Line::styled(sparkline, Style::default().fg(Color::Green)));
    }

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title(" Cost Tracker "),
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

/// Render a simple ASCII sparkline from cost history
fn render_sparkline(history: &[CostDataPoint], width: usize) -> String {
    if history.is_empty() {
        return "No data yet".to_string();
    }

    // Normalize to 8 height levels
    let max_cost = history
        .iter()
        .map(|d| d.request_cost_usd)
        .fold(0.0f64, f64::max);
    let min_cost = history
        .iter()
        .map(|d| d.request_cost_usd)
        .fold(f64::INFINITY, f64::min);

    let range = if max_cost - min_cost > 0.001 {
        max_cost - min_cost
    } else {
        1.0
    };

    // Sample to fit width
    let step = if history.len() > width {
        (history.len() as f64 / width as f64).ceil() as usize
    } else {
        1
    };

    let spark_chars = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    let mut result = String::new();

    for i in (0..history.len()).step_by(step) {
        let value = history[i].request_cost_usd;
        let normalized = ((value - min_cost) / range) as usize;
        let char_idx = (normalized * (spark_chars.len() - 1)).min(spark_chars.len() - 1);
        result.push_str(spark_chars[char_idx]);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_cost_panel_state_default() {
        let state = CostPanelState::default();
        assert!(!state.visible);
        assert_eq!(state.max_history, 100);
    }

    #[test]
    fn test_sparkline_rendering() {
        let history = vec![
            CostDataPoint {
                timestamp: Utc::now(),
                cumulative_cost_usd: 0.001,
                request_cost_usd: 0.001,
            },
            CostDataPoint {
                timestamp: Utc::now(),
                cumulative_cost_usd: 0.005,
                request_cost_usd: 0.004,
            },
            CostDataPoint {
                timestamp: Utc::now(),
                cumulative_cost_usd: 0.010,
                request_cost_usd: 0.005,
            },
        ];

        let sparkline = render_sparkline(&history, 20);
        assert!(!sparkline.is_empty());
        assert_ne!(sparkline, "No data yet");
    }

    #[test]
    fn test_sparkline_empty() {
        let history = vec![];
        let sparkline = render_sparkline(&history, 20);
        assert_eq!(sparkline, "No data yet");
    }
}
