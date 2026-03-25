//! ZeroClaw TUI Dashboard
//!
//! Terminal user interface for ZeroClaw AI agent with multi-session support,
//! real-time agent monitoring, and chat functionality.

pub mod app;
pub mod agents;
pub mod events;
pub mod sessions;
pub mod ui;

pub use app::{AppState, InputMode, Message, MessageRole, Session};
pub use agents::{ZeroClawClient, format_agent_status, format_quota_percent};
pub use events::{AppEvent, map_key_event, get_help_text};
