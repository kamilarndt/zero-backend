//! End-to-End and Integration Tests for ZeroClaw TUI
//!
//! This test suite provides comprehensive testing of the TUI dashboard.

use super::app::{AppState, InputMode, MessageRole};
use super::events::{map_key_event, AppEvent};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Helper: Create a test AppState with default configuration
fn create_test_app() -> AppState {
    AppState::default()
}

/// Helper: Simulate a keypress event and verify the resulting AppEvent
fn simulate_keypress(key: KeyCode, modifiers: KeyModifiers, mode: InputMode) -> Option<AppEvent> {
    let key_event = KeyEvent::new(key, modifiers);
    map_key_event(key_event, &mode)
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
        AppEvent::SpawnAgent => {
            app.add_assistant_message(
                "[Info] Spawning new subagent...".to_string(),
                Some("system".to_string()),
            );
            app.active_agents.push(super::app::AgentStatus {
                id: uuid::Uuid::new_v4().to_string(),
                name: "New Agent".to_string(),
                model: "gpt-4-turbo".to_string(),
                progress: 0,
                status: super::app::AgentState::Running,
            });
        }
        AppEvent::RunTest => {
            let output = run_tui_diagnostic();
            app.add_assistant_message(output, Some("system".to_string()));
        }
        _ => {}
    }
}

/// Run TUI diagnostic test
fn run_tui_diagnostic() -> String {
    "🔍 ZeroClaw TUI Diagnostics\n✓ Config: Loaded\n✓ Memory: SQLite backend".to_string()
}

// ============================================================================
// TEST SUITE 1: Mode Switching E2E
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

// ============================================================================
// TEST SUITE 2: Session Management E2E
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

    // Add a message to session 0
    app.add_user_message("Message in session 0".to_string());

    // Switch back to session 1
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::NextSession, &mut app).await;
        });

    assert_eq!(app.active_session, 1);
    assert_eq!(app.sessions[1].messages.len(), 1);
    assert_eq!(app.sessions[1].messages[0].content, "Message in session 1");
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
    assert_eq!(app.sessions[app.active_session].messages.len(), 1);

    // Close current session (should switch to session 1)
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::CloseSession, &mut app).await;
        });

    assert_eq!(app.sessions.len(), 2);
    assert_eq!(app.active_session, 1);
}

#[test]
fn test_session_close_last_session() {
    let mut app = create_test_app();

    // Only one session
    assert_eq!(app.sessions.len(), 1);

    // Add a message
    app.add_user_message("Test message".to_string());
    assert_eq!(app.sessions[0].messages.len(), 1);

    // Close last session (should clear messages but keep session)
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::CloseSession, &mut app).await;
        });

    // Session still exists but is cleared
    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.sessions[0].messages.len(), 0);
}

#[test]
fn test_session_isolation() {
    let mut app = create_test_app();

    // Start with default session (0)
    assert_eq!(app.active_session, 0);

    // Add message to session 0
    app.add_user_message("Session 0 message".to_string());
    assert_eq!(app.sessions[0].messages.len(), 1);

    // Create a new session (this switches to session 1)
    app.new_session();
    assert_eq!(app.sessions.len(), 2);
    assert_eq!(app.active_session, 1);

    // Add message to session 1
    app.add_user_message("Session 1 message".to_string());
    assert_eq!(app.sessions[1].messages.len(), 1);

    // Verify isolation - sessions should have different messages
    assert_eq!(app.sessions[0].messages.len(), 1);
    assert_eq!(app.sessions[1].messages.len(), 1);
    assert_ne!(
        app.sessions[0].messages[0].content,
        app.sessions[1].messages[0].content
    );
    assert_eq!(app.sessions[0].messages[0].content, "Session 0 message");
    assert_eq!(app.sessions[1].messages[0].content, "Session 1 message");
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
// TEST SUITE 3: Message Handling
// ============================================================================

#[test]
fn test_send_message_in_insert_mode() {
    let mut app = create_test_app();
    app.input_mode = InputMode::Insert;
    app.input_buffer = "Test message".to_string();

    let initial_msg_count = app.sessions[app.active_session].messages.len();

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            // Press Enter to send
            handle_app_event(AppEvent::SendMessage, &mut app).await;

            // Verify message was added
            assert_eq!(app.sessions[app.active_session].messages.len(), initial_msg_count + 1);
            assert_eq!(app.sessions[app.active_session].messages.last().unwrap().role,
                      MessageRole::User);
            assert_eq!(app.sessions[app.active_session].messages.last().unwrap().content, "Test message");

            // Buffer should be cleared
            assert_eq!(app.input_buffer, "");
        });
}

#[test]
fn test_empty_input_send_message() {
    let mut app = create_test_app();
    app.input_mode = InputMode::Insert;
    app.input_buffer.clear();

    let initial_msg_count = app.sessions[app.active_session].messages.len();

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            // Try to send empty message
            handle_app_event(AppEvent::SendMessage, &mut app).await;
        });

    // No message should be added
    assert_eq!(app.sessions[app.active_session].messages.len(), initial_msg_count);
}

#[test]
fn test_scrolling_chat() {
    let mut app = create_test_app();

    // Add many messages
    for i in 0..20 {
        app.add_user_message(format!("Message {}", i));
    }

    let initial_scroll = app.chat_scroll;

    // Scroll up
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::ScrollUp, &mut app).await;
        });

    // Scroll should increase
    assert!(app.chat_scroll > initial_scroll);

    // Scroll down
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::ScrollDown, &mut app).await;
        });

    // Should return toward initial scroll
    assert!(app.chat_scroll < initial_scroll + 1);
}

// ============================================================================
// TEST SUITE 4: Edge Cases
// ============================================================================

#[test]
fn test_quit_flag() {
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

#[test]
fn test_spawn_agent() {
    let mut app = create_test_app();

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            handle_app_event(AppEvent::SpawnAgent, &mut app).await;
        });

    // Should have added a system message and an agent
    assert!(!app.sessions[app.active_session].messages.is_empty());
    assert_eq!(app.active_agents.len(), 1);
    assert_eq!(app.active_agents[0].name, "New Agent");
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
    assert_eq!(app.sessions[app.active_session].messages.len(), 1);

    app.execute_command("clear");

    assert_eq!(app.sessions[app.active_session].messages.len(), 0);
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

#[tokio::test]
async fn test_async_event_handling_with_timeout() {
    let mut app = create_test_app();

    // Use timeout to prevent hangs
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        handle_app_event(AppEvent::NewSession, &mut app)
    ).await;

    assert!(result.is_ok(), "Event handling should complete within timeout");
    assert_eq!(app.sessions.len(), 2);
}

#[tokio::test]
async fn test_async_multiple_events_sequence() {
    let mut app = create_test_app();

    let events = vec![
        AppEvent::NewSession,
        AppEvent::NextSession,
        AppEvent::PrevSession,
        AppEvent::SpawnAgent,
    ];

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        async {
            for event in events {
                handle_app_event(event, &mut app).await;
            }
        }
    ).await;

    assert!(result.is_ok(), "Multiple events should be processed within timeout");
    assert_eq!(app.sessions.len(), 2);
    assert_eq!(app.active_agents.len(), 1);
}
