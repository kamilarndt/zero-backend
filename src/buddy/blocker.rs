//! Blocker — Interdependence mechanic
//!
//! Blocker reprezentuje zadanie/decyzję na którą Buddy czeka od usera.
//! Im dłużej user nie odpowiada, tym większa severity.

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// Poziom severity blokady
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BlockerSeverity {
    /// Niski — mogę poczekać
    Low,
    /// Średni — zaczynam się niepokoić
    Medium,
    /// Wysoki — naprawdę potrzebuję odpowiedzi
    High,
    /// Krytyczny — wszystko stoi
    Critical,
}

impl BlockerSeverity {
    /// Emoji dla severity
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Low => "⏳",
            Self::Medium => "🔔",
            Self::High => "⚠️",
            Self::Critical => "🚨",
        }
    }

    /// Tekstowy opis (PL)
    pub fn describe(&self) -> &'static str {
        match self {
            Self::Low => "Czekam, nie ma pośpiechu",
            Self::Medium => "Przypominam — czekam na Twoją odpowiedź",
            Self::High => "Hej, naprawdę tego potrzebuję żeby iść dalej",
            Self::Critical => "Stoimy w miejscu. Potrzebuję decyzji TERAZ.",
        }
    }
}

/// Blokada — zadanie/decyzja na którą Buddy czeka
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blocker {
    /// Co blokuje (opis)
    pub what: String,
    /// Od kiedy blokuje (timestamp unix ms)
    pub since_ms: u64,
    /// Aktualny poziom severity
    pub severity: BlockerSeverity,
    /// Po ilu godzinach severity rośnie (domyślnie 4h)
    pub escalation_hours: u32,
}

impl Blocker {
    /// Tworzy nowy blocker od teraz
    pub fn new(what: impl Into<String>) -> Self {
        let since_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            what: what.into(),
            since_ms,
            severity: BlockerSeverity::Low,
            escalation_hours: 4,
        }
    }

    /// Tworzy blocker z custom escalation time
    pub fn with_escalation(what: impl Into<String>, hours: u32) -> Self {
        let mut b = Self::new(what);
        b.escalation_hours = hours;
        b
    }

    /// Aktualizuje severity na podstawie czasu oczekiwania
    pub fn update_severity(&mut self) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let elapsed_hours = (now - self.since_ms) / (1000 * 60 * 60);
        let escalations = elapsed_hours / u64::from(self.escalation_hours);

        self.severity = match escalations {
            0 => BlockerSeverity::Low,
            1 => BlockerSeverity::Medium,
            2 => BlockerSeverity::High,
            _ => BlockerSeverity::Critical,
        };
    }

    /// Generuje response na podstawie aktualnej severity
    pub fn response(&self) -> String {
        format!(
            "{} {} — blokada: \"{}\"",
            self.severity.emoji(),
            self.severity.describe(),
            self.what
        )
    }

    /// Czas oczekiwania jako Duration
    pub fn waiting_duration(&self) -> Duration {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Duration::from_millis(now - self.since_ms)
    }

    /// Ile godzin czekamy?
    pub fn waiting_hours(&self) -> f64 {
        self.waiting_duration().as_secs_f64() / 3600.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_blocker_has_low_severity() {
        let b = Blocker::new("Decyzja o architekturze");
        assert_eq!(b.severity, BlockerSeverity::Low);
        assert!(b.what.contains("architekturze"));
    }

    #[test]
    fn test_escalation_custom_hours() {
        let b = Blocker::with_escalation("Test", 2);
        assert_eq!(b.escalation_hours, 2);
    }

    #[test]
    fn test_response_contains_emoji() {
        let b = Blocker::new("Test blocker");
        let r = b.response();
        assert!(r.contains("⏳"));
        assert!(r.contains("Test blocker"));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(BlockerSeverity::Low < BlockerSeverity::Medium);
        assert!(BlockerSeverity::Medium < BlockerSeverity::High);
        assert!(BlockerSeverity::High < BlockerSeverity::Critical);
    }

    #[test]
    fn test_severity_describe() {
        assert!(!BlockerSeverity::High.describe().is_empty());
        assert!(!BlockerSeverity::Critical.describe().is_empty());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let b = Blocker::new("Serialization test");
        let json = serde_json::to_string(&b).unwrap();
        let b2: Blocker = serde_json::from_str(&json).unwrap();
        assert_eq!(b.what, b2.what);
        assert_eq!(b.severity, b2.severity);
    }
}
