//! BuddyState — Combined state machine
//!
//! Łączy Personality, EmotionalState i Blockers w jedną działającą całość.
//! To jest główny interfejs do interakcji z Buddym.

use serde::{Deserialize, Serialize};

use super::{Blocker, EmotionalState, Personality, Situation};

/// Główny stan Buddy — łączy wszystkie komponenty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuddyState {
    /// Osobowość (config)
    pub personality: Personality,
    /// Aktualny stan emocjonalny
    pub emotion: EmotionalState,
    /// Lista aktywnych blokad
    pub blockers: Vec<Blocker>,
    /// Ile zadań user ukończył w tej sesji
    pub tasks_completed: u32,
    /// Ile zobowiązań zostało złamanych
    pub commitments_broken: u32,
}

impl BuddyState {
    /// Nowy Buddy z domyślną osobowością
    pub fn new() -> Self {
        Self {
            personality: Personality::ziomek(),
            emotion: EmotionalState::default(),
            blockers: Vec::new(),
            tasks_completed: 0,
            commitments_broken: 0,
        }
    }

    /// Buddy z custom personality
    pub fn with_personality(personality: Personality) -> Self {
        Self {
            personality,
            ..Self::new()
        }
    }

    /// Dodaje blocker
    pub fn add_blocker(&mut self, what: impl Into<String>) {
        self.blockers.push(Blocker::new(what));
    }

    /// Usuwa blocker po opisie (pierwszy match)
    pub fn remove_blocker(&mut self, what: &str) -> bool {
        if let Some(pos) = self.blockers.iter().position(|b| b.what == what) {
            self.blockers.remove(pos);
            true
        } else {
            false
        }
    }

    /// Aktualizuje severity wszystkich blockerów
    pub fn update_blockers(&mut self) {
        for blocker in &mut self.blockers {
            blocker.update_severity();
        }
    }

    /// Przetwarza sytuację — aktualizuje emocje i statystyki
    pub fn process_situation(&mut self, situation: Situation) {
        // Track stats
        match &situation {
            Situation::TaskCompleted | Situation::HardTaskCompleted => {
                self.tasks_completed += 1;
            }
            Situation::CommitmentBroken(_) => {
                self.commitments_broken += 1;
            }
            _ => {}
        }

        // Transition emotional state
        self.emotion = self.emotion.update(situation);
    }

    /// Generuje response na podstawie aktualnego stanu
    pub fn generate_response(&self) -> String {
        let mut parts = Vec::new();

        // Emotional opener
        parts.push(self.emotion.to_string());

        // Blocker status
        if !self.blockers.is_empty() {
            self.update_blockers_ref();
            parts.push(format!("\nAktywne blokady ({}):", self.blockers.len()));
            for blocker in &self.blockers {
                parts.push(format!("  {}", blocker.response()));
            }
        }

        // Stats (jeśli są jakieś)
        if self.tasks_completed > 0 {
            parts.push(format!(
                "\n📊 Postęp: {} zadań ukończonych",
                self.tasks_completed
            ));
        }

        parts.join("\n")
    }

    /// Helper do update blockerów bez mut (do generowania response)
    fn update_blockers_ref(&self) {
        // W response tylko odczytujemy — severity jest zaktualizowane
        // przez update_blockers() wywołane wcześniej
    }

    /// Czy Buddy potrzebuje uwagi usera?
    pub fn needs_attention(&self) -> bool {
        self.emotion.needs_attention()
            || self.blockers.iter().any(|b| {
                b.severity >= super::BlockerSeverity::High
            })
    }

    /// Najwyższy severity wśród blockerów
    pub fn max_blocker_severity(&self) -> Option<super::BlockerSeverity> {
        self.blockers.iter().map(|b| b.severity).max()
    }

    /// Statystyki sesji
    pub fn session_stats(&self) -> SessionStats {
        SessionStats {
            tasks_completed: self.tasks_completed,
            commitments_broken: self.commitments_broken,
            active_blockers: self.blockers.len(),
            current_emotion: self.emotion,
        }
    }
}

impl Default for BuddyState {
    fn default() -> Self {
        Self::new()
    }
}

/// Statystyki sesji
#[derive(Debug, Clone, Serialize)]
pub struct SessionStats {
    pub tasks_completed: u32,
    pub commitments_broken: u32,
    pub active_blockers: usize,
    pub current_emotion: EmotionalState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buddy_is_neutral() {
        let buddy = BuddyState::new();
        assert_eq!(buddy.emotion, EmotionalState::Neutral);
        assert!(buddy.blockers.is_empty());
    }

    #[test]
    fn test_add_and_remove_blocker() {
        let mut buddy = BuddyState::new();
        buddy.add_blocker("Decyzja A");
        assert_eq!(buddy.blockers.len(), 1);

        let removed = buddy.remove_blocker("Decyzja A");
        assert!(removed);
        assert!(buddy.blockers.is_empty());
    }

    #[test]
    fn test_remove_nonexistent_blocker() {
        let mut buddy = BuddyState::new();
        let removed = buddy.remove_blocker("Nie ma takiego");
        assert!(!removed);
    }

    #[test]
    fn test_process_task_completed() {
        let mut buddy = BuddyState::new();
        buddy.process_situation(Situation::TaskCompleted);
        assert_eq!(buddy.emotion, EmotionalState::Satisfied);
        assert_eq!(buddy.tasks_completed, 1);
    }

    #[test]
    fn test_process_hard_task() {
        let mut buddy = BuddyState::new();
        buddy.process_situation(Situation::HardTaskCompleted);
        assert_eq!(buddy.emotion, EmotionalState::Proud);
        assert_eq!(buddy.tasks_completed, 1);
    }

    #[test]
    fn test_process_commitment_broken() {
        let mut buddy = BuddyState::new();
        buddy.process_situation(Situation::CommitmentBroken(1));
        assert_eq!(buddy.emotion, EmotionalState::Frustrated);
        assert_eq!(buddy.commitments_broken, 1);
    }

    #[test]
    fn test_multiple_situations() {
        let mut buddy = BuddyState::new();
        buddy.process_situation(Situation::TaskCompleted);
        buddy.process_situation(Situation::ProgressMade);
        assert_eq!(buddy.emotion, EmotionalState::Excited);
        assert_eq!(buddy.tasks_completed, 1);
    }

    #[test]
    fn test_generate_response_contains_emoji() {
        let mut buddy = BuddyState::new();
        buddy.process_situation(Situation::TaskCompleted);
        let response = buddy.generate_response();
        assert!(response.contains("🙂"));
    }

    #[test]
    fn test_response_with_blockers() {
        let mut buddy = BuddyState::new();
        buddy.add_blocker("Test blocker");
        let response = buddy.generate_response();
        assert!(response.contains("Test blocker"));
        assert!(response.contains("Aktywne blokady"));
    }

    #[test]
    fn test_needs_attention_with_frustrated() {
        let mut buddy = BuddyState::new();
        buddy.process_situation(Situation::CommitmentBroken(1));
        assert!(buddy.needs_attention());
    }

    #[test]
    fn test_session_stats() {
        let mut buddy = BuddyState::new();
        buddy.process_situation(Situation::TaskCompleted);
        buddy.process_situation(Situation::TaskCompleted);
        buddy.process_situation(Situation::CommitmentBroken(1));
        buddy.add_blocker("Test");

        let stats = buddy.session_stats();
        assert_eq!(stats.tasks_completed, 2);
        assert_eq!(stats.commitments_broken, 1);
        assert_eq!(stats.active_blockers, 1);
    }

    #[test]
    fn test_custom_personality() {
        let p = Personality::builder()
            .name("Kumpel")
            .sarcasm_level(0.7)
            .build();
        let buddy = BuddyState::with_personality(p);
        assert_eq!(buddy.personality.name, "Kumpel");
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut buddy = BuddyState::new();
        buddy.add_blocker("Serialize test");
        buddy.process_situation(Situation::TaskCompleted);

        let json = serde_json::to_string(&buddy).unwrap();
        let buddy2: BuddyState = serde_json::from_str(&json).unwrap();
        assert_eq!(buddy.emotion, buddy2.emotion);
        assert_eq!(buddy.tasks_completed, buddy2.tasks_completed);
        assert_eq!(buddy.blockers.len(), buddy2.blockers.len());
    }
}
