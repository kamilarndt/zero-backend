//! End-to-End and Integration Tests for ZeroClaw TUI
//!
//! This test suite provides comprehensive testing of the TUI dashboard using:
//! - `ratatui::backend::TestBackend` for headless terminal testing
//! - Simulated keypress events for user interaction testing
//! - State assertions for validating application behavior
//! - Async testing patterns with timeout protection
//!
//! Test Categories:
//! 1. Panel Toggling E2E - Verify panel visibility state changes
//! 2. Mode Switching - Validate Normal/Insert/Command mode transitions
//! 3. Session Management - Test session creation, switching, and closure

use super::app::{
    AgentState, AgentStatus, AppState, ChatMessage, InputMode,
    MemoryOp, MemoryOperation, ModelStats,
};
use super::events::{map_key_event, AppEvent};
use super::ui;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use std::time::Duration;

/// Helper: Create a test AppState with default configuration
fn create_test_app() -> AppState {
    AppState::default()
}

/// Helper: Create a test AppState with populated mock data
fn create_test_app_with_data() -> AppState {
    let mut app = AppState::default();

    // Add some chat messages
    app.chat_panel.messages.push(ChatMessage {
        id: "msg1".to_string(),
        role: super::app::MessageRole::User,
        content: "Hello, ZeroClaw!".to_string(),
        model_used: None,
        timestamp: chrono::Utc::now(),
        token_count: Some(10),
    });

    app.chat_panel.messages.push(ChatMessage {
        id: "msg2".to_string(),
        role: super::app::MessageRole::Assistant,
        content: "Hello! How can I help you today?".to_string(),
        model_used: Some("glm-4.7".to_string()),
        timestamp: chrono::Utc::now(),
        token_count: Some(15),
    });

    // Add swarm agents
    app.swarm_panel.agents.push(AgentStatus {
        role: "Researcher".to_string(),
        state: AgentState::Running,
        current_task: Some("Finding documentation".to_string()),
        progress: 65,
    });

    // Add cost data
    app.cost_panel.session_cost = 0.025;
    app.cost_panel.daily_cost = 1.50;
    app.cost_panel.model_usage.push(ModelStats {
        model: "glm-4.7".to_string(),
        tokens_used: 10000,
        cost_usd: 0.02,
        percentage: 100.0,
    });

    // Add memory operations
    app.memory_panel.backend_name = "SQLite".to_string();
    app.memory_panel.vector_count = 5000;
    app.memory_panel.cache_hit_rate = 85.0;
    app.memory_panel.recent_operations.push(MemoryOperation {
        operation: MemoryOp::Recall {
            query: "test".to_string(),
            results: 3,
        },
        key: "query:test".to_string(),
        category: "search".to_string(),
        timestamp: chrono::Utc::now(),
        duration_ms: 25,
    });

    app
}

/// Helper: Create a TestBackend with specified dimensions
fn create_test_backend(width: u16, height: u16) -> TestBackend {
    TestBackend::new(width, height)
}

/// Helper: Simulate a keypress event and verify the resulting AppEvent
fn simulate_keypress(key: KeyCode, modifiers: KeyModifiers, mode: InputMode) -> Option<AppEvent> {
    let key_event = KeyEvent::new(key, modifiers);
    map_key_event(key_event, &mode)
}

/// Helper: Process a series of keypress events and update app state
async fn process_events(app: &mut AppState, events: Vec<AppEvent>) {
    for event in events {
        handle_app_event(event, app).await;
    }
}

/// Helper: Handle an AppEvent and update AppState (mirrors main.rs handle_event)
async fn handle_app_event(event: AppEvent, app: &mut AppState) {
    match event {
        AppEvent::Quit => {
            app.should_quit = true;
        }
        AppEvent::NewSession => {
            app.new_session();
        }
        AppEvent::CloseSession => {
            app.close_session();
        }
        AppEvent::NextSession => {
            app.next_session();
        }
        AppEvent::PrevSession => {
            app.prev_session();
        }
        AppEvent::ToggleInputMode => {
            app.input_mode = match app.input_mode {
                InputMode::Normal => InputMode::Insert,
                InputMode::Insert | InputMode::Command => InputMode::Normal,
            };
        }
        AppEvent::SendMessage => {
            if !app.input_buffer.is_empty() {
                app.add_user_message(app.input_buffer.clone());
                app.input_buffer.clear();
            }
        }
        AppEvent::CharInput(c) => {
            app.input_buffer.push(c);
        }
        AppEvent::Backspace => {
            app.input_buffer.pop();
        }
        AppEvent::ScrollUp => {
            app.scroll_up();
        }
        AppEvent::ScrollDown => {
            app.scroll_down();
        }
        AppEvent::ToggleSwarm => {
            app.toggle_swarm();
        }
        AppEvent::ToggleCost => {
            app.toggle_cost();
        }
        AppEvent::ToggleMemory => {
            app.toggle_memory();
        }
        AppEvent::ToggleLogs => {
            app.toggle_logs();
        }
        _ => {}
    }
}

/// Helper: Render the TUI to a TestBackend and return the buffer
fn render_to_buffer(app: &AppState, width: u16, height: u16) -> Buffer {
    let backend = create_test_backend(width, height);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            ui::render(f, app);
        })
        .unwrap();

    let backend = terminal.backend_mut();
    backend.buffer().clone()
}

/// Helper: Check if a string exists in the rendered buffer
fn buffer_contains(buffer: &Buffer, text: &str) -> bool {
    let area = buffer.area();

    for y in area.top()..area.bottom() {
        let line: String = (area.left()..area.right())
            .map(|x| buffer[(x, y)].symbol())
            .collect();

        if line.contains(text) {
            return true;
        }
    }

    false
}

/// Helper: Count occurrences of text in buffer
fn buffer_count(buffer: &Buffer, text: &str) -> usize {
    let area = buffer.area();
    let mut count = 0;

    for y in area.top()..area.bottom() {
        let line: String = (area.left()..area.right())
            .map(|x| buffer[(x, y)].symbol())
            .collect();

        if line.contains(text) {
            count += 1;
        }
    }

    count
}

// ============================================================================
// TEST SUITE 1: Panel Toggling E2E
// ============================================================================

#[test]
fn test_panel_toggling_swarm_e2e() {
    let mut app = create_test_app();

    // Initial state: Swarm panel should be visible
    assert!(app.show_swarm, "Swarm panel should be visible by default");

    // Simulate pressing 's' key in Normal mode
    let event = simulate_keypress(KeyCode::Char('s'), KeyModifiers::empty(), InputMode::Normal);
    assert_eq!(event, Some(AppEvent::ToggleSwarm));

    // Apply the event
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(event.unwrap(), &mut app).await;
        });

    // Verify: Swarm panel should now be hidden
    assert!(!app.show_swarm, "Swarm panel should be hidden after toggle");

    // Render to verify UI reflects the change
    let buffer = render_to_buffer(&app, 80, 24);

    // When swarm is hidden, "Swarm Status" should not appear in buffer
    let swarm_visible = buffer_contains(&buffer, "Swarm Status");
    assert!(!swarm_visible, "Swarm panel should not be rendered when hidden");
}

#[test]
fn test_panel_toggling_cost_e2e() {
    let mut app = create_test_app();

    // Initial state
    assert!(app.show_cost, "Cost panel should be visible by default");

    // Simulate pressing 'c' key
    let event = simulate_keypress(KeyCode::Char('c'), KeyModifiers::empty(), InputMode::Normal);
    assert_eq!(event, Some(AppEvent::ToggleCost));

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(event.unwrap(), &mut app).await;
        });

    // Verify state
    assert!(!app.show_cost, "Cost panel should be hidden after toggle");

    // Verify UI
    let buffer = render_to_buffer(&app, 80, 24);
    let cost_visible = buffer_contains(&buffer, "Cost Dashboard");
    assert!(!cost_visible, "Cost panel should not be rendered when hidden");
}

#[test]
fn test_panel_toggling_memory_e2e() {
    let mut app = create_test_app();

    // Initial state
    assert!(app.show_memory, "Memory panel should be visible by default");

    // Simulate pressing 'm' key
    let event = simulate_keypress(KeyCode::Char('m'), KeyModifiers::empty(), InputMode::Normal);
    assert_eq!(event, Some(AppEvent::ToggleMemory));

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(event.unwrap(), &mut app).await;
        });

    // Verify state
    assert!(!app.show_memory, "Memory panel should be hidden after toggle");

    // Verify UI
    let buffer = render_to_buffer(&app, 80, 24);
    let memory_visible = buffer_contains(&buffer, "Memory Inspector");
    assert!(!memory_visible, "Memory panel should not be rendered when hidden");
}

#[test]
fn test_panel_toggling_logs_e2e() {
    let mut app = create_test_app();

    // Initial state: Logs hidden by default
    assert!(!app.show_logs, "Logs panel should be hidden by default");

    // Simulate pressing 'l' key
    let event = simulate_keypress(KeyCode::Char('l'), KeyModifiers::empty(), InputMode::Normal);
    assert_eq!(event, Some(AppEvent::ToggleLogs));

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(event.unwrap(), &mut app).await;
        });

    // Verify state
    assert!(app.show_logs, "Logs panel should be visible after toggle");

    // Verify UI
    let buffer = render_to_buffer(&app, 80, 24);
    let logs_visible = buffer_contains(&buffer, "System Logs");
    assert!(logs_visible, "Logs panel should be rendered when visible");
}

#[test]
fn test_panel_toggling_all_panels_sequence() {
    let mut app = create_test_app();

    // Test toggling all panels in sequence
    let events = vec![
        KeyCode::Char('s'), // Toggle Swarm
        KeyCode::Char('c'), // Toggle Cost
        KeyCode::Char('m'), // Toggle Memory
        KeyCode::Char('l'), // Toggle Logs
    ];

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            for key in events {
                let event = simulate_keypress(key, KeyModifiers::empty(), InputMode::Normal);
                if let Some(evt) = event {
                    handle_app_event(evt, &mut app).await;
                }
            }
        });

    // Verify all panels have been toggled
    assert!(!app.show_swarm, "Swarm should be toggled off");
    assert!(!app.show_cost, "Cost should be toggled off");
    assert!(!app.show_memory, "Memory should be toggled off");
    assert!(app.show_logs, "Logs should be toggled on");
}

#[test]
fn test_panel_toggling_with_render_verification() {
    let mut app = create_test_app_with_data();

    // Render initial state
    let buffer_initial = render_to_buffer(&app, 80, 24);
    assert!(buffer_contains(&buffer_initial, "Swarm Status"));
    assert!(buffer_contains(&buffer_initial, "Cost Dashboard"));
    assert!(buffer_contains(&buffer_initial, "Memory Inspector"));

    // Toggle off all sidebar panels
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::ToggleSwarm, &mut app).await;
            handle_app_event(AppEvent::ToggleCost, &mut app).await;
            handle_app_event(AppEvent::ToggleMemory, &mut app).await;
        });

    // Render after toggling
    let buffer_after = render_to_buffer(&app, 80, 24);

    // Verify sidebar is hidden or shows empty state
    assert!(!buffer_contains(&buffer_after, "Swarm Status"));
    assert!(!buffer_contains(&buffer_after, "Cost Dashboard"));
    assert!(!buffer_contains(&buffer_after, "Memory Inspector"));

    // Chat panel should still be visible
    assert!(buffer_contains(&buffer_after, "Chat"));
}

// ============================================================================
// TEST SUITE 2: Mode Switching E2E
// ============================================================================

#[test]
fn test_mode_switching_normal_to_insert() {
    let mut app = create_test_app();

    // Initial state: Normal mode
    assert_eq!(app.input_mode, InputMode::Normal);

    // Simulate pressing 'i' to enter Insert mode
    let event = simulate_keypress(KeyCode::Char('i'), KeyModifiers::empty(), InputMode::Normal);
    assert_eq!(event, Some(AppEvent::ToggleInputMode));

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(event.unwrap(), &mut app).await;
        });

    // Verify mode changed to Insert
    assert_eq!(app.input_mode, InputMode::Insert);
}

#[test]
fn test_mode_switching_insert_to_normal() {
    let mut app = create_test_app();
    app.input_mode = InputMode::Insert;

    // Simulate pressing Esc to return to Normal mode
    let event = simulate_keypress(KeyCode::Esc, KeyModifiers::empty(), InputMode::Insert);
    assert_eq!(event, Some(AppEvent::ToggleInputMode));

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(event.unwrap(), &mut app).await;
        });

    // Verify mode changed back to Normal
    assert_eq!(app.input_mode, InputMode::Normal);
}

#[test]
fn test_mode_switching_complete_cycle() {
    let mut app = create_test_app();

    // Start in Normal mode
    assert_eq!(app.input_mode, InputMode::Normal);

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            // Normal -> Insert
            handle_app_event(AppEvent::ToggleInputMode, &mut app).await;
            assert_eq!(app.input_mode, InputMode::Insert);

            // Insert -> Normal
            handle_app_event(AppEvent::ToggleInputMode, &mut app).await;
            assert_eq!(app.input_mode, InputMode::Normal);

            // Normal -> Insert again
            handle_app_event(AppEvent::ToggleInputMode, &mut app).await;
            assert_eq!(app.input_mode, InputMode::Insert);

            // Insert -> Normal again
            handle_app_event(AppEvent::ToggleInputMode, &mut app).await;
            assert_eq!(app.input_mode, InputMode::Normal);
        });
}

#[test]
fn test_mode_switching_with_typing() {
    let mut app = create_test_app();

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            // Enter Insert mode
            handle_app_event(AppEvent::ToggleInputMode, &mut app).await;
            assert_eq!(app.input_mode, InputMode::Insert);

            // Type a message character by character
            let message = "Hello ZeroClaw";
            for ch in message.chars() {
                handle_app_event(AppEvent::CharInput(ch), &mut app).await;
            }

            // Verify buffer
            assert_eq!(app.input_buffer, message);

            // Press Esc to exit Insert mode
            handle_app_event(AppEvent::ToggleInputMode, &mut app).await;
            assert_eq!(app.input_mode, InputMode::Normal);

            // Buffer should be preserved
            assert_eq!(app.input_buffer, message);
        });
}

#[test]
fn test_mode_switching_with_backspace() {
    let mut app = create_test_app();
    app.input_mode = InputMode::Insert;

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            // Type some text
            app.input_buffer = "Hello".to_string();

            // Press backspace
            handle_app_event(AppEvent::Backspace, &mut app).await;
            assert_eq!(app.input_buffer, "Hell");

            // More backspaces
            handle_app_event(AppEvent::Backspace, &mut app).await;
            handle_app_event(AppEvent::Backspace, &mut app).await;
            assert_eq!(app.input_buffer, "He");

            // Backspace on empty buffer
            app.input_buffer.clear();
            handle_app_event(AppEvent::Backspace, &mut app).await;
            assert_eq!(app.input_buffer, "");
        });
}

#[test]
fn test_mode_switching_render_verification() {
    let mut app = create_test_app();
    app.input_buffer = "Test message".to_string();

    // Render in Normal mode
    app.input_mode = InputMode::Normal;
    let buffer_normal = render_to_buffer(&app, 80, 24);
    assert!(buffer_contains(&buffer_normal, "NORMAL"));

    // Render in Insert mode
    app.input_mode = InputMode::Insert;
    let buffer_insert = render_to_buffer(&app, 80, 24);
    assert!(buffer_contains(&buffer_insert, "INSERT"));
    assert!(buffer_contains(&buffer_insert, "Test message"));
}

#[test]
fn test_send_message_in_insert_mode() {
    let mut app = create_test_app();
    app.input_mode = InputMode::Insert;
    app.input_buffer = "Test message".to_string();

    let initial_msg_count = app.chat_panel.messages.len();

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            // Press Enter to send
            handle_app_event(AppEvent::SendMessage, &mut app).await;

            // Verify message was added
            assert_eq!(app.chat_panel.messages.len(), initial_msg_count + 1);
            assert_eq!(app.chat_panel.messages.last().unwrap().role,
                      super::app::MessageRole::User);
            assert_eq!(app.chat_panel.messages.last().unwrap().content, "Test message");

            // Buffer should be cleared
            assert_eq!(app.input_buffer, "");
        });
}

// ============================================================================
// TEST SUITE 3: Session Management E2E
// ============================================================================

#[test]
fn test_session_creation_ctrl_t() {
    let mut app = create_test_app();

    let initial_count = app.sessions.len();
    let initial_active = app.active_session;

    // Simulate Ctrl+T to create new session
    let event = simulate_keypress(KeyCode::Char('t'), KeyModifiers::CONTROL, InputMode::Normal);
    assert_eq!(event, Some(AppEvent::NewSession));

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(event.unwrap(), &mut app).await;
        });

    // Verify new session created
    assert_eq!(app.sessions.len(), initial_count + 1);
    assert_eq!(app.active_session, initial_active + 1);
    assert_eq!(app.sessions[app.active_session].name, "Session 2");
}

#[test]
fn test_session_switching_tab() {
    let mut app = create_test_app();

    // Create a second session
    app.new_session();
    assert_eq!(app.sessions.len(), 2);
    assert_eq!(app.active_session, 1);

    // Add a message to session 1
    app.add_user_message("Message in session 1".to_string());

    // Switch to session 0
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::NextSession, &mut app).await;
        });

    assert_eq!(app.active_session, 0);
    assert_eq!(app.chat_panel.messages.len(), 0);

    // Add a message to session 0
    app.add_user_message("Message in session 0".to_string());

    // Switch back to session 1
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::NextSession, &mut app).await;
        });

    assert_eq!(app.active_session, 1);
    assert_eq!(app.chat_panel.messages.len(), 1);
    assert_eq!(app.chat_panel.messages[0].content, "Message in session 1");
}

#[test]
fn test_session_switching_previous() {
    let mut app = create_test_app();

    // Create three sessions
    app.new_session();
    app.new_session();
    assert_eq!(app.sessions.len(), 3);
    assert_eq!(app.active_session, 2);

    // Press Tab (wraps to 0)
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::NextSession, &mut app).await;
        });

    assert_eq!(app.active_session, 0);

    // Press Shift+Tab (goes to 2)
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::PrevSession, &mut app).await;
        });

    assert_eq!(app.active_session, 2);
}

#[test]
fn test_session_close_with_multiple() {
    let mut app = create_test_app();

    // Create additional sessions
    app.new_session();
    app.new_session();
    assert_eq!(app.sessions.len(), 3);
    assert_eq!(app.active_session, 2);

    // Add message to current session
    app.add_user_message("Test message".to_string());
    assert_eq!(app.chat_panel.messages.len(), 1);

    // Close current session (should switch to session 1)
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::CloseSession, &mut app).await;
        });

    assert_eq!(app.sessions.len(), 2);
    assert_eq!(app.active_session, 1);
    assert_eq!(app.chat_panel.messages.len(), 0); // Session 1 has no messages
}

#[test]
fn test_session_close_last_session() {
    let mut app = create_test_app();

    // Only one session
    assert_eq!(app.sessions.len(), 1);

    // Add a message
    app.add_user_message("Test message".to_string());
    assert_eq!(app.chat_panel.messages.len(), 1);

    // Close last session (should clear messages but keep session)
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::CloseSession, &mut app).await;
        });

    // Session still exists but is cleared
    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.chat_panel.messages.len(), 0);
    assert_eq!(app.sessions[0].chat_panel.messages.len(), 0);
}

#[test]
fn test_session_isolation() {
    let mut app = create_test_app();

    // Start with default session (0)
    assert_eq!(app.active_session, 0);

    // Add message to session 0
    app.add_user_message("Session 0 message".to_string());
    assert_eq!(app.sessions[0].chat_panel.messages.len(), 1);

    // Create a new session (this switches to session 1)
    app.new_session();
    assert_eq!(app.sessions.len(), 2);
    assert_eq!(app.active_session, 1);

    // Add message to session 1
    app.add_user_message("Session 1 message".to_string());
    assert_eq!(app.sessions[1].chat_panel.messages.len(), 1);

    // Verify isolation - sessions should have different messages
    assert_eq!(app.sessions[0].chat_panel.messages.len(), 1);
    assert_eq!(app.sessions[1].chat_panel.messages.len(), 1);
    assert_ne!(
        app.sessions[0].chat_panel.messages[0].content,
        app.sessions[1].chat_panel.messages[0].content
    );
    assert_eq!(app.sessions[0].chat_panel.messages[0].content, "Session 0 message");
    assert_eq!(app.sessions[1].chat_panel.messages[0].content, "Session 1 message");
}

#[test]
fn test_session_rendering_tabs() {
    let mut app = create_test_app();

    // Create multiple sessions
    app.new_session();
    app.new_session();
    app.sessions[0].name = "Main".to_string();
    app.sessions[1].name = "Work".to_string();
    app.sessions[2].name = "Personal".to_string();

    // Set active session
    app.active_session = 1;

    // Render
    let buffer = render_to_buffer(&app, 80, 24);

    // Verify tabs are rendered
    assert!(buffer_contains(&buffer, "Main"));
    assert!(buffer_contains(&buffer, "Work"));
    assert!(buffer_contains(&buffer, "Personal"));
}

#[test]
fn test_session_switching_with_tab_key() {
    let mut app = create_test_app();

    // Simulate Tab key press
    let event = simulate_keypress(KeyCode::Tab, KeyModifiers::empty(), InputMode::Normal);
    assert_eq!(event, Some(AppEvent::NextSession));

    // Create sessions first
    app.new_session();
    app.new_session();
    assert_eq!(app.active_session, 2);

    // Tab should wrap to 0
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::NextSession, &mut app).await;
        });

    assert_eq!(app.active_session, 0);
}

#[test]
fn test_session_switching_with_shift_tab() {
    let mut app = create_test_app();

    // Simulate Shift+Tab (BackTab)
    let event = simulate_keypress(KeyCode::BackTab, KeyModifiers::SHIFT, InputMode::Normal);
    assert_eq!(event, Some(AppEvent::PrevSession));

    // Create sessions
    app.new_session();
    app.new_session();
    app.active_session = 0;

    // Shift+Tab should go to last session (2)
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::PrevSession, &mut app).await;
        });

    assert_eq!(app.active_session, 2);
}

// ============================================================================
// TEST SUITE 4: Edge Cases and Integration
// ============================================================================

#[test]
fn test_terminal_too_small() {
    let app = create_test_app();

    // Terminal smaller than minimum (60x15)
    let buffer = render_to_buffer(&app, 50, 10);

    // Should show error message
    assert!(buffer_contains(&buffer, "Terminal too small"));
}

#[test]
fn test_scrolling_chat_panel() {
    let mut app = create_test_app();

    // Add many messages
    for i in 0..20 {
        app.add_user_message(format!("Message {}", i));
    }

    let initial_offset = app.chat_panel.scroll_offset;

    // Scroll up (increases offset)
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::ScrollUp, &mut app).await;
        });

    // Offset should increase by 1
    assert_eq!(app.chat_panel.scroll_offset, initial_offset + 1);

    // Scroll down (decreases offset)
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::ScrollDown, &mut app).await;
        });

    // Should return to initial offset
    assert_eq!(app.chat_panel.scroll_offset, initial_offset);
}

#[test]
fn test_empty_input_send_message() {
    let mut app = create_test_app();
    app.input_mode = InputMode::Insert;
    app.input_buffer.clear();

    let initial_msg_count = app.chat_panel.messages.len();

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            // Try to send empty message
            handle_app_event(AppEvent::SendMessage, &mut app).await;
        });

    // No message should be added
    assert_eq!(app.chat_panel.messages.len(), initial_msg_count);
}

#[test]
fn test_command_execution_quit() {
    let mut app = create_test_app();

    assert!(!app.should_quit);

    // Execute :quit command
    let (output, should_clear) = app.execute_command("quit");

    assert!(app.should_quit);
    assert!(output.contains("Quitting"));
    assert!(should_clear);
}

#[test]
fn test_command_execution_help() {
    let mut app = create_test_app();

    let (output, should_clear) = app.execute_command("help");

    assert!(output.contains("ZeroClaw TUI"));
    assert!(should_clear);
}

#[test]
fn test_command_execution_new_session() {
    let mut app = create_test_app();
    let initial_count = app.sessions.len();

    app.execute_command("new");

    assert_eq!(app.sessions.len(), initial_count + 1);
}

#[test]
fn test_command_execution_clear() {
    let mut app = create_test_app();
    app.add_user_message("Test".to_string());
    assert_eq!(app.chat_panel.messages.len(), 1);

    app.execute_command("clear");

    assert_eq!(app.chat_panel.messages.len(), 0);
}

#[test]
fn test_unknown_command() {
    let mut app = create_test_app();

    let (output, should_clear) = app.execute_command("unknown_command");

    assert!(output.contains("Unknown command"));
    assert!(!should_clear);
}

#[test]
fn test_session_rename() {
    let mut app = create_test_app();

    app.rename_current_session("My Custom Session".to_string());

    assert_eq!(app.current_session().unwrap().name, "My Custom Session");
}

#[test]
fn test_quit_flag() {
    let mut app = create_test_app();

    // Simulate 'q' key in Normal mode
    let event = simulate_keypress(KeyCode::Char('q'), KeyModifiers::empty(), InputMode::Normal);
    assert_eq!(event, Some(AppEvent::Quit));

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(event.unwrap(), &mut app).await;
        });

    assert!(app.should_quit);
}

#[test]
fn test_ctrl_q_quit() {
    let mut app = create_test_app();

    // Simulate Ctrl+Q
    let event = simulate_keypress(KeyCode::Char('q'), KeyModifiers::CONTROL, InputMode::Normal);
    assert_eq!(event, Some(AppEvent::Quit));

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(event.unwrap(), &mut app).await;
        });

    assert!(app.should_quit);
}

// ============================================================================
// TEST SUITE 5: Async Timeout Protection
// ============================================================================

#[tokio::test]
async fn test_async_event_handling_with_timeout() {
    let mut app = create_test_app();

    // Use timeout to prevent hangs
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        handle_app_event(AppEvent::ToggleSwarm, &mut app)
    ).await;

    assert!(result.is_ok(), "Event handling should complete within timeout");
    assert!(!app.show_swarm);
}

#[tokio::test]
async fn test_async_multiple_events_sequence() {
    let mut app = create_test_app();

    let events = vec![
        AppEvent::NewSession,
        AppEvent::NextSession,
        AppEvent::PrevSession,
        AppEvent::ToggleSwarm,
        AppEvent::ToggleCost,
        AppEvent::ToggleMemory,
        AppEvent::ToggleLogs,
    ];

    let result = tokio::time::timeout(
        Duration::from_secs(1),
        process_events(&mut app, events)
    ).await;

    assert!(result.is_ok(), "Multiple events should be processed within timeout");
    assert_eq!(app.sessions.len(), 2);
    assert!(app.show_logs);
    assert!(!app.show_swarm);
}

// ============================================================================
// TEST SUITE 6: State Persistence
// ============================================================================

#[test]
fn test_state_persistence_across_session_switch() {
    let mut app = create_test_app();

    // Add message to session 0
    app.active_session = 0;
    app.add_user_message("Session 0 message".to_string());

    // Create and switch to session 1
    app.new_session();
    app.add_user_message("Session 1 message".to_string());

    // Switch back to session 0
    app.next_session();

    // Verify state was preserved
    assert_eq!(app.active_session, 0);
    assert_eq!(app.chat_panel.messages.len(), 1);
    assert_eq!(app.chat_panel.messages[0].content, "Session 0 message");
}

#[test]
fn test_panel_state_persistence() {
    let mut app = create_test_app();

    // Toggle panels
    app.toggle_swarm();
    app.toggle_logs();

    assert!(!app.show_swarm);
    assert!(app.show_logs);

    // Create new session (should preserve panel state)
    app.new_session();

    // Panel visibility should be consistent across sessions
    assert!(!app.show_swarm);
    assert!(app.show_logs);
}

// ============================================================================
// TEST SUITE 7: UI Rendering Integration
// ============================================================================

#[test]
fn test_ui_rendering_with_all_panels_visible() {
    let app = create_test_app_with_data();

    let buffer = render_to_buffer(&app, 80, 24);

    // Verify all main components are rendered
    assert!(buffer_contains(&buffer, "Sessions"));
    assert!(buffer_contains(&buffer, "Chat"));
    assert!(buffer_contains(&buffer, "Swarm Status"));
    assert!(buffer_contains(&buffer, "Cost Dashboard"));
    assert!(buffer_contains(&buffer, "Memory Inspector"));
}

#[test]
fn test_ui_rendering_with_messages() {
    let app = create_test_app_with_data();

    let buffer = render_to_buffer(&app, 80, 24);

    // Verify messages are rendered
    assert!(buffer_contains(&buffer, "Hello, ZeroClaw!"));
    assert!(buffer_contains(&buffer, "Hello! How can I help"));
    assert!(buffer_contains(&buffer, "USER"));
    assert!(buffer_contains(&buffer, "ASSISTANT"));
}

#[test]
fn test_ui_rendering_status_bar() {
    let app = create_test_app_with_data();

    let buffer = render_to_buffer(&app, 80, 24);

    // Verify status bar content
    assert!(buffer_contains(&buffer, "ZeroClaw"));
    assert!(buffer_contains(&buffer, "Sessions:"));
    assert!(buffer_contains(&buffer, "Msgs:"));
}

#[test]
fn test_ui_rendering_input_box() {
    let mut app = create_test_app();
    app.input_buffer = "Test input".to_string();

    // Test Normal mode
    app.input_mode = InputMode::Normal;
    let buffer_normal = render_to_buffer(&app, 80, 24);
    assert!(buffer_contains(&buffer_normal, "NORMAL"));

    // Test Insert mode
    app.input_mode = InputMode::Insert;
    let buffer_insert = render_to_buffer(&app, 80, 24);
    assert!(buffer_contains(&buffer_insert, "INSERT"));
    assert!(buffer_contains(&buffer_insert, "Test input"));
}
