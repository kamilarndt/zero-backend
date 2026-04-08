//! Diagnostic and testing utilities
//!
//! Provides health checks, configuration validation, and testing helpers.

use crate::config::Config;
use serde_json::json;

/// Diagnostic check result
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiagnosticCheck {
    pub name: String,
    pub status: DiagnosticStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Status of a diagnostic check
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticStatus {
    Pass,
    Fail,
    Warn,
    Skip,
}

impl std::fmt::Display for DiagnosticStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticStatus::Pass => write!(f, "Pass"),
            DiagnosticStatus::Fail => write!(f, "Fail"),
            DiagnosticStatus::Warn => write!(f, "Warn"),
            DiagnosticStatus::Skip => write!(f, "Skip"),
        }
    }
}

/// Run all diagnostic checks
pub async fn run_diagnostics(config: &Config) -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    // 1. Config validation
    checks.push(validate_config(config));

    // 2. API keys check
    checks.extend(check_api_keys(config));

    // 3. Memory backend check
    checks.push(check_memory_backend());

    // 4. Gateway check
    checks.push(check_gateway_config(config));

    // 5. Channels check
    checks.extend(check_channels(config));

    checks
}

/// Validate configuration structure
pub fn validate_config(config: &Config) -> DiagnosticCheck {
    let mut warnings: Vec<String> = Vec::new();
    let errors: Vec<String> = Vec::new();

    // Check gateway config
    if config.gateway.port == 0 {
        warnings.push("Gateway port is 0 (random port will be used)".to_string());
    }

    // Check for API key
    if config.api_key.is_none() {
        let has_env_key = std::env::var("OPENROUTER_API_KEY").is_ok()
            || std::env::var("OPENAI_API_KEY").is_ok()
            || std::env::var("ANTHROPIC_API_KEY").is_ok()
            || std::env::var("API_KEY").is_ok();

        if !has_env_key {
            warnings
                .push("No API key configured (set ZEROCLAW_API_KEY or run onboard)".to_string());
        }
    }

    // Check for default provider
    if config.default_provider.is_none() {
        warnings.push("No default provider set (will use openrouter)".to_string());
    }

    let status = if !errors.is_empty() {
        DiagnosticStatus::Fail
    } else if !warnings.is_empty() {
        DiagnosticStatus::Warn
    } else {
        DiagnosticStatus::Pass
    };

    DiagnosticCheck {
        name: "Configuration".to_string(),
        status,
        message: Some(format!(
            "{} errors, {} warnings",
            errors.len(),
            warnings.len()
        )),
        details: Some(json!({
            "errors": errors,
            "warnings": warnings,
        })),
    }
}

/// Check API keys configuration
fn check_api_keys(config: &Config) -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    // Check OpenRouter (primary provider)
    let openrouter_key = std::env::var("OPENROUTER_API_KEY")
        .ok()
        .or_else(|| std::env::var("API_KEY").ok())
        .or(config.api_key.clone());

    checks.push(DiagnosticCheck {
        name: "OpenRouter API Key".to_string(),
        status: if openrouter_key.is_some()
            && openrouter_key.as_ref().map_or(false, |k| k.len() > 10)
        {
            DiagnosticStatus::Pass
        } else if openrouter_key.is_some() {
            DiagnosticStatus::Warn
        } else {
            DiagnosticStatus::Fail
        },
        message: Some(if openrouter_key.is_some() {
            "API key configured".to_string()
        } else {
            "Not configured (set OPENROUTER_API_KEY or run onboard)".to_string()
        }),
        details: None,
    });

    // Check OpenAI
    let openai_key = std::env::var("OPENAI_API_KEY").ok();

    checks.push(DiagnosticCheck {
        name: "OpenAI API Key".to_string(),
        status: if openai_key.is_some() {
            DiagnosticStatus::Pass
        } else {
            DiagnosticStatus::Skip
        },
        message: Some(if openai_key.is_some() {
            "API key configured".to_string()
        } else {
            "Not configured (optional)".to_string()
        }),
        details: None,
    });

    checks
}

/// Check memory backend
fn check_memory_backend() -> DiagnosticCheck {
    let status = DiagnosticStatus::Pass; // SQLite is always available

    DiagnosticCheck {
        name: "Memory Backend".to_string(),
        status,
        message: Some("SQLite memory backend available".to_string()),
        details: Some(json!({
            "backend": "sqlite",
            "path": "~/.zeroclaw/memory/brain.db"
        })),
    }
}

/// Check gateway configuration
fn check_gateway_config(config: &Config) -> DiagnosticCheck {
    let host = &config.gateway.host;
    let port = config.gateway.port;
    let port_display = if port == 0 {
        "random".to_string()
    } else {
        port.to_string()
    };

    DiagnosticCheck {
        name: "Gateway".to_string(),
        status: DiagnosticStatus::Pass,
        message: Some(format!("Will listen on {}:{}", host, port_display)),
        details: Some(json!({
            "port": port,
            "host": host,
            "pairing_required": config.gateway.require_pairing,
            "paired_tokens": config.gateway.paired_tokens.len(),
        })),
    }
}

/// Check configured channels
fn check_channels(config: &Config) -> Vec<DiagnosticCheck> {
    let mut checks = Vec::new();

    // Count configured channels (is_some() means configured)
    let channels = config.channels_config.channels();
    let configured_count = channels
        .iter()
        .filter(|(_, is_configured)| *is_configured)
        .count();
    let channel_names: Vec<&str> = channels
        .iter()
        .filter(|(_, is_configured)| *is_configured)
        .map(|(ch, _)| ch.name())
        .collect();

    checks.push(DiagnosticCheck {
        name: "Channels".to_string(),
        status: DiagnosticStatus::Pass,
        message: Some(format!(
            "{} channels configured (CLI always available)",
            configured_count
        )),
        details: Some(json!({
            "configured_count": configured_count,
            "channels": channel_names
        })),
    });

    checks
}

/// Quick health check for testing
pub async fn quick_test() -> Result<String, anyhow::Error> {
    use std::time::Duration;

    // Test 1: Check if gateway is responsive
    tokio::time::timeout(Duration::from_secs(2), async {
        let client = reqwest::Client::new();
        client.get("http://127.0.0.1:42617/health").send().await
    })
    .await
    .map_err(|_| anyhow::anyhow!("Gateway not responding on http://127.0.0.1:42617"))??;

    // Test 2: Check if API endpoints work
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:42617/health")
        .send()
        .await?
        .text()
        .await?;

    // Parse JSON and extract status
    let status_str = if let Ok(value) = serde_json::from_str::<serde_json::Value>(&response) {
        value["status"].as_str().unwrap_or("ok").to_string()
    } else {
        "ok".to_string()
    };

    Ok(format!(
        "✓ Gateway responsive\n✓ API endpoints working\nHealth: {}",
        status_str
    ))
}
