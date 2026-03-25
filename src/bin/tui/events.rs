//! Event handling for ZeroClaw TUI
//!
//! This module provides keyboard event mapping and handling for the terminal UI.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::app::InputMode;

/// Application events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    /// Quit application
    Quit,

    /// Create new session
    NewSession,

    /// Close current session
    CloseSession,

    /// Next session (Tab)
    NextSession,

    /// Previous session (Shift+Tab)
    PrevSession,

    /// Toggle between Normal/Insert mode
    ToggleInputMode,

    /// Send the current message
    SendMessage,

    /// Character input in Insert mode
    CharInput(char),

    /// Backspace
    Backspace,

    /// Spawn a new subagent
    SpawnAgent,

    /// Scroll chat up
    ScrollUp,

    /// Scroll chat down
    ScrollDown,

    /// Show help
    Help,

    /// Run diagnostics test
    RunTest,
}

/// Map keyboard events to application events
pub fn map_key_event(key: KeyEvent, mode: &InputMode) -> Option<AppEvent> {
    match mode {
        InputMode::Normal => map_normal_mode(key),
        InputMode::Insert => map_insert_mode(key),
        InputMode::Command => map_command_mode(key),
    }
}

/// Normal mode key bindings (navigation)
fn map_normal_mode(key: KeyEvent) -> Option<AppEvent> {
    match (key.code, key.modifiers) {
        // Ctrl+Q - Quit
        (KeyCode::Char('q'), KeyModifiers::CONTROL) => Some(AppEvent::Quit),

        // Ctrl+T - New session
        (KeyCode::Char('t'), KeyModifiers::CONTROL) => Some(AppEvent::NewSession),

        // Ctrl+W - Close session
        (KeyCode::Char('w'), KeyModifiers::CONTROL) => Some(AppEvent::CloseSession),

        // Tab - Next session (must check no other modifiers)
        (KeyCode::Tab, mods) if mods.is_empty() => Some(AppEvent::NextSession),

        // Shift+Tab - Previous session (BackTab)
        (KeyCode::BackTab, _) => Some(AppEvent::PrevSession),

        // 'i' - Enter insert mode (vim-style)
        (KeyCode::Char('i'), mods) if mods.is_empty() => Some(AppEvent::ToggleInputMode),

        // Ctrl+A - Spawn agent
        (KeyCode::Char('a'), KeyModifiers::CONTROL) => Some(AppEvent::SpawnAgent),

        // '?' - Show help
        (KeyCode::Char('?'), mods) if mods.is_empty() => Some(AppEvent::Help),

        // Arrow keys - Scroll
        (KeyCode::Up, mods) if mods.is_empty() => Some(AppEvent::ScrollUp),
        (KeyCode::Down, mods) if mods.is_empty() => Some(AppEvent::ScrollDown),

        _ => None,
    }
}

/// Insert mode key bindings (typing messages)
fn map_insert_mode(key: KeyEvent) -> Option<AppEvent> {
    match (key.code, key.modifiers) {
        // Esc - Back to normal mode
        (KeyCode::Esc, _) => Some(AppEvent::ToggleInputMode),

        // Enter - Send message
        (KeyCode::Enter, mods) if mods.is_empty() => Some(AppEvent::SendMessage),

        // Character input
        (KeyCode::Char(c), mods) if mods.is_empty() => Some(AppEvent::CharInput(c)),

        // Backspace
        (KeyCode::Backspace, _) => Some(AppEvent::Backspace),

        _ => None,
    }
}

/// Command mode key bindings (`:` commands)
fn map_command_mode(key: KeyEvent) -> Option<AppEvent> {
    match (key.code, key.modifiers) {
        // Esc - Back to normal mode
        (KeyCode::Esc, _) => Some(AppEvent::ToggleInputMode),

        // Enter - Execute command
        (KeyCode::Enter, _) => {
            // Command execution would be handled separately
            Some(AppEvent::ToggleInputMode)
        }

        // Character input for command
        (KeyCode::Char(c), mods) if mods.is_empty() => Some(AppEvent::CharInput(c)),

        // Backspace
        (KeyCode::Backspace, _) => Some(AppEvent::Backspace),

        _ => None,
    }
}

/// Get help text for key bindings
pub fn get_help_text() -> &'static str {
    r#"ZeroClaw TUI Dashboard - Key Bindings

Navigation (Normal Mode):
  Ctrl+Q    Quit application
  Ctrl+T    Create new session
  Ctrl+W    Close current session
  Tab       Next session
  Shift+Tab Previous session
  i         Enter insert mode (type message)
  Ctrl+A    Spawn new subagent
  ?         Show this help
  Up/Down   Scroll chat

Insert Mode (Typing):
  Esc       Back to normal mode
  Enter     Send message

Command Mode (:):
  Esc       Back to normal mode
  Enter     Execute command

Available Commands:
  :q         Quit
  :w         Export session
  :n         New session
  :rename X  Rename session
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_mode_quit() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        let event = map_key_event(key, &InputMode::Normal);
        assert_eq!(event, Some(AppEvent::Quit));
    }

    #[test]
    fn test_normal_mode_new_session() {
        let key = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL);
        let event = map_key_event(key, &InputMode::Normal);
        assert_eq!(event, Some(AppEvent::NewSession));
    }

    #[test]
    fn test_normal_mode_next_session() {
        let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::empty());
        let event = map_key_event(key, &InputMode::Normal);
        assert_eq!(event, Some(AppEvent::NextSession));
    }

    #[test]
    fn test_insert_mode_send_message() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let event = map_key_event(key, &InputMode::Insert);
        assert_eq!(event, Some(AppEvent::SendMessage));
    }

    #[test]
    fn test_insert_mode_char_input() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
        let event = map_key_event(key, &InputMode::Insert);
        assert_eq!(event, Some(AppEvent::CharInput('a')));
    }

    #[test]
    fn test_insert_mode_escape() {
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        let event = map_key_event(key, &InputMode::Insert);
        assert_eq!(event, Some(AppEvent::ToggleInputMode));
    }
}
