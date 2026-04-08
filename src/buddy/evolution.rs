//! Evolution Manager — Ties Logger + PatternExtractor together
//!
//! Reads conversation logs, extracts patterns, generates evolution reports,
//! and saves version snapshots for tracking Buddy's growth over time.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;

use super::logger::ConversationLogger;
use super::patterns::{top_patterns, Pattern, PatternExtractor};
use super::state::BuddyState;

/// Report generated from analyzing conversation history
#[derive(Debug, Clone)]
pub struct EvolutionReport {
    /// Number of conversation turns analyzed
    pub conversations_analyzed: usize,
    /// All patterns extracted from the logs
    pub patterns: Vec<Pattern>,
    /// The single best pattern by success rate
    pub top_pattern: Option<Pattern>,
    /// Human-readable recommendations based on patterns
    pub recommendations: Vec<String>,
    /// When this report was generated
    pub timestamp: DateTime<Utc>,
}

/// Manages conversation analysis and evolution tracking
pub struct EvolutionManager {
    logger: ConversationLogger,
}

impl EvolutionManager {
    /// Create a new EvolutionManager with the default logger
    pub fn new() -> Self {
        Self {
            logger: ConversationLogger::new(),
        }
    }

    /// Create an EvolutionManager with a specific logger
    pub fn with_logger(logger: ConversationLogger) -> Self {
        Self { logger }
    }

    /// Run daily analysis: read all logs, extract patterns, generate a report
    pub fn daily_analysis(&self) -> Result<EvolutionReport> {
        let turns = self
            .logger
            .read_all()
            .context("failed to read conversation logs")?;

        let conversations_analyzed = turns.len();
        let patterns = PatternExtractor::analyze(&turns);

        // Top pattern is the one with highest success_rate
        let top_pattern = top_patterns(&patterns, 1).into_iter().next();

        // Generate recommendations from patterns
        let recommendations = Self::generate_recommendations(&patterns);

        Ok(EvolutionReport {
            conversations_analyzed,
            patterns,
            top_pattern,
            recommendations,
            timestamp: Utc::now(),
        })
    }

    /// Save a JSON snapshot of BuddyState to a custom directory
    pub fn save_snapshot_to(&self, state: &BuddyState, dir: &Path) -> Result<()> {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create snapshot dir: {}", dir.display()))?;

        let filename = format!(
            "snapshot_{}.json",
            Utc::now().format("%Y%m%d_%H%M%S")
        );
        let path = dir.join(&filename);

        let json = serde_json::to_string_pretty(state)
            .context("failed to serialize BuddyState")?;

        fs::write(&path, json)
            .with_context(|| format!("failed to write snapshot: {}", path.display()))?;

        Ok(())
    }

    /// Save a JSON snapshot of BuddyState to ~/.buddy/evolution/
    pub fn save_snapshot(&self, state: &BuddyState) -> Result<()> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let snapshot_dir = Path::new(&home).join(".buddy").join("evolution");
        self.save_snapshot_to(state, &snapshot_dir)
    }

    /// Pretty-print an EvolutionReport for CLI display
    pub fn format_report(report: &EvolutionReport) -> String {
        let mut out = String::new();

        out.push_str("=== Evolution Report ===\n");
        out.push_str(&format!(
            "Generated: {}\n",
            report.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        out.push_str(&format!(
            "Conversations analyzed: {}\n\n",
            report.conversations_analyzed
        ));

        // Top pattern highlight
        if let Some(ref top) = report.top_pattern {
            out.push_str(&format!(
                "Top pattern: [{}] success={:.0}% ({} turns)\n\n",
                top.trigger,
                top.success_rate * 100.0,
                top.count,
            ));
        }

        // All patterns
        if !report.patterns.is_empty() {
            out.push_str("Patterns:\n");
            for (i, p) in report.patterns.iter().enumerate() {
                out.push_str(&format!(
                    "  {:>2}. [{}] success={:.0}% ({} turns)\n",
                    i + 1,
                    p.trigger,
                    p.success_rate * 100.0,
                    p.count,
                ));
            }
            out.push('\n');
        }

        // Recommendations
        if !report.recommendations.is_empty() {
            out.push_str("Recommendations:\n");
            for rec in &report.recommendations {
                out.push_str(&format!("  * {}\n", rec));
            }
        }

        if report.conversations_analyzed == 0 {
            out.push_str("No conversation data yet. Start logging to see patterns.\n");
        }

        out
    }

    /// Generate recommendations based on extracted patterns
    fn generate_recommendations(patterns: &[Pattern]) -> Vec<String> {
        let mut recs = Vec::new();

        for p in patterns {
            if p.count < 2 {
                continue; // need at least 2 observations
            }

            let pct = (p.success_rate * 100.0).round() as u32;

            if p.success_rate >= 0.8 {
                recs.push(format!(
                    "{} works {}% of the time — keep doing it",
                    p.trigger, pct
                ));
            } else if p.success_rate >= 0.5 {
                recs.push(format!(
                    "{} has {}% success — room for improvement",
                    p.trigger, pct
                ));
            } else {
                recs.push(format!(
                    "{} only succeeds {}% — consider changing approach",
                    p.trigger, pct
                ));
            }
        }

        if recs.is_empty() && !patterns.is_empty() {
            recs.push("Not enough data for recommendations yet. Keep logging!".into());
        }

        recs
    }
}

impl Default for EvolutionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buddy::logger::ConversationOutcome;
    use crate::buddy::{EmotionalState, Situation};
    use tempfile::TempDir;

    #[test]
    fn test_evolution_manager_new() {
        let manager = EvolutionManager::new();
        // Just verify it constructs without panic
        let report = manager.daily_analysis().unwrap();
        assert_eq!(report.conversations_analyzed, 0);
    }

    #[test]
    fn test_daily_analysis_empty_logs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("empty.jsonl");
        let logger = ConversationLogger::with_path(path);
        let manager = EvolutionManager::with_logger(logger);

        let report = manager.daily_analysis().unwrap();
        assert_eq!(report.conversations_analyzed, 0);
        assert!(report.patterns.is_empty());
        assert!(report.top_pattern.is_none());
        assert!(report.recommendations.is_empty());
        // timestamp should be recent
        let diff = Utc::now() - report.timestamp;
        assert!(diff.num_seconds() < 5);
    }

    #[test]
    fn test_daily_analysis_with_data() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("analysis.jsonl");
        let logger = ConversationLogger::with_path(path);

        // Log some turns
        use crate::buddy::logger::ConversationTurn;
        for _ in 0..3 {
            let turn = ConversationTurn {
                timestamp: Utc::now(),
                user_input: "Hej, co dziś robimy?".into(),
                buddy_response: "Plan na dziś...".into(),
                emotion: EmotionalState::Neutral,
                situation: Situation::NewDay,
                outcome: ConversationOutcome::Completed,
            };
            logger.log(&turn).unwrap();
        }

        let manager = EvolutionManager::with_logger(logger);
        let report = manager.daily_analysis().unwrap();

        assert_eq!(report.conversations_analyzed, 3);
        assert!(!report.patterns.is_empty());
        assert!(report.top_pattern.is_some());
        let top = report.top_pattern.unwrap();
        assert_eq!(top.trigger, "morning_planning");
        assert_eq!(top.success_rate, 1.0);
    }

    #[test]
    fn test_format_report_not_empty() {
        let report = EvolutionReport {
            conversations_analyzed: 5,
            patterns: vec![
                Pattern {
                    trigger: "morning_planning".into(),
                    response: "Plan!".into(),
                    success_rate: 0.85,
                    count: 12,
                },
            ],
            top_pattern: Some(Pattern {
                trigger: "morning_planning".into(),
                response: "Plan!".into(),
                success_rate: 0.85,
                count: 12,
            }),
            recommendations: vec!["morning_planning works 85% — keep doing it".into()],
            timestamp: Utc::now(),
        };

        let formatted = EvolutionManager::format_report(&report);
        assert!(!formatted.is_empty());
        assert!(formatted.contains("Evolution Report"));
        assert!(formatted.contains("5"));
        assert!(formatted.contains("morning_planning"));
        assert!(formatted.contains("85%"));
        assert!(formatted.contains("Recommendations"));
    }

    #[test]
    fn test_format_report_empty() {
        let report = EvolutionReport {
            conversations_analyzed: 0,
            patterns: vec![],
            top_pattern: None,
            recommendations: vec![],
            timestamp: Utc::now(),
        };

        let formatted = EvolutionManager::format_report(&report);
        assert!(formatted.contains("No conversation data yet"));
    }

    #[test]
    fn test_save_snapshot_creates_file() {
        let dir = TempDir::new().unwrap();
        let state = BuddyState::new();

        let manager = EvolutionManager::new();
        manager.save_snapshot_to(&state, dir.path()).unwrap();

        // Verify a snapshot file was created
        let entries: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1);
        let path = entries[0].as_ref().unwrap().path();
        assert!(path.to_string_lossy().ends_with(".json"));

        let content = fs::read_to_string(&path).unwrap();
        let deserialized: BuddyState = serde_json::from_str(&content).unwrap();
        assert_eq!(deserialized.emotion, state.emotion);
    }

    #[test]
    fn test_generate_recommendations() {
        let patterns = vec![
            Pattern {
                trigger: "morning_planning".into(),
                response: "x".into(),
                success_rate: 0.85,
                count: 5,
            },
            Pattern {
                trigger: "help_request".into(),
                response: "x".into(),
                success_rate: 0.3,
                count: 3,
            },
            Pattern {
                trigger: "rare".into(),
                response: "x".into(),
                success_rate: 1.0,
                count: 1, // too few, skipped
            },
        ];

        let recs = EvolutionManager::generate_recommendations(&patterns);
        assert_eq!(recs.len(), 2);
        assert!(recs[0].contains("85%"));
        assert!(recs[0].contains("keep doing it"));
        assert!(recs[1].contains("30%"));
        assert!(recs[1].contains("changing approach"));
    }

    #[test]
    fn test_default_is_same_as_new() {
        let m1 = EvolutionManager::new();
        let m2 = EvolutionManager::default();
        let r1 = m1.daily_analysis().unwrap();
        let r2 = m2.daily_analysis().unwrap();
        assert_eq!(r1.conversations_analyzed, r2.conversations_analyzed);
    }
}
