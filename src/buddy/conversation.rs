//! ConversationProcessor — Przetwarzanie rozmowy z userem
//!
//! Pattern matching na intencje użytkownika + generowanie odpowiedzi
//! na podstawie aktualnego BuddyState.

use super::{BuddyState, Situation};

/// Intencja wykryta w wiadomości użytkownika
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationIntent {
    /// Powitanie (hej/cześć/hi/elo)
    Greeting,
    /// Pytanie o status (co robisz/jak leci/jak tam)
    StatusQuery,
    /// Ukończenie zadania (zrobione/gotowe/skończone)
    TaskCompleted,
    /// Ukończenie trudnego zadania (trudne zadanie/dałem radę)
    HardTaskCompleted,
    /// Blokada rozwiązana — user mówi co zrobił
    BlockerResolved(String),
    /// Pożegnanie (pa/nara/dozo)
    Goodbye,
    /// Ogólna wiadomość
    General(String),
}

/// Processor konwersacji — główny interfejs do obsługi chatu
pub struct ConversationProcessor;

impl ConversationProcessor {
    /// Przetwarza input użytkownika na podstawie aktualnego stanu Buddy
    pub fn process(state: &mut BuddyState, input: &str) -> String {
        let intent = Self::detect_intent(input);

        match intent {
            ConversationIntent::Greeting => Self::handle_greeting(state),
            ConversationIntent::StatusQuery => Self::handle_status_query(state),
            ConversationIntent::TaskCompleted => Self::handle_task_completed(state, false),
            ConversationIntent::HardTaskCompleted => Self::handle_task_completed(state, true),
            ConversationIntent::BlockerResolved(what) => {
                Self::handle_blocker_resolved(state, &what)
            }
            ConversationIntent::Goodbye => Self::handle_goodbye(state),
            ConversationIntent::General(msg) => Self::handle_general(state, &msg),
        }
    }

    /// Wykrywa intencję z tekstu użytkownika
    fn detect_intent(input: &str) -> ConversationIntent {
        let lower = input.to_lowercase();
        let trimmed = lower.trim();

        // Greetings
        if Self::matches_any(
            trimmed,
            &[
                "hej",
                "cześć",
                "czesc",
                "siema",
                "siemka",
                "elo",
                "witam",
                "dzień dobry",
                "dzien dobry",
                "yo",
                "hejka",
                "hi",
                "hello",
                "hejo",
            ],
        ) {
            return ConversationIntent::Greeting;
        }

        // Goodbyes
        if Self::matches_any(
            trimmed,
            &[
                "pa",
                "nara",
                "dozo",
                "cześć!",
                "czesc!",
                "do widzenia",
                "do zobaczenia",
                "do jutra",
                "pa pa",
                "narazie",
                "na razie",
                "spadam",
                "lecę",
                "lecę na razie",
            ],
        ) {
            return ConversationIntent::Goodbye;
        }

        // Status queries
        if Self::matches_any(
            trimmed,
            &[
                "co robisz",
                "jak leci",
                "jak tam",
                "jak się masz",
                "jak sie masz",
                "co słychać",
                "co slychac",
                "co u ciebie",
                "jak idzie",
                "jaki status",
                "co nowego",
                "how are you",
            ],
        ) {
            return ConversationIntent::StatusQuery;
        }

        // Hard task completion (check before simple task)
        if Self::matches_any(
            trimmed,
            &[
                "trudne zadanie",
                "dałem radę",
                "dalem rade",
                "udało się",
                "udalo sie",
                "to było trudne",
                "bylo trudne",
                "ale dałem",
                "w końcu!",
                "w koncu!",
                "hard task",
            ],
        ) {
            return ConversationIntent::HardTaskCompleted;
        }

        // Task completion
        if Self::matches_any(
            trimmed,
            &[
                "zrobione",
                "gotowe",
                "skończone",
                "skonczone",
                "zrobione!",
                "gotowe!",
                "ukończone",
                "ukonczone",
                "done",
                "finished",
                "odhaczone",
                "zaliczone",
                "zrobilem",
                "zrobiłem",
                "ukończyłem",
                "ukonczylem",
            ],
        ) {
            return ConversationIntent::TaskCompleted;
        }

        // Check for blocker resolution patterns
        if let Some(blocker_desc) = Self::detect_blocker_resolution(trimmed) {
            return ConversationIntent::BlockerResolved(blocker_desc);
        }

        ConversationIntent::General(input.to_string())
    }

    /// Sprawdza czy tekst pasuje do któregokolwiek z wzorców
    fn matches_any(text: &str, patterns: &[&str]) -> bool {
        patterns.iter().any(|p| text.contains(p))
    }

    /// Wykrywa czy user mówi że rozwiązał blokadę
    fn detect_blocker_resolution(text: &str) -> Option<String> {
        let prefixes = [
            "już zrobiłem ",
            "juz zrobilem ",
            "już jest ",
            "juz jest ",
            "już odpowiedziałem ",
            "juz odpowiedzialem ",
            "zdecydowałem że ",
            "zdecydowalem ze ",
            "wybrałem ",
            "wybralem ",
            "zrobiłem to ",
            "zrobilem to ",
        ];

        for prefix in &prefixes {
            if let Some(rest) = text.strip_prefix(prefix) {
                if !rest.is_empty() {
                    return Some(rest.to_string());
                }
            }
        }

        None
    }

    /// Obsługuje powitanie
    fn handle_greeting(state: &BuddyState) -> String {
        let name = &state.personality.name;
        let mut response = format!("{} {}", state.emotion.emoji(), Self::greeting_phrase());

        if state.needs_attention() {
            response.push_str(&format!(
                "\n{} — czekaj, mamy nierozwiązane sprawy. {}",
                name,
                Self::blocker_summary(state)
            ));
        }

        response
    }

    /// Losowe powitanie
    fn greeting_phrase() -> &'static str {
        let phrases = [
            "Hej! Co tam?",
            "Siema! Jak leci?",
            "Cześć! Co robimy?",
            "Hejo! Gotowy do działania?",
            "Siemka! Co dziś ogarniamy?",
        ];
        phrases[Self::hash_mod(phrases.len())]
    }

    /// Obsługuje pytanie o status
    fn handle_status_query(state: &BuddyState) -> String {
        let mut parts = vec![state.emotion.to_string()];

        if !state.blockers.is_empty() {
            parts.push(format!(
                "\nCzekam na {} decyzji — {}",
                state.blockers.len(),
                Self::blocker_summary(state)
            ));
        } else if state.tasks_completed > 0 {
            parts.push(format!(
                "\nNa razie {} zadań ogarnięte. Lecimy dalej?",
                state.tasks_completed
            ));
        } else {
            parts.push("\nNa razie cisza. Co robimy?".to_string());
        }

        parts.join("")
    }

    /// Obsługuje ukończenie zadania
    fn handle_task_completed(state: &mut BuddyState, hard: bool) -> String {
        let situation = if hard {
            Situation::HardTaskCompleted
        } else {
            Situation::TaskCompleted
        };
        state.process_situation(situation);

        if hard {
            format!(
                "{} No nieźle! Trudne zadanie zaliczone. Jesteś w formie! 💪",
                state.emotion.emoji()
            )
        } else {
            let phrases = [
                "Spoko, odhaczone! Co dalej?",
                "Git! Lecimy z następnym?",
                "No i pięknie. Kolejne z głowy!",
                "Zrobione! Tempo niezłe.",
            ];
            format!(
                "{} {}",
                state.emotion.emoji(),
                phrases[Self::hash_mod(phrases.len())]
            )
        }
    }

    /// Obsługuje rozwiązanie blokady
    fn handle_blocker_resolved(state: &mut BuddyState, what: &str) -> String {
        // Try to remove a matching blocker
        let removed = state.remove_blocker(what);

        if removed {
            state.process_situation(Situation::ProgressMade);
            format!(
                "{} W końcu! Blokada zdjęta: \"{}\". Co dalej?",
                state.emotion.emoji(),
                what
            )
        } else {
            // No exact match — still acknowledge
            format!(
                "{} Ok, zanotowane. A propos blokad — {}",
                state.emotion.emoji(),
                Self::blocker_summary(state)
            )
        }
    }

    /// Obsługuje pożegnanie
    fn handle_goodbye(state: &BuddyState) -> String {
        if state.needs_attention() {
            format!(
                "Ej, jeszcze nie skończyliśmy! {}. Ale dobra, wpadaj jak będziesz gotowy.",
                Self::blocker_summary(state)
            )
        } else {
            let phrases = [
                "Nara! Wracaj szybko.",
                "Pa! Nie każ mi czekać.",
                "Dozo! Będę tu.",
                "Na razie! Trzymaj się.",
            ];
            phrases[Self::hash_mod(phrases.len())].to_string()
        }
    }

    /// Obsługuje ogólną wiadomość
    fn handle_general(state: &mut BuddyState, _msg: &str) -> String {
        state.process_situation(Situation::UserEngaged);

        if state.needs_attention() {
            format!("Hmm, ok. Ale {}", Self::blocker_summary(state))
        } else {
            let phrases = [
                "Okej, rozumiem.",
                "No dobra.",
                "Jasne.",
                "Rozumiem, lecimy dalej.",
            ];
            phrases[Self::hash_mod(phrases.len())].to_string()
        }
    }

    /// Podsumowanie blokad (krótkie)
    fn blocker_summary(state: &BuddyState) -> String {
        if state.blockers.is_empty() {
            return String::new();
        }

        if state.blockers.len() == 1 {
            let b = &state.blockers[0];
            return format!("{} blokada: \"{}\"", b.severity.emoji(), b.what);
        }

        format!(
            "{} aktywnych blokad. Najpilniejsza: \"{}\"",
            state.blockers.len(),
            state.blockers[0].what
        )
    }

    /// Prosty hash do pseudo-losowego wyboru fraz
    fn hash_mod(modulus: usize) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .hash(&mut hasher);
        (hasher.finish() as usize) % modulus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_greeting() {
        assert_eq!(
            ConversationProcessor::detect_intent("hej"),
            ConversationIntent::Greeting
        );
        assert_eq!(
            ConversationProcessor::detect_intent("Cześć Buddy!"),
            ConversationIntent::Greeting
        );
        assert_eq!(
            ConversationProcessor::detect_intent("Siema"),
            ConversationIntent::Greeting
        );
    }

    #[test]
    fn test_detect_status_query() {
        assert_eq!(
            ConversationProcessor::detect_intent("jak leci?"),
            ConversationIntent::StatusQuery
        );
        assert_eq!(
            ConversationProcessor::detect_intent("co robisz?"),
            ConversationIntent::StatusQuery
        );
        assert_eq!(
            ConversationProcessor::detect_intent("co słychać?"),
            ConversationIntent::StatusQuery
        );
    }

    #[test]
    fn test_detect_task_completed() {
        assert_eq!(
            ConversationProcessor::detect_intent("zrobione!"),
            ConversationIntent::TaskCompleted
        );
        assert_eq!(
            ConversationProcessor::detect_intent("gotowe"),
            ConversationIntent::TaskCompleted
        );
        assert_eq!(
            ConversationProcessor::detect_intent("ukończone"),
            ConversationIntent::TaskCompleted
        );
    }

    #[test]
    fn test_detect_hard_task() {
        assert_eq!(
            ConversationProcessor::detect_intent("dałem radę z trudnym zadaniem!"),
            ConversationIntent::HardTaskCompleted
        );
        assert_eq!(
            ConversationProcessor::detect_intent("to było trudne ale udało się"),
            ConversationIntent::HardTaskCompleted
        );
    }

    #[test]
    fn test_detect_goodbye() {
        assert_eq!(
            ConversationProcessor::detect_intent("nara"),
            ConversationIntent::Goodbye
        );
        assert_eq!(
            ConversationProcessor::detect_intent("do zobaczenia"),
            ConversationIntent::Goodbye
        );
    }

    #[test]
    fn test_detect_blocker_resolution() {
        assert_eq!(
            ConversationProcessor::detect_intent("już zrobiłem decyzję o API"),
            ConversationIntent::BlockerResolved("decyzję o API".to_string())
        );
        assert_eq!(
            ConversationProcessor::detect_intent("zdecydowałem że używamy Rust"),
            ConversationIntent::BlockerResolved("używamy Rust".to_string())
        );
    }

    #[test]
    fn test_detect_general() {
        match ConversationProcessor::detect_intent("cośtam losowego") {
            ConversationIntent::General(_) => {} // ok
            other => panic!("Expected General, got {:?}", other),
        }
    }

    #[test]
    fn test_process_greeting_returns_response() {
        let mut state = BuddyState::new();
        let response = ConversationProcessor::process(&mut state, "hej!");
        assert!(!response.is_empty());
    }

    #[test]
    fn test_process_task_completed_increments_counter() {
        let mut state = BuddyState::new();
        ConversationProcessor::process(&mut state, "zrobione!");
        assert_eq!(state.tasks_completed, 1);
    }

    #[test]
    fn test_process_hard_task_sets_proud() {
        let mut state = BuddyState::new();
        ConversationProcessor::process(&mut state, "dałem radę z trudnym zadaniem!");
        assert_eq!(state.tasks_completed, 1);
        assert_eq!(state.emotion, super::super::EmotionalState::Proud);
    }

    #[test]
    fn test_process_with_blockers_mentions_them() {
        let mut state = BuddyState::new();
        state.add_blocker("Decyzja o bazie danych");
        let response = ConversationProcessor::process(&mut state, "co robisz?");
        assert!(response.contains("Decyzja o bazie danych"));
    }

    #[test]
    fn test_process_blocker_resolved_removes_blocker() {
        let mut state = BuddyState::new();
        state.add_blocker("decyzję o API");
        let response = ConversationProcessor::process(&mut state, "już zrobiłem decyzję o API");
        assert!(state.blockers.is_empty());
        assert!(response.contains("W końcu"));
    }

    #[test]
    fn test_process_goodbye_with_blockers() {
        let mut state = BuddyState::new();
        state.add_blocker("Coś ważnego");
        let response = ConversationProcessor::process(&mut state, "pa");
        assert!(response.contains("nie skończyliśmy"));
    }
}
