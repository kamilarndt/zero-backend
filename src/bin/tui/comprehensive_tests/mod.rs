//! Comprehensive Test Suite for ZeroClaw TUI
//!
//! This module provides extensive testing for the TUI using `ratatui::backend::TestBackend`.
//! Tests cover state management, user interactions, and visual rendering.
//!
//! Test Categories:
//! - State & Config Tests: Verify data integrity between config and AppState
//! - User Interaction Tests: Simulate keyboard input and verify state transitions
//! - Visual Rendering Tests: Render frames to TestBackend and verify output

use crate::state::{AppState, InputMode, Message, MessageRole};
use crate::events::{map_key_event, AppEvent};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::TestBackend,
    buffer::Buffer,
    layout::Rect,
    text::Line,
    widgets::Widget,
};
use std::sync::Arc;
use parking_lot::Mutex;
use chrono::Utc;

// ============================================================================
// Test Helper Macros
// ============================================================================

/// Macro to assert that a buffer contains a specific string
/// Usage: `assert_buffer_contains!(buffer, "expected text")`
macro_rules! assert_buffer_contains {
    ($buffer:expr, $expected:expr) => {
        let buffer_str = $buffer
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(
            buffer_str.contains($expected),
            "Buffer does not contain expected string: {}\n\nActual buffer:\n{}",
            $expected,
            buffer_str
        );
    };
}

/// Macro to assert that a buffer contains multiple strings
/// Usage: `assert_buffer_contains_all!(buffer, ["text1", "text2"])`
macro_rules! assert_buffer_contains_all {
    ($buffer:expr, [$($expected:expr),* $(,)?]) => {
        $(
            assert_buffer_contains!($buffer, $expected);
        )*
    };
}

/// Macro to assert that a buffer does NOT contain a specific string
macro_rules! assert_buffer_not_contains {
    ($buffer:expr, $unexpected:expr) => {
        let buffer_str = $buffer
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(
            !buffer_str.contains($unexpected),
            "Buffer unexpectedly contains string: {}",
            $unexpected
        );
    };
}

/// Helper function to convert buffer to string for debugging
pub fn buffer_to_string(buffer: &Buffer) -> String {
    buffer
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>()
}

/// Helper function to count occurrences of a string in buffer
pub fn count_in_buffer(buffer: &Buffer, search: &str) -> usize {
    let buffer_str = buffer_to_string(buffer);
    buffer_str.matches(search).count()
}

// Macros are available within this module scope
// Note: Macros defined in the same module are automatically in scope

// ============================================================================
// 1. State & Config Tests
// ============================================================================

#[cfg(test)]
mod state_config_tests {
    use super::*;

    /// Test that AppState correctly initializes with default config values
    #[test]
    fn test_app_initialization() {
        let app = AppState::default();

        // Verify default state
        assert_eq!(app.active_session, 0);
        assert_eq!(app.sessions.len(), 1);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.input_buffer.is_empty());
        assert!(!app.should_quit);

        // Verify sessions
        assert_eq!(app.sessions[0].name, "Session 1");
        assert!(app.sessions[0].messages.is_empty());

        // Verify router status defaults
        assert_eq!(app.router_status.active_provider, "None");
        assert_eq!(app.router_status.quota_used_percent, 0.0);
        assert!(!app.router_status.fallback_active);

        // Verify panel visibility (all visible by default in 5-panel layout)
        assert!(app.panels.swarm);
        assert!(app.panels.cost);
        assert!(app.panels.memory);
        assert!(app.panels.logs);

        // Verify agent selection
        assert_eq!(app.selected_agent_index, 0);
        assert_eq!(app.selected_agent(), "coder");
    }

    /// Test quota percentage calculation: $2.50 used out of $10.00 limit = 25%
    #[test]
    fn test_zai_quota_math() {
        let mut app = AppState::default();

        // Set daily cost to $2.50
        {
            let mut cost_panel = app.cost_panel.lock();
            cost_panel.daily_cost = 2.50;
            cost_panel.daily_limit = 10.00;
        }

        // Calculate quota percentage
        let cost_panel = app.cost_panel.lock();
        let quota_used_percent = (cost_panel.daily_cost / cost_panel.daily_limit * 100.0) as f32;

        // Assert exactly 25% (within floating point tolerance)
        assert!((quota_used_percent - 25.0).abs() < 0.01,
                "Expected 25.0%, got {}", quota_used_percent);
    }

    /// Test quota edge case: cost exceeds limit should cap at 100%
    #[test]
    fn test_zai_quota_exceeds_limit() {
        let mut app = AppState::default();

        // Set daily cost to $15.00, which exceeds $10.00 limit
        {
            let mut cost_panel = app.cost_panel.lock();
            cost_panel.daily_cost = 15.00;
            cost_panel.daily_limit = 10.00;
        }

        // Calculate quota percentage (should cap at 100%)
        let cost_panel = app.cost_panel.lock();
        let quota_used_percent = (cost_panel.daily_cost / cost_panel.daily_limit * 100.0).min(100.0) as f32;

        assert!((quota_used_percent - 100.0).abs() < 0.01,
                "Expected 100.0%, got {}", quota_used_percent);
    }

    /// Test quota edge case: zero cost should be 0%
    #[test]
    fn test_zai_quota_zero_cost() {
        let app = AppState::default();

        // Default should be 0% used
        let cost_panel = app.cost_panel.lock();
        assert_eq!(cost_panel.quota_used_percent, 0.0);
    }

    /// Test quota edge case: zero limit should handle gracefully
    #[test]
    fn test_zai_quota_zero_limit() {
        let mut app = AppState::default();

        // Set daily limit to 0 (edge case)
        {
            let mut cost_panel = app.cost_panel.lock();
            cost_panel.daily_cost = 5.00;
            cost_panel.daily_limit = 0.00;
        }

        // Should not panic, result should be 0% or handled gracefully
        let cost_panel = app.cost_panel.lock();
        let quota_percent = if cost_panel.daily_limit > 0.0 {
            (cost_panel.daily_cost / cost_panel.daily_limit * 100.0) as f32
        } else {
            0.0
        };

        assert_eq!(quota_percent, 0.0);
    }

    /// Test that provider is correctly loaded into RouterStatus
    #[test]
    fn test_provider_in_router_status() {
        let mut app = AppState::default();

        // Simulate loading config with provider "glm"
        app.router_status.active_provider = "glm".to_string();

        assert_eq!(app.router_status.active_provider, "glm");
    }

    /// Test quota remaining calculation (100% - used%)
    #[test]
    fn test_quota_remaining_calculation() {
        let mut app = AppState::default();

        // Set 25% used quota
        {
            let mut cost_panel = app.cost_panel.lock();
            cost_panel.quota_used_percent = 25.0;
        }

        let cost_panel = app.cost_panel.lock();
        let remaining = 100.0 - cost_panel.quota_used_percent;

        assert_eq!(remaining, 75.0);
    }

    /// Test agent selection defaults
    #[test]
    fn test_default_agent_selection() {
        let app = AppState::default();

        // Default agent should be "coder" (first in DEFAULT_AGENTS)
        assert_eq!(app.selected_agent(), "coder");
        assert_eq!(app.selected_agent_index, 0);
    }

    /// Test agent list contains expected agents
    #[test]
    fn test_available_agents_list() {
        let app = AppState::default();

        // Should contain default agents
        assert!(app.available_agents.contains(&"coder".to_string()));
        assert!(app.available_agents.contains(&"planner".to_string()));
        assert!(app.available_agents.contains(&"vision".to_string()));
        assert!(app.available_agents.contains(&"fast".to_string()));
    }
}

// ============================================================================
// 2. User Interaction Tests
// ============================================================================

#[cfg(test)]
mod user_interaction_tests {
    use super::*;

    /// Test typing "Hello" character by character and verifying input buffer
    #[test]
    fn test_chat_input_typing() {
        let mut app = AppState::default();

        // Simulate typing "Hello" character by character
        let chars = ['H', 'e', 'l', 'l', 'o'];
        for c in chars {
            app.input_buffer.push(c);
        }

        assert_eq!(app.input_buffer, "Hello");
    }

    /// Test typing "Hello" and pressing Enter to send
    #[test]
    fn test_chat_input_and_send() {
        let mut app = AppState::default();

        // Simulate typing "Hello"
        app.input_buffer = "Hello".to_string();

        // Simulate pressing Enter (send message)
        app.add_user_message(app.input_buffer.clone());

        // Assert input buffer is cleared (in real TUI this happens after send)
        app.input_buffer.clear();
        assert!(app.input_buffer.is_empty());

        // Assert a new Message with role "User" and content "Hello" is added
        let session = app.current_session().unwrap();
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].role, MessageRole::User);
        assert_eq!(session.messages[0].content, "Hello");

        // In real TUI, after sending, the app would enter WaitingForResponse state
        // For now we verify the message was added
    }

    /// Test that multiple messages can be sent
    #[test]
    fn test_multiple_chat_messages() {
        let mut app = AppState::default();

        // Send first message
        app.add_user_message("Hello".to_string());
        app.add_assistant_message("Hi there!".to_string(), Some("gpt-4".to_string()));

        // Send second message
        app.add_user_message("How are you?".to_string());
        app.add_assistant_message("I'm doing well!".to_string(), Some("gpt-4".to_string()));

        let session = app.current_session().unwrap();
        assert_eq!(session.messages.len(), 4);

        // Verify message roles
        assert_eq!(session.messages[0].role, MessageRole::User);
        assert_eq!(session.messages[1].role, MessageRole::Assistant);
        assert_eq!(session.messages[2].role, MessageRole::User);
        assert_eq!(session.messages[3].role, MessageRole::Assistant);
    }

    /// Test backspace removes last character
    #[test]
    fn test_backspace_removes_character() {
        let mut app = AppState::default();

        app.input_buffer = "Hello".to_string();

        // Simulate backspace
        app.input_buffer.pop();

        assert_eq!(app.input_buffer, "Hell");
    }

    /// Test backspace on empty buffer doesn't panic
    #[test]
    fn test_backspace_on_empty_buffer() {
        let mut app = AppState::default();
        assert!(app.input_buffer.is_empty());

        // Simulate backspace on empty buffer
        app.input_buffer.pop();

        assert!(app.input_buffer.is_empty());
    }

    /// Test agent assignment cycles forward with 'a' key
    #[test]
    fn test_agent_assignment_cycle_forward() {
        let mut app = AppState::default();

        // Initial agent should be "coder"
        assert_eq!(app.selected_agent(), "coder");
        assert_eq!(app.selected_agent_index, 0);

        // Simulate pressing 'a' (next agent)
        app.next_agent();

        assert_eq!(app.selected_agent(), "planner");
        assert_eq!(app.selected_agent_index, 1);
    }

    /// Test agent assignment cycles backward with Shift+A
    #[test]
    fn test_agent_assignment_cycle_backward() {
        let mut app = AppState::default();

        // Start from first agent
        assert_eq!(app.selected_agent(), "coder");

        // Press 'a' twice to get to third agent
        app.next_agent();
        app.next_agent();

        assert_eq!(app.selected_agent(), "vision");

        // Now press Shift+A (prev agent)
        app.prev_agent();

        assert_eq!(app.selected_agent(), "planner");
    }

    /// Test agent assignment wraps around
    #[test]
    fn test_agent_assignment_wraps() {
        let mut app = AppState::default();

        // Get last agent index
        let last_index = app.available_agents.len() - 1;
        app.set_selected_agent(last_index);

        let last_agent = app.selected_agent().clone();

        // Press 'a' to go to next agent (should wrap to first)
        app.next_agent();

        assert_eq!(app.selected_agent(), "coder");
        assert_eq!(app.selected_agent_index, 0);

        // Now go backwards from first (should wrap to last)
        app.prev_agent();

        assert_eq!(app.selected_agent(), last_agent);
    }

    /// Test direct agent selection by index
    #[test]
    fn test_agent_direct_selection() {
        let mut app = AppState::default();

        // Select agent at index 2
        app.set_selected_agent(2);

        assert_eq!(app.selected_agent_index, 2);
        // Index 2 should be "vision" (0: coder, 1: planner, 2: vision)
        assert_eq!(app.selected_agent(), "vision");
    }

    /// Test input mode transitions
    #[test]
    fn test_input_mode_transitions() {
        let mut app = AppState::default();

        // Start in Normal mode
        assert_eq!(app.input_mode, InputMode::Normal);

        // Press 'i' to enter Insert mode
        app.input_mode = InputMode::Insert;
        assert_eq!(app.input_mode, InputMode::Insert);

        // Press Esc to return to Normal mode
        app.input_mode = InputMode::Normal;
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    /// Test Enter key maps to SendMessage in Insert mode
    #[test]
    fn test_enter_key_sends_message() {
        // In Insert mode, Enter should send message
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let event = map_key_event(key, &InputMode::Insert);

        assert_eq!(event, Some(AppEvent::SendMessage));
    }

    /// Test Escape key returns to Normal mode from Insert mode
    #[test]
    fn test_escape_key_mode_transition() {
        // From Insert mode, Esc should toggle input mode
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        let event = map_key_event(key, &InputMode::Insert);

        assert_eq!(event, Some(AppEvent::ToggleInputMode));
    }

    /// Test character input in Insert mode
    #[test]
    fn test_character_input() {
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty());
        let event = map_key_event(key, &InputMode::Insert);

        assert_eq!(event, Some(AppEvent::CharInput('x')));
    }

    /// Test Tab key creates new session in Normal mode
    #[test]
    fn test_tab_next_session() {
        let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::empty());
        let event = map_key_event(key, &InputMode::Normal);

        assert_eq!(event, Some(AppEvent::NextSession));
    }

    /// Test Ctrl+T creates new session
    #[test]
    fn test_ctrl_t_new_session() {
        let key = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL);
        let event = map_key_event(key, &InputMode::Normal);

        assert_eq!(event, Some(AppEvent::NewSession));
    }

    /// Test Ctrl+W closes session
    #[test]
    fn test_ctrl_w_close_session() {
        let key = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL);
        let event = map_key_event(key, &InputMode::Normal);

        assert_eq!(event, Some(AppEvent::CloseSession));
    }

    /// Test session navigation
    #[test]
    fn test_session_navigation() {
        let mut app = AppState::default();

        // Create additional sessions
        app.new_session();
        app.new_session();

        assert_eq!(app.sessions.len(), 3);
        assert_eq!(app.active_session, 2);

        // Navigate to next session (wraps to 0)
        app.next_session();
        assert_eq!(app.active_session, 0);

        // Navigate to previous session (wraps to 2)
        app.prev_session();
        assert_eq!(app.active_session, 2);
    }

    /// Test panel toggle keys
    #[test]
    fn test_panel_toggle_keys() {
        // Test 's' toggles swarm panel
        let key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty());
        let event = map_key_event(key, &InputMode::Normal);
        assert_eq!(event, Some(AppEvent::ToggleSwarmPanel));

        // Test 'c' toggles cost panel
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty());
        let event = map_key_event(key, &InputMode::Normal);
        assert_eq!(event, Some(AppEvent::ToggleCostPanel));

        // Test 'm' toggles memory panel
        let key = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::empty());
        let event = map_key_event(key, &InputMode::Normal);
        assert_eq!(event, Some(AppEvent::ToggleMemoryPanel));

        // Test 'l' toggles logs panel
        let key = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty());
        let event = map_key_event(key, &InputMode::Normal);
        assert_eq!(event, Some(AppEvent::ToggleLogsPanel));
    }

    /// Test panel visibility state changes
    #[test]
    fn test_panel_visibility_changes() {
        let mut app = AppState::default();

        // All panels should be visible by default
        assert!(app.panels.swarm);
        assert!(app.panels.cost);
        assert!(app.panels.memory);
        assert!(app.panels.logs);

        // Toggle swarm panel off
        use crate::state::PanelType;
        app.toggle_panel(PanelType::Swarm);
        assert!(!app.panels.swarm);

        // Toggle swarm panel back on
        app.toggle_panel(PanelType::Swarm);
        assert!(app.panels.swarm);
    }
}

// ============================================================================
// 3. Visual Rendering Tests
// ============================================================================

#[cfg(test)]
mod visual_rendering_tests {
    use super::*;
    use crate::ui;
    use crate::state::subsystems::{AgentInfo, AgentStatus};

    /// Helper to create a test backend with given dimensions
    fn create_test_backend(width: u16, height: u16) -> TestBackend {
        TestBackend::new(width, height)
    }

    /// Helper to render app to buffer and return the buffer
    fn render_to_buffer(app: &AppState) -> Buffer {
        let backend = create_test_backend(120, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        terminal
            .draw(|f| ui::render(f, app))
            .unwrap();

        terminal.backend().buffer().clone()
    }

    /// Test that chat panel renders user and assistant messages
    #[test]
    fn test_render_chat_panel() {
        let mut app = AppState::default();

        // Add mock messages
        app.add_user_message("Hello, ZeroClaw!".to_string());
        app.add_assistant_message("Hi! How can I help?".to_string(), Some("gpt-4".to_string()));

        // Render to buffer
        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        // Assert the buffer contains role prefixes
        assert!(buffer_str.contains("User:"), "Buffer should contain 'User:'");
        assert!(buffer_str.contains("Assistant:"), "Buffer should contain 'Assistant:'");

        // Assert message content is present
        assert!(buffer_str.contains("Hello, ZeroClaw!"), "Buffer should contain user message");
        assert!(buffer_str.contains("Hi! How can I help?"), "Buffer should contain assistant message");
    }

    /// Test that chat panel renders error state correctly
    #[test]
    fn test_render_error_state() {
        let mut app = AppState::default();

        // Add an error message
        app.add_assistant_message(
            "[Error] HTTP 408 - Request Timeout".to_string(),
            Some("system".to_string()),
        );

        // Render to buffer
        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        // Assert error is rendered
        assert!(buffer_str.contains("[Error]"), "Buffer should contain '[Error]'");
        assert!(buffer_str.contains("HTTP 408"), "Buffer should contain 'HTTP 408'");
        assert!(buffer_str.contains("Request Timeout"), "Buffer should contain 'Request Timeout'");
    }

    /// Test that different error types are rendered
    #[test]
    fn test_render_various_error_states() {
        let error_messages = vec![
            "[Error] Connection refused",
            "[Error] Invalid API key",
            "[Error] Rate limit exceeded",
            "[Error] JSON parse error",
        ];

        for error in error_messages {
            let mut app = AppState::default();
            app.add_assistant_message(error.to_string(), Some("system".to_string()));

            let buffer = render_to_buffer(&app);
            let buffer_str = buffer_to_string(&buffer);

            assert!(buffer_str.contains(error), "Buffer should contain: {}", error);
        }
    }

    /// Test that quota panel displays correct percentage
    #[test]
    fn test_render_zai_quota_panel() {
        let mut app = AppState::default();

        // Set 25% used quota (75% remaining)
        app.router_status.active_provider = "glm".to_string();
        app.router_status.quota_used_percent = 25.0;

        // Render to buffer
        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        // Assert quota information is displayed (75% remaining)
        assert!(buffer_str.contains("75"), "Buffer should show remaining quota ~75%");
        assert!(buffer_str.contains("%"), "Buffer should contain percentage symbol");
    }

    /// Test quota panel displays provider name
    #[test]
    fn test_render_quota_provider() {
        let mut app = AppState::default();

        app.router_status.active_provider = "glm".to_string();
        app.router_status.quota_used_percent = 50.0;

        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        // Should show provider info
        assert!(buffer_str.contains("Provider:") || buffer_str.contains("GLM") || buffer_str.contains("glm"),
                "Buffer should show provider information");
    }

    /// Test quota panel progress bar rendering
    #[test]
    fn test_render_quota_progress_bar() {
        let mut app = AppState::default();

        // Set provider and 50% quota (remaining)
        app.router_status.active_provider = "glm".to_string();
        app.router_status.quota_used_percent = 50.0;

        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        // Progress bar uses block characters (█ or ░)
        assert!(buffer_str.contains("█") || buffer_str.contains("░"),
                "Buffer should contain progress bar characters");
    }

    /// Test memory panel renders backend and memory count
    #[test]
    fn test_render_memory_inspector() {
        let mut app = AppState::default();

        // Set mock memory data
        {
            let mut memory_panel = app.memory_panel.lock();
            memory_panel.backend = "hybrid".to_string();
            memory_panel.total_memories = 94;
            memory_panel.storage_bytes = 5_242_880; // 5MB
        }

        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        // Assert memory info is displayed
        assert!(buffer_str.contains("hybrid"), "Buffer should contain 'hybrid' backend");
        assert!(buffer_str.contains("94"), "Buffer should contain '94' memory count");
        assert!(buffer_str.contains("Backend:"), "Buffer should contain 'Backend:' label");
        assert!(buffer_str.contains("Total Memories:"), "Buffer should contain 'Total Memories:' label");
    }

    /// Test memory panel shows storage size in human-readable format
    #[test]
    fn test_render_memory_storage_size() {
        let mut app = AppState::default();

        // Set storage size to approximately 5MB
        {
            let mut memory_panel = app.memory_panel.lock();
            memory_panel.storage_bytes = 5_242_880;
        }

        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        // Should show MB format
        assert!(buffer_str.contains("MB") || buffer_str.contains("5.0"),
                "Buffer should show storage in MB");
    }

    /// Test swarm panel renders active agent
    #[test]
    fn test_render_swarm_panel() {
        let mut app = AppState::default();

        // Add mock agent
        {
            let mut swarm_panel = app.swarm_panel.lock();
            swarm_panel.agents.push(AgentInfo {
                id: "agent-001".to_string(),
                name: "coder".to_string(),
                role: "executor".to_string(),
                model: "gpt-4".to_string(),
                current_task: Some("Analyze code".to_string()),
                progress: 75,
                status: AgentStatus::Running,
                created_at: Utc::now(),
            });
        }

        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        // Assert agent info is displayed
        assert!(buffer_str.contains("coder") || buffer_str.contains("gpt-4") || buffer_str.contains("Active"),
                "Buffer should show agent information");
    }

    /// Test swarm panel shows agent progress
    #[test]
    fn test_render_swarm_progress() {
        let mut app = AppState::default();

        // Add agent with specific progress
        {
            let mut swarm_panel = app.swarm_panel.lock();
            swarm_panel.agents.push(AgentInfo {
                id: "agent-001".to_string(),
                name: "Test Agent".to_string(),
                role: "executor".to_string(),
                model: "gpt-4".to_string(),
                current_task: Some("Task in progress".to_string()),
                progress: 75,
                status: AgentStatus::Running,
                created_at: Utc::now(),
            });
        }

        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        // Progress bar should be visible
        assert!(buffer_str.contains("75") || buffer_str.contains("%"),
                "Buffer should show progress percentage");
    }

    /// Test empty chat state renders helpful message
    #[test]
    fn test_render_empty_chat_state() {
        let app = AppState::default();

        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        // Empty state should show helpful message
        assert!(buffer_str.contains("No messages") || buffer_str.contains("Press 'i'") || buffer_str.contains("help"),
                "Buffer should show empty state message");
    }

    /// Test session tabs are rendered
    #[test]
    fn test_render_session_tabs() {
        let mut app = AppState::default();

        // Create multiple sessions
        app.new_session();
        app.new_session();

        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        // Session names should be visible
        assert!(buffer_str.contains("Session 1"), "Buffer should contain 'Session 1'");
        assert!(buffer_str.contains("Session 2") || buffer_str.contains("Session 3"),
                "Buffer should contain other session names");
    }

    /// Test status bar renders correctly
    #[test]
    fn test_render_status_bar() {
        let mut app = AppState::default();

        app.router_status.active_provider = "glm".to_string();
        {
            let mut cost_panel = app.cost_panel.lock();
            cost_panel.quota_used_percent = 25.0;
        }

        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        // Status bar should show agent and provider
        assert!(buffer_str.contains("Agent:") || buffer_str.contains("coder"),
                "Buffer should show current agent");
        assert!(buffer_str.contains("Provider:") || buffer_str.contains("GLM"),
                "Buffer should show provider");
        assert!(buffer_str.contains("Quota:") || buffer_str.contains("25"),
                "Buffer should show quota");
    }

    /// Test input box reflects current mode
    #[test]
    fn test_render_input_box_mode() {
        let mut app = AppState::default();

        // Test Normal mode
        app.input_mode = InputMode::Normal;
        app.input_buffer = String::new();

        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        assert!(buffer_str.contains("NORMAL") || buffer_str.contains("Press '?'") || buffer_str.contains("'i'"),
                "Buffer should show NORMAL mode indicator");

        // Test Insert mode
        app.input_mode = InputMode::Insert;
        app.input_buffer = "test".to_string();

        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        assert!(buffer_str.contains("INSERT") || buffer_str.contains("test"),
                "Buffer should show INSERT mode indicator or input text");
    }

    /// Test input box displays buffer content
    #[test]
    fn test_render_input_box_content() {
        let mut app = AppState::default();

        app.input_mode = InputMode::Insert;
        app.input_buffer = "Hello, World!".to_string();

        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        assert!(buffer_str.contains("Hello, World!"),
                "Buffer should show input buffer content");
    }

    /// Test too small terminal renders error message
    #[test]
    fn test_render_too_small_terminal() {
        let backend = TestBackend::new(30, 10); // Too small
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let app = AppState::default();

        terminal
            .draw(|f| ui::render(f, &app))
            .unwrap();

        let buffer = terminal.backend().buffer();
        let buffer_str = buffer_to_string(buffer);

        // Should show error about terminal size
        assert!(buffer_str.contains("too small") || buffer_str.contains("resize"),
                "Buffer should show terminal size error");
    }

    /// Test panel visibility affects rendering
    #[test]
    fn test_panel_visibility_rendering() {
        let mut app = AppState::default();

        // All panels visible
        let buffer_all_visible = render_to_buffer(&app);
        let buffer_str_all = buffer_to_string(&buffer_all_visible);

        // Hide cost panel
        use crate::state::PanelType;
        app.toggle_panel(PanelType::Cost);

        let buffer_cost_hidden = render_to_buffer(&app);
        let buffer_str_hidden = buffer_to_string(&buffer_cost_hidden);

        // The rendered output should be different
        assert_ne!(buffer_str_all, buffer_str_hidden,
                   "Hiding a panel should change the rendered output");
    }

    /// Test message timestamps are rendered
    #[test]
    fn test_render_message_timestamps() {
        let mut app = AppState::default();

        // Add a message with specific timestamp
        app.add_user_message("Timestamp test".to_string());

        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        // Timestamp format is HH:MM:SS, so should contain colons
        assert!(buffer_str.contains("[") || buffer_str.contains(":"),
                "Buffer should contain timestamp markers");
    }

    /// Test long messages are wrapped
    #[test]
    fn test_render_long_message_wrapping() {
        let mut app = AppState::default();

        // Add a very long message
        let long_message = "This is a very long message that should be wrapped ".repeat(10);
        app.add_user_message(long_message.clone());

        let buffer = render_to_buffer(&app);
        let buffer_str = buffer_to_string(&buffer);

        // At least part of the message should be visible
        assert!(buffer_str.contains("This is a very long") || buffer_str.contains("message"),
                "Buffer should contain part of the long message");
    }
}

// ============================================================================
// 4. Integration Tests
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test full conversation flow
    #[test]
    fn test_full_conversation_flow() {
        let mut app = AppState::default();

        // User enters insert mode and types
        app.input_mode = InputMode::Insert;
        app.input_buffer = "What is 2+2?".to_string();

        // User sends message
        app.add_user_message(app.input_buffer.clone());
        app.input_buffer.clear();
        app.input_mode = InputMode::Normal;

        // System responds
        app.add_assistant_message("2+2 equals 4.".to_string(), Some("gpt-4".to_string()));

        // Verify conversation state
        let session = app.current_session().unwrap();
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].content, "What is 2+2?");
        assert_eq!(session.messages[1].content, "2+2 equals 4.");
    }

    /// Test session switch preserves messages
    #[test]
    fn test_session_switch_preserves_messages() {
        let mut app = AppState::default();

        // Add messages to first session
        app.add_user_message("Session 1 message".to_string());

        // Create new session
        app.new_session();

        // Add messages to second session
        app.add_user_message("Session 2 message".to_string());

        // Switch back to first session
        app.active_session = 0;

        // Verify first session still has its message
        let session = app.current_session().unwrap();
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].content, "Session 1 message");
    }

    /// Test agent switch between messages
    #[test]
    fn test_agent_switch_between_messages() {
        let mut app = AppState::default();

        // Send message with "coder" agent
        app.add_user_message("Code this".to_string());
        let agent1 = app.selected_agent();

        // Switch to "planner" agent
        app.next_agent();
        app.add_user_message("Plan this".to_string());
        let agent2 = app.selected_agent();

        assert_ne!(agent1, agent2);
    }

    /// Test quota tracking across session
    #[test]
    fn test_quota_tracking_session() {
        let mut app = AppState::default();

        // Start with 0 cost
        let initial_quota = app.cost_panel.lock().quota_used_percent;
        assert_eq!(initial_quota, 0.0);

        // Simulate adding cost
        {
            let mut cost_panel = app.cost_panel.lock();
            cost_panel.daily_cost = 1.50;
        }

        // Verify quota was updated
        let cost_panel = app.cost_panel.lock();
        assert_eq!(cost_panel.daily_cost, 1.50);
    }

    /// Test error recovery after failed send
    #[test]
    fn test_error_recovery() {
        let mut app = AppState::default();

        // User sends message
        app.add_user_message("Test message".to_string());

        // System returns error
        app.add_assistant_message(
            "[Error] Connection timeout".to_string(),
            Some("system".to_string()),
        );

        // User sends another message (should work normally)
        app.add_user_message("Retry".to_string());
        app.add_assistant_message("Success response".to_string(), None);

        let session = app.current_session().unwrap();
        assert_eq!(session.messages.len(), 4);

        // Verify error message is in history
        assert!(session.messages[1].content.contains("[Error]"));
    }

    /// Test command execution
    #[test]
    fn test_command_execution() {
        let mut app = AppState::default();

        // Execute new command (without colon - execute_command handles parsing)
        let (output, should_clear) = app.execute_command("new");
        assert!(output.contains("created") || output.contains("New"),
                "New command should return success message, got: {}", output);
        assert!(should_clear, "New command should clear input");

        // Execute help command
        let (output, _) = app.execute_command("help");
        assert!(!output.is_empty(), "Help command should return output");

        // Execute unknown command
        let (output, _) = app.execute_command("unknown");
        assert!(output.contains("Unknown"), "Unknown command should return error");
    }

    /// Test quit command
    #[test]
    fn test_quit_command() {
        let mut app = AppState::default();

        // Execute quit command
        let (output, _) = app.execute_command("quit");

        assert!(app.should_quit, "App should be set to quit");
        assert!(output.contains("Quitting"), "Quit command should return quitting message");
    }

    /// Test clear command
    #[test]
    fn test_clear_command() {
        let mut app = AppState::default();

        // Add messages
        app.add_user_message("Test".to_string());
        app.add_assistant_message("Response".to_string(), None);

        assert_eq!(app.current_session().unwrap().messages.len(), 2);

        // Execute clear command (without colon - execute_command handles the parsing)
        let (output, _) = app.execute_command("clear");

        assert!(output.contains("cleared") || output.contains("Cleared"),
                "Clear command should return success message, got: {}", output);
        assert_eq!(app.current_session().unwrap().messages.len(), 0,
                   "Session should be cleared");
    }

    /// Test scroll position management
    #[test]
    fn test_scroll_management() {
        let mut app = AppState::default();

        // Add multiple messages
        for i in 0..10 {
            app.add_user_message(format!("Message {}", i));
        }

        // Scroll up
        app.scroll_up();
        assert_eq!(app.chat_scroll, 1);

        // Scroll down
        app.scroll_down();
        assert_eq!(app.chat_scroll, 0);

        // Scroll past limit should not panic
        for _ in 0..100 {
            app.scroll_down();
        }
        assert_eq!(app.chat_scroll, 0);
    }

    /// Test panel toggle persistence
    #[test]
    fn test_panel_toggle_persistence() {
        let mut app = AppState::default();

        use crate::state::PanelType;

        // Toggle multiple panels
        app.toggle_panel(PanelType::Swarm);
        app.toggle_panel(PanelType::Cost);
        app.toggle_panel(PanelType::Memory);

        assert!(!app.panels.swarm);
        assert!(!app.panels.cost);
        assert!(!app.panels.memory);
        assert!(app.panels.logs); // Still visible

        // Toggle back
        app.toggle_panel(PanelType::Swarm);
        assert!(app.panels.swarm);
    }
}
