//! ZeroClaw TUI Dashboard
//!
//! A terminal user interface for interacting with ZeroClaw AI agent.
//! Features multiple sessions, agent monitoring, and real-time chat.

#![allow(clippy::too_many_lines)]

mod app;
mod agents;
mod events;
mod sessions;
mod ui;

use app::AppState;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use events::{map_key_event, AppEvent};
use agents::ZeroClawClient;
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use std::time::Duration;

/// Demo mode environment variable
const DEMO_MODE_ENV: &str = "ZEROCLAW_TUI_DEMO";

/// Check if stdout is a terminal (TTY)
fn is_terminal() -> bool {
    atty::is(atty::Stream::Stdout)
}

/// Print usage information
fn print_usage() {
    println!("ZeroClaw TUI Dashboard v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE:");
    println!("  zeroclaw-tui           Start TUI dashboard");
    println!("  zeroclaw-tui --help    Show this help");
    println!("  zeroclaw-tui --version Show version");
    println!();
    println!("ENVIRONMENT:");
    println!("  ZEROCLAW_TUI_DEMO=1    Run in demo mode (no API connection)");
    println!();
    println!("KEY BINDINGS:");
    println!("  i          Enter insert mode (type messages)");
    println!("  Esc        Return to normal mode");
    println!("  Ctrl+T     Create new session");
    println!("  Tab        Next session");
    println!("  Shift+Tab  Previous session");
    println!("  Ctrl+W     Close current session");
    println!("  :          Enter command mode (:q to quit, :help)");
    println!("  ?          Show help");
    println!("  q          Quit (in normal mode)");
}

/// Print version information
fn print_version() {
    println!("zeroclaw-tui {}", env!("CARGO_PKG_VERSION"));
}

/// Main entry point for ZeroClaw TUI
fn run() -> anyhow::Result<()> {
    // Check for help/version flags
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            "--version" | "-V" => {
                print_version();
                return Ok(());
            }
            _ => {
                eprintln!("Unknown option: {}", args[1]);
                eprintln!("Run 'zeroclaw-tui --help' for usage information");
                std::process::exit(1);
            }
        }
    }

    // Verify we're in a terminal
    if !is_terminal() {
        eprintln!("Error: zeroclaw-tui requires a terminal (TTY) to run.");
        eprintln!();
        eprintln!("This typically means:");
        eprintln!("  1. You're piping input/output (e.g., via ssh or a script)");
        eprintln!("  2. You're running in a non-interactive environment");
        eprintln!();
        eprintln!("To run the TUI, ensure you have an interactive terminal session.");
        eprintln!("Then simply run: zeroclaw-tui");
        std::process::exit(1);
    }

    // Run the async main
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async_main())
}

/// Async main function
async fn async_main() -> anyhow::Result<()> {
    // Check for demo mode
    let demo_mode = std::env::var(DEMO_MODE_ENV).is_ok();

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize app state
    let mut app = AppState::default();

    // ZeroClaw HTTP client
    let zeroclaw_client = if demo_mode {
        None
    } else {
        Some(ZeroClawClient::localhost())
    };

    // Spawn background status refresh task
    let app_ref = &app as *const AppState as usize;
    let _refresh_handle = if !demo_mode {
        Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            loop {
                interval.tick().await;
                // In real implementation, this would update app.active_agents
                // and app.router_status via shared state
            }
        }))
    } else {
        None
    };

    // Main event loop
    let mut help_visible = false;
    loop {
        // Draw UI
        terminal.draw(|f| {
            if help_visible {
                ui::render_help(f);
            } else {
                ui::render(f, &app);
            }
        })?;

        // Poll for events (50ms timeout)
        if crossterm::event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = crossterm::event::read()? {
                // Handle help toggle
                if key.code == KeyCode::Char('?') && app.input_mode == app::InputMode::Normal {
                    help_visible = !help_visible;
                    continue;
                }

                // Skip other input if help is visible
                if help_visible {
                    if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
                        help_visible = false;
                    }
                    continue;
                }

                // Map key to app event
                if let Some(app_event) = map_key_event(key, &app.input_mode) {
                    // Special handling for command execution
                    if app_event == AppEvent::ToggleInputMode && app.input_mode == app::InputMode::Command {
                        // Exiting command mode - execute command
                        if !app.input_buffer.is_empty() {
                            let input_clone = app.input_buffer.clone();
                            let (output, should_clear) = app.execute_command(&input_clone);
                            app.add_system_message(format!(":{}", app.input_buffer));
                            if !output.is_empty() && output != format!(":{}", app.input_buffer) {
                                app.add_system_message(output);
                            }
                            if should_clear {
                                app.input_buffer.clear();
                            }
                        }
                    }
                    handle_event(app_event, &mut app, &zeroclaw_client).await;
                }
            }
        }

        // Exit conditions
        if app.should_quit {
            break;
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

/// Handle application events
async fn handle_event(
    event: AppEvent,
    app: &mut AppState,
    client: &Option<ZeroClawClient>,
) {
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
                app::InputMode::Normal => app::InputMode::Insert,
                app::InputMode::Insert | app::InputMode::Command => app::InputMode::Normal,
            };
        }

        AppEvent::SendMessage => {
            if !app.input_buffer.is_empty() {
                let message = app.input_buffer.clone();
                app.add_user_message(message.clone());

                // In demo mode, echo a fake response
                if client.is_none() {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    app.add_assistant_message(
                        format!("[Demo Mode] Received: {}", message),
                        Some("demo-model".to_string()),
                    );
                } else if let Some(client) = client {
                    // Send to ZeroClaw API
                    let session_id = app.current_session()
                        .map(|s| s.id.clone())
                        .unwrap_or_default();

                    match client.send_message(&session_id, &message).await {
                        Ok(response) => {
                            app.add_assistant_message(response, None);
                        }
                        Err(e) => {
                            app.add_assistant_message(
                                format!("[Error] {}", e),
                                None,
                            );
                        }
                    }
                }

                app.input_buffer.clear();
            }
        }

        AppEvent::CharInput(c) => {
            app.input_buffer.push(c);
        }

        AppEvent::Backspace => {
            app.input_buffer.pop();
        }

        AppEvent::SpawnAgent => {
            // Trigger subagent spawning
            if let Some(_client) = client {
                // Would call agent spawn endpoint here
                app.add_assistant_message(
                    "[Info] Subagent spawning not yet implemented".to_string(),
                    None,
                );
            }
        }

        AppEvent::ScrollUp => {
            app.scroll_up();
        }

        AppEvent::ScrollDown => {
            app.scroll_down();
        }

        AppEvent::RunTest => {
            // Run diagnostic test and display results
            let output = run_tui_diagnostic();
            app.add_system_message(output);
        }

        AppEvent::Help => {
            // Handled in main loop
        }
    }
}

/// Run TUI diagnostic test
fn run_tui_diagnostic() -> String {
    use std::time::Duration;

    // Try to connect to local gateway
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default();

    let mut results = Vec::new();
    results.push("🔍 ZeroClaw TUI Diagnostics".to_string());
    results.push("─".repeat(40).to_string());

    // Test 1: Gateway connection
    match client.get("http://127.0.0.1:42617/health").send() {
        Ok(response) => {
            if response.status().is_success() {
                results.push("✓ Gateway: CONNECTED".to_string());
            } else {
                results.push(format!("✗ Gateway: HTTP {}", response.status()));
            }
        }
        Err(e) => {
            results.push(format!("✗ Gateway: {}", e));
        }
    }

    // Test 2: Diagnostic endpoint
    match client.get("http://127.0.0.1:42617/api/diagnostic").send() {
        Ok(response) => {
            if response.status().is_success() {
                results.push("✓ Diagnostic API: WORKING".to_string());
            } else {
                results.push(format!("! Diagnostic API: HTTP {}", response.status()));
            }
        }
        Err(e) => {
            results.push(format!("✗ Diagnostic API: {}", e));
        }
    }

    results.push("─".repeat(40).to_string());
    results.push("Commands:".to_string());
    results.push("  :test    Run diagnostics".to_string());
    results.push("  :new     Create new session".to_string());
    results.push("  :clear   Clear current session".to_string());
    results.push("  :q       Quit".to_string());

    results.join("\n")
}

/// Entry point - delegates to run() for proper error handling
fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app::InputMode;

    #[test]
    fn test_event_handling_quit() {
        let mut app = AppState::default();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                handle_event(AppEvent::Quit, &mut app, &None).await;
                assert!(app.should_quit);
            });
    }

    #[test]
    fn test_event_handling_new_session() {
        let mut app = AppState::default();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                handle_event(AppEvent::NewSession, &mut app, &None).await;
                assert_eq!(app.sessions.len(), 2);
            });
    }

    #[test]
    fn test_event_handling_toggle_mode() {
        let mut app = AppState::default();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                assert_eq!(app.input_mode, InputMode::Normal);
                handle_event(AppEvent::ToggleInputMode, &mut app, &None).await;
                assert_eq!(app.input_mode, InputMode::Insert);
                handle_event(AppEvent::ToggleInputMode, &mut app, &None).await;
                assert_eq!(app.input_mode, InputMode::Normal);
            });
    }

    #[test]
    fn test_event_handling_char_input() {
        let mut app = AppState::default();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                handle_event(AppEvent::CharInput('a'), &mut app, &None).await;
                handle_event(AppEvent::CharInput('b'), &mut app, &None).await;
                assert_eq!(app.input_buffer, "ab");
            });
    }

    #[test]
    fn test_event_handling_backspace() {
        let mut app = AppState::default();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                app.input_buffer = "test".to_string();
                handle_event(AppEvent::Backspace, &mut app, &None).await;
                assert_eq!(app.input_buffer, "tes");
            });
    }
}
