//! Agent monitoring for ZeroClaw TUI
//!
//! This module provides integration with ZeroClaw's SubAgentManager
//! to display active subagents and their progress.

use super::app::{AgentState, AgentStatus};
use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

/// HTTP client for communicating with ZeroClaw gateway
pub struct ZeroClawClient {
    base_url: String,
    http: Client,
}

impl ZeroClawClient {
    /// Create a new ZeroClaw client
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            http: Client::new(),
        }
    }

    /// Create with default localhost URL
    pub fn localhost() -> Self {
        Self::new("http://127.0.0.1:42617".to_string())
    }

    /// Send a message to the current session
    pub async fn send_message(&self, session_id: &str, message: &str) -> Result<String> {
        #[derive(serde::Serialize)]
        struct Request {
            session_id: String,
            content: String,
        }

        let resp = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&Request {
                session_id: session_id.to_string(),
                content: message.to_string(),
            })
            .send()
            .await?;

        #[derive(Deserialize)]
        struct Response {
            response: Option<String>,
            error: Option<String>,
        }

        let data: Response = resp.json().await?;

        if let Some(error) = data.error {
            anyhow::bail!("API error: {}", error);
        }

        data.response.ok_or_else(|| anyhow::anyhow!("No response from API"))
    }

    /// List active subagents
    pub async fn list_agents(&self) -> Result<Vec<AgentStatus>> {
        #[derive(Deserialize)]
        struct AgentResponse {
            id: String,
            name: String,
            model: String,
            progress: u8,
            status: String,
        }

        let resp = self
            .http
            .get(format!("{}/api/agents/active", self.base_url))
            .send()
            .await?;

        let agents: Vec<AgentResponse> = resp.json().await?;

        Ok(agents
            .into_iter()
            .map(|a| AgentStatus {
                id: a.id,
                name: a.name,
                model: a.model,
                progress: a.progress,
                status: match a.status.as_str() {
                    "idle" => AgentState::Idle,
                    "running" => AgentState::Running,
                    "done" => AgentState::Done,
                    "failed" => AgentState::Failed,
                    _ => AgentState::Idle,
                },
            })
            .collect())
    }

    /// Get router status
    pub async fn get_router_status(&self) -> Result<RouterStatusData> {
        #[derive(Deserialize)]
        struct StatusResponse {
            active_provider: String,
            quota_used_percent: f32,
            fallback_active: bool,
        }

        let resp = self
            .http
            .get(format!("{}/api/routing/status", self.base_url))
            .send()
            .await?;

        let data: StatusResponse = resp.json().await?;

        Ok(RouterStatusData {
            active_provider: data.active_provider,
            quota_used_percent: data.quota_used_percent,
            fallback_active: data.fallback_active,
        })
    }
}

/// Router status from API
#[derive(Debug, Clone, Deserialize)]
pub struct RouterStatusData {
    pub active_provider: String,
    pub quota_used_percent: f32,
    pub fallback_active: bool,
}

/// Format agent status for display
pub fn format_agent_status(agent: &AgentStatus) -> String {
    let status_symbol = match agent.status {
        AgentState::Idle => "○",
        AgentState::Running => "●",
        AgentState::Done => "✓",
        AgentState::Failed => "✗",
    };

    let progress_bar = if agent.progress > 0 {
        let filled = agent.progress as usize / 10;
        let empty = 10 - filled;
        format!(" [{}{}]", "█".repeat(filled), "░".repeat(empty))
    } else {
        String::new()
    };

    format!(
        "{} {}{} - {}",
        status_symbol,
        agent.name,
        progress_bar,
        agent.model
    )
}

/// Format quota percentage with color indicator
pub fn format_quota_percent(percent: f32) -> String {
    let indicator = if percent < 50.0 {
        "●"
    } else if percent < 80.0 {
        "◐"
    } else {
        "◑"
    };

    format!("{} {:.0}%", indicator, percent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_agent_status() {
        let agent = AgentStatus {
            id: "test".to_string(),
            name: "TestAgent".to_string(),
            model: "gpt-4".to_string(),
            progress: 65,
            status: AgentState::Running,
        };

        let formatted = format_agent_status(&agent);
        assert!(formatted.contains("TestAgent"));
        assert!(formatted.contains("gpt-4"));
        assert!(formatted.contains("●")); // Running symbol
    }

    #[test]
    fn test_format_quota_percent() {
        assert_eq!(format_quota_percent(25.0), "● 25%");
        assert_eq!(format_quota_percent(60.0), "◐ 60%");
        assert_eq!(format_quota_percent(90.0), "◑ 90%");
    }

    #[test]
    fn test_zeroclaw_client_localhost() {
        let client = ZeroClawClient::localhost();
        assert_eq!(client.base_url, "http://127.0.0.1:42617");
    }
}
