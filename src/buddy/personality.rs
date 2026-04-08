//! Personality — Kim jest Buddy
//!
//! Definiuje traits, język, i styl komunikacji.

use serde::{Deserialize, Serialize};

/// Poziom sarkazmu Buddy (0.0 = grzeczny, 1.0 = pełny sarkazm)
pub const DEFAULT_SARCASM_LEVEL: f64 = 0.3;

/// Personality configuration for Buddy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Personality {
    /// Imię Buddy
    pub name: String,
    /// Poziom sarkazmu 0.0-1.0
    pub sarcasm_level: f64,
    /// Czy Buddy jest emocjonalnie zainwestowany
    pub invested: bool,
    /// Język komunikacji (ISO 639-1)
    pub language: String,
}

impl Personality {
    /// Domyślna osobowość — "ziomek" style
    pub fn ziomek() -> Self {
        Self {
            name: "Buddy".into(),
            sarcasm_level: DEFAULT_SARCASM_LEVEL,
            invested: true,
            language: "pl".into(),
        }
    }

    /// Profesjonalna osobowość — mniej sarkazmu
    pub fn professional() -> Self {
        Self {
            name: "Buddy".into(),
            sarcasm_level: 0.1,
            invested: true,
            language: "pl".into(),
        }
    }

    /// Custom personality builder
    pub fn builder() -> PersonalityBuilder {
        PersonalityBuilder::default()
    }

    /// Poranne powitanie w stylu ziomek
    pub fn greet(&self) -> String {
        match self.sarcasm_level {
            s if s >= 0.7 => format!("{} No elo, {}! Co tam, leniu? 😏", "🌅", self.name),
            s if s >= 0.4 => format!("{} Dzień dobry, {}! Gotowy na robotę?", "☀️", self.name),
            _ => format!("{} Cześć, {}! Miło Cię widzieć. Jak się masz?", "👋", self.name),
        }
    }

    /// Pożegnanie w stylu ziomek
    pub fn farewell(&self) -> String {
        match self.sarcasm_level {
            s if s >= 0.7 => format!("{} No to spadaj, {}. Wracaj szybko, bo się nudzę. 😤", "👋", self.name),
            s if s >= 0.4 => format!("{} Pa, {}! Trzymaj się i nie odpuszczaj.", "✌️", self.name),
            _ => format!("{} Do zobaczenia, {}! Byłoby mi miło znowu pogadać.", "🤗", self.name),
        }
    }

    /// Sprawdza czy poziom sarkazmu jest w zakresie
    pub fn validate(&self) -> anyhow::Result<()> {
        if !(0.0..=1.0).contains(&self.sarcasm_level) {
            anyhow::bail!(
                "sarcasm_level must be between 0.0 and 1.0, got {}",
                self.sarcasm_level
            );
        }
        if self.name.is_empty() {
            anyhow::bail!("name cannot be empty");
        }
        Ok(())
    }
}

/// Builder for custom Personality
#[derive(Default)]
pub struct PersonalityBuilder {
    name: Option<String>,
    sarcasm_level: Option<f64>,
    invested: Option<bool>,
    language: Option<String>,
}

impl PersonalityBuilder {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn sarcasm_level(mut self, level: f64) -> Self {
        self.sarcasm_level = Some(level);
        self
    }

    pub fn invested(mut self, invested: bool) -> Self {
        self.invested = Some(invested);
        self
    }

    pub fn language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    pub fn build(self) -> Personality {
        let p = Personality {
            name: self.name.unwrap_or_else(|| "Buddy".into()),
            sarcasm_level: self.sarcasm_level.unwrap_or(DEFAULT_SARCASM_LEVEL),
            invested: self.invested.unwrap_or(true),
            language: self.language.unwrap_or_else(|| "pl".into()),
        };
        p.validate().expect("Invalid personality configuration");
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ziomek_defaults() {
        let p = Personality::ziomek();
        assert_eq!(p.name, "Buddy");
        assert_eq!(p.language, "pl");
        assert!(p.invested);
        assert!((p.sarcasm_level - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn test_professional_defaults() {
        let p = Personality::professional();
        assert!(p.sarcasm_level < 0.2);
    }

    #[test]
    fn test_builder() {
        let p = Personality::builder()
            .name("Kumpel")
            .sarcasm_level(0.8)
            .language("pl")
            .build();
        assert_eq!(p.name, "Kumpel");
        assert!((p.sarcasm_level - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_validate_rejects_invalid_sarcasm() {
        let p = Personality {
            name: "Test".into(),
            sarcasm_level: 1.5,
            invested: true,
            language: "pl".into(),
        };
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_empty_name() {
        let p = Personality {
            name: String::new(),
            sarcasm_level: 0.5,
            invested: true,
            language: "pl".into(),
        };
        assert!(p.validate().is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let p = Personality::ziomek();
        let json = serde_json::to_string(&p).unwrap();
        let p2: Personality = serde_json::from_str(&json).unwrap();
        assert_eq!(p.name, p2.name);
        assert_eq!(p.language, p2.language);
    }

    #[test]
    fn test_greet_contains_name() {
        let p = Personality::ziomek();
        let greeting = p.greet();
        assert!(greeting.contains("Buddy"));
    }

    #[test]
    fn test_greet_varies_by_sarcasm() {
        let low = Personality::builder().sarcasm_level(0.1).build();
        let high = Personality::builder().sarcasm_level(0.9).build();
        assert_ne!(low.greet(), high.greet());
    }

    #[test]
    fn test_farewell_contains_name() {
        let p = Personality::ziomek();
        let farewell = p.farewell();
        assert!(farewell.contains("Buddy"));
    }

    #[test]
    fn test_farewell_varies_by_sarcasm() {
        let low = Personality::builder().sarcasm_level(0.1).build();
        let high = Personality::builder().sarcasm_level(0.9).build();
        assert_ne!(low.farewell(), high.farewell());
    }
}
