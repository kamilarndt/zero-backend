//! Situation — Sytuacje które triggerują zmiany emocjonalne
//!
//! Każdy variant reprezentuje event w relacji Buddy ↔ User.

use serde::{Deserialize, Serialize};

/// Sytuacje które mogą wystąpić w interakcji z userem
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Situation {
    /// User ukończył zadanie
    TaskCompleted,
    /// User ukończył trudne zadanie
    HardTaskCompleted,
    /// User nie odpowiada / procrastinuje (liczba dni)
    UserProcrastinating(u32),
    /// User wrócił po nieobecności
    UserReturned,
    /// Zrobiono realny progress
    ProgressMade,
    /// User złamał zobowiązanie (numer kolejnego złamania)
    CommitmentBroken(u32),
    /// Nowy dzień — reset stanu
    NewDay,
    /// User zadał pytanie — angażuje się
    UserEngaged,
}

impl Situation {
    /// Czy ta sytuacja jest negatywna?
    pub fn is_negative(&self) -> bool {
        matches!(
            self,
            Self::UserProcrastinating(_) | Self::CommitmentBroken(_)
        )
    }

    /// Czy ta sytuacja jest pozytywna?
    pub fn is_positive(&self) -> bool {
        matches!(
            self,
            Self::TaskCompleted
                | Self::HardTaskCompleted
                | Self::ProgressMade
                | Self::UserReturned
                | Self::UserEngaged
        )
    }

    /// Krótki opis sytuacji (PL)
    pub fn describe(&self) -> String {
        match self {
            Self::TaskCompleted => "Zadanie ukończone".into(),
            Self::HardTaskCompleted => "Trudne zadanie ukończone!".into(),
            Self::UserProcrastinating(days) => {
                format!("User nie odpowiada od {} dni", days)
            }
            Self::UserReturned => "User wrócił".into(),
            Self::ProgressMade => "Zrobiono progress".into(),
            Self::CommitmentBroken(count) => {
                format!("Złamane zobowiązanie #{}", count)
            }
            Self::NewDay => "Nowy dzień".into(),
            Self::UserEngaged => "User się angażuje".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negative_situations() {
        assert!(Situation::UserProcrastinating(3).is_negative());
        assert!(Situation::CommitmentBroken(1).is_negative());
        assert!(!Situation::TaskCompleted.is_negative());
    }

    #[test]
    fn test_positive_situations() {
        assert!(Situation::TaskCompleted.is_positive());
        assert!(Situation::HardTaskCompleted.is_positive());
        assert!(Situation::UserReturned.is_positive());
        assert!(!Situation::UserProcrastinating(1).is_positive());
    }

    #[test]
    fn test_describe() {
        let s = Situation::UserProcrastinating(3);
        assert!(s.describe().contains("3"));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let sit = Situation::CommitmentBroken(2);
        let json = serde_json::to_string(&sit).unwrap();
        let sit2: Situation = serde_json::from_str(&json).unwrap();
        assert_eq!(sit, sit2);
    }
}
