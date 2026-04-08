//! Pattern Extractor — Analyze conversation logs for successful patterns
//!
//! Extracts patterns from conversation history: which input types lead to
//! successful outcomes, and what Buddy's responses look like for each trigger.

use super::logger::{ConversationOutcome, ConversationTurn};

/// A pattern discovered in conversation history
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    /// Input category trigger (e.g. "morning_planning", "task_completion")
    pub trigger: String,
    /// What Buddy said in response
    pub response: String,
    /// Success rate as a float between 0.0 and 1.0
    pub success_rate: f32,
    /// How many times this pattern was observed
    pub count: usize,
}

/// Analyzer that extracts patterns from conversation turns
pub struct PatternExtractor;

impl PatternExtractor {
    /// Analyze a slice of conversation turns and extract patterns grouped by input category
    pub fn analyze(turns: &[ConversationTurn]) -> Vec<Pattern> {
        // Group turns by classified input category
        let mut groups: std::collections::HashMap<String, Vec<&ConversationTurn>> =
            std::collections::HashMap::new();

        for turn in turns {
            let category = Self::classify_input(&turn.user_input);
            groups
                .entry(category.to_string())
                .or_default()
                .push(turn);
        }

        // Build patterns from each group
        let mut patterns = Vec::new();

        for (trigger, group_turns) in groups {
            let count = group_turns.len();
            let success_count = group_turns
                .iter()
                .filter(|t| Self::is_success(&t.outcome))
                .count();

            let success_rate = if count > 0 {
                success_count as f32 / count as f32
            } else {
                0.0
            };

            // Use the most recent buddy response as representative
            let response = group_turns
                .last()
                .map(|t| t.buddy_response.clone())
                .unwrap_or_default();

            patterns.push(Pattern {
                trigger,
                response,
                success_rate,
                count,
            });
        }

        // Sort by count descending for deterministic output
        patterns.sort_by(|a, b| b.count.cmp(&a.count));

        patterns
    }

    /// Classify a user input string into a category
    pub fn classify_input(input: &str) -> &'static str {
        let lower = input.to_lowercase();

        // morning_planning: co dziś, co robimy, plan, hej, cześć
        if lower.contains("co dziś")
            || lower.contains("co robimy")
            || lower.contains("plan")
            || lower.contains("hej")
            || lower.contains("cześć")
        {
            return "morning_planning";
        }

        // task_completion: zrobiłem, done, gotowe, skończone, zrobione
        if lower.contains("zrobiłem")
            || lower.contains("done")
            || lower.contains("gotowe")
            || lower.contains("skończone")
            || lower.contains("zrobione")
        {
            return "task_completion";
        }

        // help_request: pomocy, help, nie wiem, stuck, utknąłem
        if lower.contains("pomocy")
            || lower.contains("help")
            || lower.contains("nie wiem")
            || lower.contains("stuck")
            || lower.contains("utknąłem")
        {
            return "help_request";
        }

        // status_query: jak leci, jak tam, co słychać, status
        if lower.contains("jak leci")
            || lower.contains("jak tam")
            || lower.contains("co słychać")
            || lower.contains("status")
        {
            return "status_query";
        }

        // general: everything else
        "general"
    }

    /// Check if a conversation outcome counts as success
    fn is_success(outcome: &ConversationOutcome) -> bool {
        matches!(
            outcome,
            ConversationOutcome::Completed | ConversationOutcome::Continued
        )
    }
}

/// Return the top N patterns sorted by success_rate (descending), then by count as tiebreaker
pub fn top_patterns(patterns: &[Pattern], n: usize) -> Vec<Pattern> {
    let mut sorted: Vec<Pattern> = patterns.to_vec();
    sorted.sort_by(|a, b| {
        b.success_rate
            .partial_cmp(&a.success_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.count.cmp(&a.count))
    });
    sorted.into_iter().take(n).collect()
}

/// Pretty-print patterns for CLI display
pub fn format_patterns(patterns: &[Pattern]) -> String {
    if patterns.is_empty() {
        return "No patterns found.".to_string();
    }

    let mut out = String::from("=== Conversation Patterns ===\n");

    for (i, p) in patterns.iter().enumerate() {
        out.push_str(&format!(
            "{:>3}. [{}] success={:.0}% ({} turns)\n",
            i + 1,
            p.trigger,
            p.success_rate * 100.0,
            p.count,
        ));
        // Truncate long responses for readability (char-based for UTF-8 safety)
        let preview = if p.response.chars().count() > 80 {
            let truncated: String = p.response.chars().take(77).collect();
            format!("{}...", truncated)
        } else {
            p.response.clone()
        };
        if !preview.is_empty() {
            out.push_str(&format!("     last: \"{}\"\n", preview));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buddy::{EmotionalState, Situation};
    use chrono::Utc;

    fn make_turn(input: &str, response: &str, outcome: ConversationOutcome) -> ConversationTurn {
        ConversationTurn {
            timestamp: Utc::now(),
            user_input: input.to_string(),
            buddy_response: response.to_string(),
            emotion: EmotionalState::Neutral,
            situation: Situation::NewDay,
            outcome,
        }
    }

    #[test]
    fn test_classify_morning_planning() {
        assert_eq!(PatternExtractor::classify_input("Hej, co dziś robimy?"), "morning_planning");
        assert_eq!(PatternExtractor::classify_input("Mam plan na dziś"), "morning_planning");
        assert_eq!(PatternExtractor::classify_input("Cześć Buddy!"), "morning_planning");
    }

    #[test]
    fn test_classify_task_completion() {
        assert_eq!(PatternExtractor::classify_input("Zrobiłem to zadanie"), "task_completion");
        assert_eq!(PatternExtractor::classify_input("Done!"), "task_completion");
        assert_eq!(PatternExtractor::classify_input("Wszystko gotowe"), "task_completion");
        assert_eq!(PatternExtractor::classify_input("Skończone, zrobione"), "task_completion");
    }

    #[test]
    fn test_classify_general() {
        assert_eq!(PatternExtractor::classify_input("Co myślisz o tym?"), "general");
        assert_eq!(PatternExtractor::classify_input("Lorem ipsum"), "general");
        assert_eq!(PatternExtractor::classify_input(""), "general");
    }

    #[test]
    fn test_analyze_empty() {
        let turns: Vec<ConversationTurn> = vec![];
        let patterns = PatternExtractor::analyze(&turns);
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_analyze_with_data() {
        let turns = vec![
            make_turn("Hej, co dziś?", "Plan na dziś...", ConversationOutcome::Completed),
            make_turn("Zrobiłem task", "Super!", ConversationOutcome::Completed),
            make_turn("Zrobiłem kolejny", "Świetnie!", ConversationOutcome::Continued),
            make_turn("Nie wiem co dalej", "Podpowiem...", ConversationOutcome::Abandoned),
        ];

        let patterns = PatternExtractor::analyze(&turns);

        // Should have 3 categories
        assert_eq!(patterns.len(), 3);

        let task = patterns.iter().find(|p| p.trigger == "task_completion").unwrap();
        assert_eq!(task.count, 2);
        assert_eq!(task.success_rate, 1.0);

        let morning = patterns.iter().find(|p| p.trigger == "morning_planning").unwrap();
        assert_eq!(morning.count, 1);
        assert_eq!(morning.success_rate, 1.0);

        let help = patterns.iter().find(|p| p.trigger == "help_request").unwrap();
        assert_eq!(help.count, 1);
        assert_eq!(help.success_rate, 0.0);
    }

    #[test]
    fn test_top_patterns() {
        let patterns = vec![
            Pattern { trigger: "a".into(), response: "r1".into(), success_rate: 0.5, count: 10 },
            Pattern { trigger: "b".into(), response: "r2".into(), success_rate: 0.9, count: 3 },
            Pattern { trigger: "c".into(), response: "r3".into(), success_rate: 0.9, count: 7 },
            Pattern { trigger: "d".into(), response: "r4".into(), success_rate: 0.1, count: 20 },
        ];

        let top = top_patterns(&patterns, 2);
        assert_eq!(top.len(), 2);
        // c has 0.9 success and 7 count, should be first (higher count tiebreaker)
        assert_eq!(top[0].trigger, "c");
        assert_eq!(top[1].trigger, "b");
    }

    #[test]
    fn test_format_patterns_not_empty() {
        let patterns = vec![
            Pattern {
                trigger: "morning_planning".into(),
                response: "Oto plan na dziś!".into(),
                success_rate: 0.85,
                count: 12,
            },
            Pattern {
                trigger: "task_completion".into(),
                response: "Super robota!".into(),
                success_rate: 0.95,
                count: 20,
            },
        ];

        let formatted = format_patterns(&patterns);
        assert!(!formatted.is_empty());
        assert!(formatted.contains("morning_planning"));
        assert!(formatted.contains("task_completion"));
        assert!(formatted.contains("85%"));
        assert!(formatted.contains("95%"));
        assert!(formatted.contains("=== Conversation Patterns ==="));

        // Empty patterns
        let empty = format_patterns(&[]);
        assert_eq!(empty, "No patterns found.");
    }
}
