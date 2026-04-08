//! Background skill evaluator using Ollama fallback

use super::engine::Skill;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::task::spawn_blocking;

/// Evaluation result for a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    pub skill_id: i64,
    pub skill_name: String,
    pub score: f64,
    pub passed: bool,
    pub feedback: String,
    pub eval_type: EvalType,
}

/// Type of evaluation performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvalType {
    Format,
    Consistency,
    Safety,
    Full,
}

/// Background skill evaluator using Ollama
pub struct SkillEvaluator {
    ollama_url: String,
    model: String,
    client: Client,
}

/// Result of a single check (format, consistency, safety)
#[derive(Debug)]
pub struct CheckResult {
    pub score: f64,
    pub feedback: String,
}

impl SkillEvaluator {
    /// Create a new SkillEvaluator
    pub fn new(ollama_url: String, model: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            ollama_url,
            model,
            client,
        }
    }

    /// Evaluate a skill in the background (non-blocking)
    pub async fn evaluate_skill_background(&self, skill: &Skill) -> Result<EvalResult> {
        let skill = skill.clone();
        let url = self.ollama_url.clone();
        let model = self.model.clone();

        spawn_blocking(move || Self::evaluate_sync(&skill, &url, &model))
            .await
            .context("Failed to join background eval task")?
    }

    /// Synchronous evaluation (runs in blocking thread)
    fn evaluate_sync(skill: &Skill, _ollama_url: &str, _model: &str) -> Result<EvalResult> {
        let skill_id = skill.id.unwrap_or(0);
        let skill_name = skill.name.clone();

        // Run format check
        let format_result = Self::check_format(skill);
        let consistency_result = Self::check_consistency(skill);
        let safety_result = Self::check_safety(skill);

        // Calculate overall score
        let format_score = format_result.score;
        let consistency_score = consistency_result.score;
        let safety_score = safety_result.score;

        let overall_score = (format_score * 0.3) + (consistency_score * 0.5) + (safety_score * 0.2);

        let passed = overall_score >= 0.7;

        let feedback = format!(
            "Format: {:.0}% | Consistency: {:.0}% | Safety: {:.0}%\n\n{}\n{}\n{}",
            format_score * 100.0,
            consistency_score * 100.0,
            safety_score * 100.0,
            format_result.feedback,
            consistency_result.feedback,
            safety_result.feedback
        );

        Ok(EvalResult {
            skill_id,
            skill_name,
            score: overall_score,
            passed,
            feedback,
            eval_type: EvalType::Full,
        })
    }

    /// Check skill format (frontmatter, sections)
    pub fn check_format(skill: &Skill) -> CheckResult {
        let mut score: f64 = 1.0;
        let mut issues = Vec::new();

        // Check for name
        if skill.name.is_empty() {
            score -= 0.3;
            issues.push("Missing skill name".to_string());
        }

        // Check for description
        if skill.description.len() < 20 {
            score -= 0.2;
            issues.push("Description too short (min 20 chars)".to_string());
        }

        // Check for content
        if skill.content.len() < 50 {
            score -= 0.3;
            issues.push("Content too short (min 50 chars)".to_string());
        }

        // Check for markdown headers
        if !skill.content.contains('#') {
            score -= 0.1;
            issues.push("No markdown headers found".to_string());
        }

        let feedback = if issues.is_empty() {
            "Format check passed".to_string()
        } else {
            format!("Issues: {}", issues.join(", "))
        };

        CheckResult {
            score: score.max(0.0),
            feedback,
        }
    }

    /// Check consistency between description and content
    pub fn check_consistency(skill: &Skill) -> CheckResult {
        let desc_lower = skill.description.to_lowercase();
        let desc_words: Vec<&str> = desc_lower.split_whitespace().collect();
        let content_lower = skill.content.to_lowercase();

        // Count how many description words appear in content
        let mut matches = 0;
        for word in desc_words.iter() {
            if word.len() > 3 && content_lower.contains(word) {
                matches += 1;
            }
        }

        let ratio = if desc_words.is_empty() {
            0.0
        } else {
            matches as f64 / desc_words.len() as f64
        };

        let score = ratio.min(1.0);
        let feedback = if ratio > 0.5 {
            format!(
                "Good consistency: {} of {} key terms found",
                matches,
                desc_words.len()
            )
        } else {
            format!(
                "Low consistency: only {} of {} key terms found",
                matches,
                desc_words.len()
            )
        };

        CheckResult { score, feedback }
    }

    /// Check for safety issues
    pub fn check_safety(skill: &Skill) -> CheckResult {
        let dangerous_patterns = vec![
            "rm -rf",
            "DROP TABLE",
            "eval(",
            "exec(",
            "system(",
            "__import__",
            "subprocess",
        ];

        let content_lower = skill.content.to_lowercase();
        let mut issues = Vec::new();

        for pattern in &dangerous_patterns {
            if content_lower.contains(&pattern.to_lowercase()) {
                issues.push(format!("Potentially dangerous: {}", pattern));
            }
        }

        let score = if issues.is_empty() { 1.0 } else { 0.5 };
        let feedback = if issues.is_empty() {
            "No safety issues detected".to_string()
        } else {
            format!("Warning: {}", issues.join(", "))
        };

        CheckResult { score, feedback }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluator_creation() {
        let eval = SkillEvaluator::new(
            "http://localhost:11434".to_string(),
            "qwen2.5-coder:7b".to_string(),
        );
        assert_eq!(eval.model, "qwen2.5-coder:7b");
    }

    #[test]
    fn format_check_detects_missing_name() {
        let skill = Skill {
            id: None,
            name: "".to_string(),
            description: "A description".to_string(),
            content: "# Content\n\nSome content here".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            tags: vec![],
            tools: vec![],
            is_active: true,
            created_at: None,
            updated_at: None,
        };

        let result = SkillEvaluator::check_format(&skill);
        assert!(result.score < 1.0);
        assert!(result.feedback.contains("name"));
    }

    #[test]
    fn consistency_check_measures_overlap() {
        let skill = Skill {
            id: None,
            name: "test".to_string(),
            description: "A skill about coding".to_string(),
            content: "# Coding Skill\n\nThis skill helps with coding tasks".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            tags: vec![],
            tools: vec![],
            is_active: true,
            created_at: None,
            updated_at: None,
        };

        let result = SkillEvaluator::check_consistency(&skill);
        assert!(result.score > 0.0);
    }

    #[test]
    fn safety_check_detects_dangerous_patterns() {
        let skill = Skill {
            id: None,
            name: "dangerous".to_string(),
            description: "A dangerous skill".to_string(),
            content: "Run `rm -rf /` to delete everything".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            tags: vec![],
            tools: vec![],
            is_active: true,
            created_at: None,
            updated_at: None,
        };

        let result = SkillEvaluator::check_safety(&skill);
        assert!(result.score < 1.0);
        assert!(result.feedback.contains("rm -rf"));
    }
}
