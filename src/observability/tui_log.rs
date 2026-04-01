//! TUI log capture layer for tracing
//!
//! This module provides a tracing layer that captures log events
//! in a circular buffer for consumption by the TUI dashboard.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock as StdOnceLock};
use tracing::Level;
use tracing_subscriber::Layer;

/// Maximum log lines to keep in memory
const MAX_LOG_LINES: usize = 200;

/// A single log entry captured by the TUI layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiLogEntry {
    /// Log level
    pub level: String,
    /// Log message
    pub message: String,
    /// Module path (if available)
    pub module: Option<String>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl TuiLogEntry {
    fn from_event(event: &tracing::Event<'_>) -> Option<Self> {
        // Get level from metadata
        let level_str = match event.metadata().level() {
            &Level::ERROR => "ERROR".to_string(),
            &Level::WARN => "WARN".to_string(),
            &Level::INFO => "INFO".to_string(),
            &Level::DEBUG => "DEBUG".to_string(),
            &Level::TRACE => "TRACE".to_string(),
            // If level() returns None, use INFO as default
            _ => "INFO".to_string(),
        };

        // Extract module from metadata
        let module = event.metadata().module_path().map(|s| s.to_string());

        // Try to get message from event
        let mut message = String::new();
        let mut visitor = TuiLogVisitor(&mut message);
        event.record(&mut visitor);

        // If message is still empty, provide a default
        if message.is_empty() {
            message = format!("{}", event.metadata().name());
        }

        Some(TuiLogEntry {
            level: level_str,
            message,
            module,
            timestamp: chrono::Utc::now(),
        })
    }
}

/// Visitor for extracting the message field from tracing events
struct TuiLogVisitor<'a>(&'a mut String);

impl<'a> tracing::field::Visit for TuiLogVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            use std::fmt::Write;
            write!(self.0, "{:?}", value).ok();
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0.push_str(value);
        }
    }

    fn record_i64(&mut self, _field: &tracing::field::Field, _value: i64) {}
    fn record_u64(&mut self, _field: &tracing::field::Field, _value: u64) {}
    fn record_bool(&mut self, _field: &tracing::field::Field, _value: bool) {}
    fn record_f64(&mut self, _field: &tracing::field::Field, _value: f64) {}
}

/// Shared log buffer for TUI consumption
#[derive(Clone)]
pub struct TuiLogBuffer {
    inner: Arc<Mutex<std::collections::VecDeque<TuiLogEntry>>>,
}

impl TuiLogBuffer {
    /// Create a new log buffer
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(std::collections::VecDeque::with_capacity(MAX_LOG_LINES))),
        }
    }

    /// Get all current log entries
    pub fn get_logs(&self) -> Vec<TuiLogEntry> {
        self.inner.lock().iter().cloned().collect()
    }

    /// Add a log entry to the buffer
    pub fn add_entry(&self, entry: TuiLogEntry) {
        let mut buf = self.inner.lock();
        if buf.len() >= MAX_LOG_LINES {
            buf.pop_front();
        }
        buf.push_back(entry);
    }

    /// Clear all log entries
    pub fn clear(&self) {
        self.inner.lock().clear();
    }
}

impl Default for TuiLogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Global TUI log buffer instance
static TUI_LOG_BUFFER: StdOnceLock<TuiLogBuffer> = StdOnceLock::new();

/// Initialize the global TUI log buffer
fn init_tui_log_buffer() -> &'static TuiLogBuffer {
    TUI_LOG_BUFFER.get_or_init(|| TuiLogBuffer::new())
}

/// Get the global TUI log buffer
pub fn get_tui_log_buffer() -> TuiLogBuffer {
    init_tui_log_buffer().clone()
}

/// Tracing layer that captures logs for TUI consumption
pub struct TuiLogLayer {
    _buffer: TuiLogBuffer,
}

impl TuiLogLayer {
    /// Create a new TUI log layer
    pub fn new() -> Self {
        // Ensure the buffer is initialized
        let _ = init_tui_log_buffer();
        Self {
            _buffer: get_tui_log_buffer(),
        }
    }
}

impl Default for TuiLogLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for TuiLogLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Filter out TRACE level to reduce noise
        if matches!(event.metadata().level(), &Level::TRACE) {
            return;
        }

        if let Some(entry) = TuiLogEntry::from_event(event) {
            // Filter out very noisy modules
            if let Some(ref module) = entry.module {
                // Skip hyper and tokio internal logs
                if module.contains("hyper::")
                    || module.contains("tokio::")
                    || module.contains("h2::")
                    || module.contains("want::")
                {
                    return;
                }
            }

            get_tui_log_buffer().add_entry(entry);
        }
    }

    fn enabled(
        &self,
        metadata: &tracing::Metadata<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        // Only capture INFO and above (ignore DEBUG/TRACE to reduce noise)
        metadata.level() <= &Level::INFO
    }
}
