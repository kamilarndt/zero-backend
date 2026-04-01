//! Logging initialization with file appender and console filtering.
//!
//! This module provides a centralized logging setup that:
//! - Writes all logs (INFO+) to a rotating file in `~/.zeroclaw/logs/`
//! - Filters console output to WARN/ERROR by default (INFO+ in verbose mode)
//! - Respects the RUST_LOG environment variable for fine-grained control

use std::path::PathBuf;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry, Layer};

use super::tui_log::TuiLogLayer;

/// Initialize logging with file appender and filtered console output.
///
/// # Arguments
/// * `verbose` - If true, show INFO level logs on console. Otherwise, only WARN/ERROR.
///
/// # Behavior
/// - All logs (INFO+) are written to `~/.zeroclaw/logs/zeroclaw.log` with daily rotation
/// - Console shows WARN/ERROR by default, INFO+ when `verbose=true`
/// - Respects `RUST_LOG` environment variable for directive-based filtering
/// - Safe to call multiple times: will only initialize once, subsequent calls are no-ops
///
/// # Note
/// This function uses `~/.zeroclaw/logs/` to be consistent with the rest of ZeroClaw's configuration.
pub fn init_logging(verbose: bool) {
    // Set up log file directory in ~/.zeroclaw/logs/
    // Use BaseDirs to get home directory, then append .zeroclaw/logs
    let log_dir = directories::BaseDirs::new()
        .map(|base| base.home_dir().join(".zeroclaw/logs"))
        .unwrap_or_else(|| PathBuf::from(".zeroclaw/logs"));

    // Create log directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!("Failed to create log directory {:?}: {}", log_dir, e);
        // Fall back to console-only logging
        let filter = build_console_filter(verbose);
        let subscriber = fmt::Subscriber::builder()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .finish();
        // Use try_init to handle case where subscriber is already set
        let _ = tracing::subscriber::set_global_default(subscriber);
        return;
    }

    // Set up rolling file appender (daily rotation)
    // Note: RollingFileAppender::new() panics if it cannot create the file
    let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "zeroclaw.log");

    // Build separate filters for console and file
    // Console: WARN/ERROR only (clean UX)
    // File: INFO+ for full observability
    let console_filter = build_console_filter(verbose);
    let file_filter = build_file_filter();

    // Set up console layer (filtered to stderr, WARN/ERROR only in normal mode)
    let console_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_filter(console_filter);

    // Set up file layer (all logs, full detail - always INFO+)
    let file_layer = fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_filter(file_filter);

    // Combine and initialize with proper type
    // Use try_init() to safely handle multiple calls (no-op if already initialized)
    let _ = Registry::default()
        .with(console_layer)
        .with(file_layer)
        .with(TuiLogLayer::new())
        .try_init();
}

/// Build the console filter based on verbose mode and RUST_LOG.
///
/// - If RUST_LOG is set, use it (overrides everything)
/// - Otherwise: `warn` in normal mode, `info` in verbose mode
fn build_console_filter(verbose: bool) -> EnvFilter {
    // Try to read RUST_LOG from environment
    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        // User has set RUST_LOG - respect their directive
        if let Ok(filter) = EnvFilter::try_new(&rust_log) {
            return filter;
        }
        // If RUST_LOG is invalid, fall through to defaults
    }

    // Default: warn in normal mode, info in verbose
    let level = if verbose { "info" } else { "warn" };
    EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("warn"))
}

/// Build the file filter - always INFO+ for full observability.
///
/// The file always captures INFO+ logs regardless of console verbosity.
/// If RUST_LOG is set, it affects both console and file.
fn build_file_filter() -> EnvFilter {
    // If RUST_LOG is set, use it (affects both console and file)
    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        if let Ok(filter) = EnvFilter::try_new(&rust_log) {
            return filter;
        }
    }

    // File always gets INFO+ for full observability
    EnvFilter::try_new("info").unwrap_or_else(|_| EnvFilter::new("info"))
}
