//! Clean terminal feedback for CLI agent sessions.
//!
//! This module provides colored, formatted feedback functions that use stderr
//! for progress/status messages, keeping stdout clean for actual agent output.
//!
//! # Design
//! - All feedback goes to stderr (not stdout)
//! - Uses colored output for visual clarity
//! - Minimal and fast - no external dependencies beyond what's already available
//! - Integrates with existing VerboseObserver for event-driven feedback

use std::io::Write;
use std::time::Duration;

/// Print "thinking" status when LLM is processing.
///
/// Uses stderr to avoid polluting stdout with status messages.
pub fn print_thinking() {
    let _ = writeln!(std::io::stderr(), "{}", "🤔 Thinking...".cyan());
}

/// Print tool execution start message.
///
/// # Arguments
/// * `tool_name` - Name of the tool being executed
///
/// Example output: `[⚙️ Tool: shell] Executing...`
pub fn print_tool_start(tool_name: &str) {
    let _ = writeln!(
        std::io::stderr(),
        "{}",
        format!("[⚙️ Tool: {}] Executing...", tool_name).yellow()
    );
}

/// Print tool execution completion with timing and success status.
///
/// # Arguments
/// * `tool_name` - Name of the tool that completed
/// * `duration_ms` - Execution time in milliseconds
/// * `success` - Whether the tool succeeded
///
/// Example output:
/// - Success: `✓ Done (104ms)`
/// - Failure: `❌ Failed (23ms)`
pub fn print_tool_done(tool_name: &str, duration_ms: u64, success: bool) {
    let status = if success {
        format!("✓ Done ({}ms)", duration_ms).green().to_string()
    } else {
        format!("❌ Failed ({}ms)", duration_ms).red().to_string()
    };
    let _ = writeln!(
        std::io::stderr(),
        "{}",
        format!("[⚙️ Tool: {}] {}", tool_name, status)
    );
}

/// Print agent response output.
///
/// This prints the actual agent output (not status) to stdout.
///
/// # Arguments
/// * `text` - The agent's response text
///
/// Note: This goes to stdout, not stderr, because it's the actual output.
pub fn print_agent_response(text: &str) {
    let _ = writeln!(std::io::stdout(), "{}", text.white().bold());
}

/// Print LLM request progress message (verbose mode).
///
/// # Arguments
/// * `provider` - LLM provider name
/// * `model` - Model identifier
/// * `messages_count` - Number of messages in the request
///
/// Example output: `> LLM Request (provider=openrouter, model=claude-sonnet-4, messages=3)`
pub fn print_llm_request(provider: &str, model: &str, messages_count: usize) {
    let _ = writeln!(
        std::io::stderr(),
        "{}",
        format!(
            "> LLM Request (provider={}, model={}, messages={})",
            provider, model, messages_count
        )
        .cyan()
    );
}

/// Print LLM response received message (verbose mode).
///
/// # Arguments
/// * `provider` - LLM provider name
/// * `model` - Model identifier
/// * `duration` - Request duration
/// * `success` - Whether the request succeeded
///
/// Example output: `< LLM Response (success=true, duration_ms=1234)`
pub fn print_llm_response(provider: &str, model: &str, duration: Duration, success: bool) {
    let ms = duration.as_millis();
    let _ = writeln!(
        std::io::stderr(),
        "{}",
        format!(
            "< LLM Response (provider={}, model={}, success={}, duration_ms={})",
            provider, model, success, ms
        )
        .cyan()
    );
}

/// Print turn completion message (verbose mode).
///
/// Example output: `< Complete`
pub fn print_turn_complete() {
    let _ = writeln!(std::io::stderr(), "{}", "< Complete".cyan());
}

// Color helpers using ansi escape codes (minimal, no colored crate dependency)

trait Colorize {
    fn cyan(&self) -> String;
    fn yellow(&self) -> String;
    fn green(&self) -> String;
    fn red(&self) -> String;
    fn white(&self) -> String;
    fn bold(&self) -> String;
}

impl Colorize for str {
    fn cyan(&self) -> String {
        format!("\x1b[36m{}\x1b[0m", self)
    }

    fn yellow(&self) -> String {
        format!("\x1b[33m{}\x1b[0m", self)
    }

    fn green(&self) -> String {
        format!("\x1b[32m{}\x1b[0m", self)
    }

    fn red(&self) -> String {
        format!("\x1b[31m{}\x1b[0m", self)
    }

    fn white(&self) -> String {
        format!("\x1b[37m{}\x1b[0m", self)
    }

    fn bold(&self) -> String {
        format!("\x1b[1m{}\x1b[0m", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colorize_cyan() {
        let colored = "test".cyan();
        assert!(colored.contains("\x1b[36m"));
        assert!(colored.contains("\x1b[0m"));
        assert!(colored.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_colorize_yellow() {
        let colored = "test".yellow();
        assert!(colored.contains("\x1b[33m"));
    }

    #[test]
    fn test_colorize_green() {
        let colored = "test".green();
        assert!(colored.contains("\x1b[32m"));
    }

    #[test]
    fn test_colorize_red() {
        let colored = "test".red();
        assert!(colored.contains("\x1b[31m"));
    }

    #[test]
    fn test_colorize_bold() {
        let colored = "test".bold();
        assert!(colored.contains("\x1b[1m"));
    }
}
