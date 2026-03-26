//! ZeroClaw TUI Dashboard
//!
//! A terminal user interface for interacting with ZeroClaw AI agent.
//! Features multiple sessions, agent monitoring, and real-time chat.

#![allow(clippy::too_many_lines)]

mod agents;
mod app;
mod events;
mod sessions;
mod ui;

#[cfg(test)]
mod e2e_tests;

use agents::ZeroClawClient;
use app::{
    AgentState, AppState, BudgetStatus, ChatMessage, LogEntry, MemoryOp, MemoryOperation,
    MessageRole, ModelStats,
};
use chrono::Utc;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use events::{map_key_event, AppEvent};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::{Duration, Instant};

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
    println!("  s, c, m, l Toggle Panels (Swarm, Cost, Memory, Logs)");
    println!("  :          Enter command mode (:q to quit, :help)");
    println!("  ?          Show help");
    println!("  q          Quit (in normal mode)");
}

/// Print version information
fn print_version() {
    println!("zeroclaw-tui {}", env!("CARGO_PKG_VERSION"));
}

/// Install panic hook to ensure terminal is restored on crash
fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
        let _ = execute!(stdout, crossterm::cursor::Show);
        original_hook(panic_info);
    }));
}

/// Main entry point for ZeroClaw TUI
fn main() {
    // Check for help/version flags
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--help" | "-h" => {
                print_usage();
                return;
            }
            "--version" | "-V" => {
                print_version();
                return;
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

    install_panic_hook();

    // Run the async main
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    if let Err(e) = runtime.block_on(async_main()) {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

/// Async main function
async fn async_main() -> anyhow::Result<()> {
    // Check for demo mode
    let demo_mode = std::env::var(DEMO_MODE_ENV).is_ok() || true; // Force demo for now to show off panels

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize app state
    let mut app = AppState::default();

    // Initialize mock data if in demo mode
    if demo_mode {
        initialize_mock_data(&mut app);
    }

    // ZeroClaw HTTP client
    let zeroclaw_client = if demo_mode {
        None
    } else {
        Some(ZeroClawClient::localhost())
    };

    // Main event loop
    let mut help_visible = false;
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(100);

    loop {
        // Update mock data periodically
        if demo_mode && last_tick.elapsed() >= Duration::from_secs(1) {
            update_mock_data(&mut app);
            last_tick = Instant::now();
        }

        // Draw UI
        terminal.draw(|f| {
            if help_visible {
                ui::render_help(f);
            } else {
                ui::render(f, &app);
            }
        })?;

        // Poll for events
        if crossterm::event::poll(tick_rate)? {
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
                    if app_event == AppEvent::ToggleInputMode
                        && app.input_mode == app::InputMode::Command
                    {
                        // Exiting command mode - execute command
                        if !app.input_buffer.is_empty() {
                            let input_clone = app.input_buffer.clone();
                            let (output, should_clear) = app.execute_command(&input_clone);
                            app.add_assistant_message(
                                format!(":{}", app.input_buffer),
                                Some("cmd".to_string()),
                            );
                            if !output.is_empty() && output != format!(":{}", app.input_buffer) {
                                app.add_assistant_message(output, Some("system".to_string()));
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
async fn handle_event(event: AppEvent, app: &mut AppState, client: &Option<ZeroClawClient>) {
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
                    app.add_assistant_message(
                        format!("[Demo Mode] Received: {}", message),
                        Some("demo-model".to_string()),
                    );
                } else if let Some(client) = client {
                    // Send to ZeroClaw API
                    let session_id = app
                        .current_session()
                        .map(|s| s.id.clone())
                        .unwrap_or_default();

                    match client.send_message(&session_id, &message).await {
                        Ok(response) => {
                            app.add_assistant_message(response, None);
                        }
                        Err(e) => {
                            app.add_assistant_message(format!("[Error] {}", e), None);
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
            app.add_assistant_message(
                "[Info] Spawning new subagent...".to_string(),
                Some("system".to_string()),
            );
            // Mock spawning
            app.swarm_panel.agents.push(app::AgentStatus {
                role: "New Agent".to_string(),
                state: AgentState::Running,
                current_task: Some("Initializing...".to_string()),
                progress: 0,
            });
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

        AppEvent::RunTest => {
            let output = run_tui_diagnostic();
            app.add_assistant_message(output, Some("system".to_string()));
        }

        AppEvent::Help => {
            // Handled in main loop
        }
    }
}

/// Initialize some mock data for the TUI dashboard
fn initialize_mock_data(app: &mut AppState) {
    // Swarm
    app.swarm_panel.agents = vec![
        app::AgentStatus {
            role: "Researcher".to_string(),
            state: AgentState::Running,
            current_task: Some("Searching for EPIC specs".to_string()),
            progress: 45,
        },
        app::AgentStatus {
            role: "Coder".to_string(),
            state: AgentState::Idle,
            current_task: None,
            progress: 100,
        },
    ];

    // Cost
    app.cost_panel.session_cost = 0.042;
    app.cost_panel.daily_cost = 1.25;
    app.cost_panel.monthly_cost = 15.80;
    app.cost_panel.model_usage = vec![
        ModelStats {
            model: "gpt-4-turbo".to_string(),
            tokens_used: 15400,
            cost_usd: 0.15,
            percentage: 65.0,
        },
        ModelStats {
            model: "claude-3-opus".to_string(),
            tokens_used: 5200,
            cost_usd: 0.08,
            percentage: 35.0,
        },
    ];

    // Memory
    app.memory_panel.backend_name = "Qdrant (Local)".to_string();
    app.memory_panel.vector_count = 12450;
    app.memory_panel.cache_hit_rate = 88.5;
    app.memory_panel.recent_operations = vec![
        MemoryOperation {
            operation: MemoryOp::Recall {
                query: "tui".to_string(),
                results: 5,
            },
            key: "query:tui".to_string(),
            category: "search".to_string(),
            timestamp: Utc::now(),
            duration_ms: 45,
        },
        MemoryOperation {
            operation: MemoryOp::Store {
                key: "session_123".to_string(),
                size_bytes: 1024,
            },
            key: "session_123".to_string(),
            category: "chat".to_string(),
            timestamp: Utc::now(),
            duration_ms: 12,
        },
    ];

    // Logs
    app.log_panel.logs = vec![
        LogEntry {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            target: "zero::tui".to_string(),
            message: "TUI Dashboard initialized".to_string(),
        },
        LogEntry {
            timestamp: Utc::now(),
            level: "DEBUG".to_string(),
            target: "zero::swarm".to_string(),
            message: "Scanning for active agents...".to_string(),
        },
    ];
}

/// Update mock data to show activity
fn update_mock_data(app: &mut AppState) {
    // Progress agents
    for agent in &mut app.swarm_panel.agents {
        if agent.state == AgentState::Running {
            agent.progress = (agent.progress + 5) % 101;
            if agent.progress == 0 {
                agent.state = AgentState::Done;
            }
        } else if agent.state == AgentState::Done {
            agent.state = AgentState::Idle;
        } else if agent.state == AgentState::Idle {
            agent.state = AgentState::Running;
            agent.progress = 0;
        }
    }

    // Add a mock log
    let levels = ["INFO", "DEBUG", "WARN"];
    let targets = ["zero::gateway", "zero::memory", "zero::cost"];
    let messages = [
        "Heartbeat received",
        "Cache invalidated",
        "Usage threshold check",
    ];

    let now = Utc::now();
    app.log_panel.logs.push(LogEntry {
        timestamp: now,
        level: levels[now.timestamp() as usize % 3].to_string(),
        target: targets[now.timestamp() as usize % 3].to_string(),
        message: messages[now.timestamp() as usize % 3].to_string(),
    });

    if app.log_panel.logs.len() > 10 {
        app.log_panel.logs.remove(0);
    }

    // Increment cost slightly
    app.cost_panel.session_cost += 0.0001;
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
    results.push("─".repeat(40));

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

    results.push("─".repeat(40));
    results.push("Commands:".to_string());
    results.push("  :test    Run diagnostics".to_string());
    results.push("  :new     Create new session".to_string());
    results.push("  :clear   Clear current session".to_string());
    results.push("  :q       Quit".to_string());

    results.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use app::InputMode;

    #[test]
    fn test_event_handling_quit() {
        let mut app = AppState::default();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            handle_event(AppEvent::Quit, &mut app, &None).await;
            assert!(app.should_quit);
        });
    }

    #[test]
    fn test_event_handling_new_session() {
        let mut app = AppState::default();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            handle_event(AppEvent::NewSession, &mut app, &None).await;
            assert_eq!(app.sessions.len(), 2);
        });
    }

    #[test]
    fn test_event_handling_toggle_mode() {
        let mut app = AppState::default();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            assert_eq!(app.input_mode, InputMode::Normal);
            handle_event(AppEvent::ToggleInputMode, &mut app, &None).await;
            assert_eq!(app.input_mode, InputMode::Insert);
            handle_event(AppEvent::ToggleInputMode, &mut app, &None).await;
            assert_eq!(app.input_mode, InputMode::Normal);
        });
    }

    #[test]
    fn test_event_handling_panel_toggles() {
        let mut app = AppState::default();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            assert!(app.show_swarm);
            handle_event(AppEvent::ToggleSwarm, &mut app, &None).await;
            assert!(!app.show_swarm);

            assert!(!app.show_logs);
            handle_event(AppEvent::ToggleLogs, &mut app, &None).await;
            assert!(app.show_logs);
        });
    }
}
