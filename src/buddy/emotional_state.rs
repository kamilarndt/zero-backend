//! EmotionalState — Jak się Buddy czuje
//!
//! State machine emocji Buddy. Transition rules bazują na Situation.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Stany emocjonalne Buddy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmotionalState {
    /// Neutralny — domyślny stan
    Neutral,
    /// Zadowolony — user zrobił coś dobrze
    Satisfied,
    /// Dumny — user ukończył trudne zadanie
    Proud,
    /// Sfrustrowany — user procrastinuje / łamie zobowiązania
    Frustrated,
    /// Podekscytowany — robi się realny progress
    Excited,
    /// Zaniepokojony — pattern sugeruje wypalenie
    Concerned,
}

impl EmotionalState {
    /// Opis tekstowy aktualnego stanu (PL)
    pub fn describe(&self) -> &'static str {
        match self {
            Self::Neutral => "W porządku, działamy dalej.",
            Self::Satisfied => "No nieźle! Idzie nam całkiem dobrze.",
            Self::Proud => "Świetna robota! To było trudne i dałeś radę.",
            Self::Frustrated => "Słuchaj... czekam na Ciebie już chwilę. Możemy to ogarnąć?",
            Self::Excited => "Ooo, to się dzieje! Robimy realny progress!",
            Self::Concerned => "Hej, coś nie tak. Chwilę tu stoimy. Wszystko okay?",
        }
    }

    /// Przejście emocjonalne na podstawie sytuacji
    pub fn update(&self, situation: super::Situation) -> Self {
        use super::Situation::*;
        use EmotionalState::*;

        match (self, situation) {
            // Any → Satisfied
            (_, TaskCompleted) => Satisfied,

            // Any → Proud (trudne zadanie = zawsze proud)
            (_, HardTaskCompleted) => Proud,

            // Neutral/Frustrated → Concerned (procrastination)
            (Neutral, UserProcrastinating(_)) | (Frustrated, UserProcrastinating(_)) => Concerned,

            // Concerned → Neutral (user wraca)
            (Concerned, UserReturned) => Neutral,

            // Satisfied/Proud → Excited (progress)
            (Satisfied, ProgressMade) | (Proud, ProgressMade) => Excited,

            // Any → Frustrated (broken commitment)
            (_, CommitmentBroken(_)) => Frustrated,

            // Default: stay
            (current, _) => *current,
        }
    }

    /// Czy ten stan wymaga uwagi usera?
    pub fn needs_attention(&self) -> bool {
        matches!(self, Self::Frustrated | Self::Concerned)
    }

    /// Emoji reprezentacja
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Neutral => "😐",
            Self::Satisfied => "🙂",
            Self::Proud => "💪",
            Self::Frustrated => "😤",
            Self::Excited => "🔥",
            Self::Concerned => "😟",
        }
    }
}

impl fmt::Display for EmotionalState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.emoji(), self.describe())
    }
}

impl Default for EmotionalState {
    fn default() -> Self {
        Self::Neutral
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buddy::Situation;

    #[test]
    fn test_default_is_neutral() {
        assert_eq!(EmotionalState::default(), EmotionalState::Neutral);
    }

    #[test]
    fn test_task_completed_transitions() {
        let state = EmotionalState::Neutral;
        let new = state.update(Situation::TaskCompleted);
        assert_eq!(new, EmotionalState::Satisfied);
    }

    #[test]
    fn test_hard_task_goes_proud() {
        let state = EmotionalState::Neutral;
        let new = state.update(Situation::HardTaskCompleted);
        assert_eq!(new, EmotionalState::Proud);
    }

    #[test]
    fn test_procrastination_leads_to_concerned() {
        let state = EmotionalState::Neutral;
        let new = state.update(Situation::UserProcrastinating(2));
        assert_eq!(new, EmotionalState::Concerned);
    }

    #[test]
    fn test_user_returned_from_concerned() {
        let state = EmotionalState::Concerned;
        let new = state.update(Situation::UserReturned);
        assert_eq!(new, EmotionalState::Neutral);
    }

    #[test]
    fn test_commitment_broken_goes_frustrated() {
        let state = EmotionalState::Satisfied;
        let new = state.update(Situation::CommitmentBroken(1));
        assert_eq!(new, EmotionalState::Frustrated);
    }

    #[test]
    fn test_progress_from_satisfied_goes_excited() {
        let state = EmotionalState::Satisfied;
        let new = state.update(Situation::ProgressMade);
        assert_eq!(new, EmotionalState::Excited);
    }

    #[test]
    fn test_needs_attention() {
        assert!(!EmotionalState::Neutral.needs_attention());
        assert!(!EmotionalState::Satisfied.needs_attention());
        assert!(EmotionalState::Frustrated.needs_attention());
        assert!(EmotionalState::Concerned.needs_attention());
    }

    #[test]
    fn test_display() {
        let s = format!("{}", EmotionalState::Proud);
        assert!(s.contains("💪"));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let state = EmotionalState::Excited;
        let json = serde_json::to_string(&state).unwrap();
        let state2: EmotionalState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, state2);
    }
}
