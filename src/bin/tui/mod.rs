//! ZeroClaw TUI Dashboard
//!
//! Terminal user interface for ZeroClaw AI agent with multi-session support,
//! real-time agent monitoring, and 5-panel dashboard layout.

pub mod agents;
pub mod app;
pub mod benchmarks;
pub mod comprehensive_tests;
pub mod events;
pub mod panels;
pub mod sessions;
pub mod state;
pub mod ui;
pub mod ui_conditional;

pub use app::{AppState, InputMode, Message, MessageRole, Session};
pub use agents::{ZeroClawClient, format_agent_status, format_quota_percent};
pub use events::{AppEvent, map_key_event, get_help_text};
pub use state::{TuiStateChannels, StateSnapshot};
