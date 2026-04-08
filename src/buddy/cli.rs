//! CLI — Interaktywny interfejs tekstowy dla Buddy
//!
//! Prosty REPL do rozmowy z Buddym przez stdin/stdout.

use super::{BuddyState, Personality};
use std::io::{self, Write};

/// Uruchamia interaktywną pętlę rozmowy z Buddym
pub fn run_cli() {
    let mut state = BuddyState::with_personality(Personality::ziomek());

    println!("{}", state.personality.greet());
    println!("(Wpisz 'quit' żeby zakończyć)\n");

    loop {
        print!("ty > ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(_) => break,
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if input == "quit" || input == "exit" {
            println!("\n{}", state.personality.farewell());
            break;
        }

        let response = state.process_input(input);
        println!("buddy > {}\n", response);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_cli_compiles() {
        // Smoke test — just verifies the function exists and is callable
        // Actual REPL testing would require mocking stdin
        let _ = run_cli as fn();
    }
}
