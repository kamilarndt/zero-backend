//! ZeroClaw TUI Dashboard
//!
//! A terminal user interface for interacting with ZeroClaw AI agent.
//! Features multiple sessions, agent monitoring, and real-time chat.

#![allow(clippy::too_many_lines)]

mod agents;
mod app;
mod events;
mod panels;
mod sessions;
mod state;
mod ui;

#[cfg(test)]
mod e2e_tests;

use agents::ZeroClawClient;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use events::{map_key_event, AppEvent};
use ratatui::{backend::CrosstermBackend, Terminal};
use state::*;
use std::io;
use std::sync::Arc;
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
    let demo_mode = std::env::var(DEMO_MODE_ENV).is_ok();

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize app state
    let mut app = if demo_mode {
        AppState::new_demo()
    } else {
        AppState::default()
    };

    // ZeroClaw HTTP client
    let zeroclaw_client = if demo_mode {
        None
    } else {
        Some(ZeroClawClient::localhost())
    };

    // Create state channels for async updates
    let channels = TuiStateChannels::new();

    // Spawn update tasks
    // Temporary: Create dummy instances for swarm_update_task
    // Task 3.1 will replace this with proper initialization
    let temp_cache = Arc::new(state::cache::RequestCache::new(Duration::from_secs(5)));
    let (_, temp_shutdown) = tokio::sync::broadcast::channel::<()>(1);

    let _swarm_handle = tokio::spawn(subsystems::swarm_update_task(
        channels.swarm_tx.clone(),
        temp_cache,
        temp_shutdown,
    ));
    let _cost_handle = tokio::spawn(subsystems::cost_update_task(channels.cost_tx.clone()));
    let _memory_handle = tokio::spawn(subsystems::memory_update_task(channels.memory_tx.clone()));
    let _logs_handle = tokio::spawn(subsystems::logs_update_task(channels.logs_tx.clone()));

    // Subscribe to updates
    let mut swarm_rx = channels.subscribe_swarm();
    let mut cost_rx = channels.subscribe_cost();
    let mut memory_rx = channels.subscribe_memory();
    let mut logs_rx = channels.subscribe_logs();

    // Main event loop
    let mut help_visible = false;
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(100);

    loop {
        // Check for state updates (non-blocking)
        if swarm_rx.has_changed().unwrap_or(false) {
            let snapshot = swarm_rx.borrow_and_update().clone();
            let mut swarm_panel = app.swarm_panel.lock();
            swarm_panel.agents = snapshot.active_agents;
            swarm_panel.tasks_completed = snapshot.tasks_completed;
            swarm_panel.throughput = snapshot.throughput;
        }

        if cost_rx.has_changed().unwrap_or(false) {
            let snapshot = cost_rx.borrow_and_update().clone();
            let mut cost_panel = app.cost_panel.lock();
            cost_panel.session_cost = snapshot.session_cost_usd;
            cost_panel.daily_cost = snapshot.daily_cost_usd;
            cost_panel.daily_limit = snapshot.daily_limit_usd;
            cost_panel.monthly_cost = snapshot.monthly_cost_usd;
            cost_panel.monthly_limit = snapshot.monthly_limit_usd;
        }

        if memory_rx.has_changed().unwrap_or(false) {
            let snapshot = memory_rx.borrow_and_update().clone();
            let mut memory_panel = app.memory_panel.lock();
            memory_panel.backend = snapshot.backend;
            memory_panel.total_memories = snapshot.total_memories;
            memory_panel.storage_bytes = snapshot.storage_bytes;
            memory_panel.recent_operations = snapshot.recent_operations;
        }

        if logs_rx.has_changed().unwrap_or(false) {
            let snapshot = logs_rx.borrow_and_update().clone();
            let mut log_panel = app.log_panel.lock();
            log_panel.log_lines = snapshot.log_lines.into();
        }

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
                if key.code == KeyCode::Char('?') && app.input_mode == InputMode::Normal {
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
                        && app.input_mode == InputMode::Command
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
                InputMode::Normal => InputMode::Insert,
                InputMode::Insert | InputMode::Command => InputMode::Normal,
            };
        }

        AppEvent::SendMessage => {
            if !app.input_buffer.is_empty() {
                let message = app.input_buffer.clone();
                app.add_user_message(message.clone());

                // Get the selected agent for routing
                let selected_agent = app.selected_agent();

                // In demo mode, echo a fake response
                if client.is_none() {
                    app.add_assistant_message(
                        format!("[Demo Mode] Received via '{}': {}", selected_agent, message),
                        Some(format!("demo-{}", selected_agent)),
                    );
                } else if let Some(client) = client {
                    // Send to ZeroClaw API with selected agent hint
                    let session_id = app
                        .current_session()
                        .map(|s| s.id.clone())
                        .unwrap_or_default();
                    let agent_hint = app.selected_agent();

                    match client
                        .send_message_with_agent(&session_id, &message, Some(&agent_hint))
                        .await
                    {
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
            // Mock spawning - add a new active agent
            app.active_agents.push(AgentStatus {
                id: uuid::Uuid::new_v4().to_string(),
                name: "New Agent".to_string(),
                model: "gpt-4-turbo".to_string(),
                progress: 0,
                status: AgentState::Running,
            });
        }

        AppEvent::ScrollUp => {
            app.scroll_up();
        }

        AppEvent::ScrollDown => {
            app.scroll_down();
        }

        AppEvent::RunTest => {
            let output = run_tui_diagnostic();
            app.add_assistant_message(output, Some("system".to_string()));
        }

        AppEvent::Help => {
            // Handled in main loop
        }

        AppEvent::ToggleSwarmPanel => {
            app.toggle_panel(PanelType::Swarm);
        }

        AppEvent::ToggleCostPanel => {
            app.toggle_panel(PanelType::Cost);
        }

        AppEvent::ToggleMemoryPanel => {
            app.toggle_panel(PanelType::Memory);
        }

        AppEvent::ToggleLogsPanel => {
            app.toggle_panel(PanelType::Logs);
        }

        AppEvent::NextAgent => {
            app.next_agent();
        }

        AppEvent::PrevAgent => {
            app.prev_agent();
        }

        AppEvent::SelectAgent(index) => {
            app.set_selected_agent(index);
        }
    }
}

/// Update mock data to show activity
fn update_mock_data(app: &mut AppState) {
    // Update swarm agents
    {
        let mut swarm = app.swarm_panel.lock();
        for agent in &mut swarm.agents {
            if agent.status == crate::state::subsystems::AgentStatus::Running {
                agent.progress = agent.progress.saturating_add(5);
                if agent.progress >= 100 {
                    agent.progress = 100;
                    agent.status = crate::state::subsystems::AgentStatus::Done;
                }
            }
        }
        swarm.tasks_completed = swarm.tasks_completed.saturating_add(1);
    }

    // Update cost panel
    {
        let mut cost = app.cost_panel.lock();
        cost.session_cost += 0.01;
        cost.daily_cost += 0.01;
        cost.monthly_cost += 0.01;
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
    use InputMode;

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
}
