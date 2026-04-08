//! Vector-based skill loader for dynamic prompt injection

use super::engine::{Skill, SkillsEngine};
use async_trait::async_trait;
use std::sync::Arc;

/// Trait for loading skills based on user queries
#[async_trait]
pub trait SkillLoader: Send + Sync {
    /// Load skills that match the query (vector search)
    async fn load_matching_skills(
        &self,
        query: &str,
        threshold: f64,
    ) -> Result<Vec<Skill>, anyhow::Error>;

    /// Enrich system prompt with matching skills
    async fn enrich_system_prompt(
        &self,
        query: &str,
        base_prompt: &str,
    ) -> Result<String, anyhow::Error>;
}

/// Vector-based skill loader using Qdrant similarity search
pub struct VectorSkillLoader {
    engine: Arc<SkillsEngine>,
    threshold: f64,
}

impl VectorSkillLoader {
    /// Create a new VectorSkillLoader
    pub fn new(engine: Arc<SkillsEngine>, threshold: f64) -> Self {
        Self { engine, threshold }
    }

    /// Format a skill for injection into system prompt
    fn format_skill_for_prompt(skill: &Skill) -> String {
        format!(
            "## 🎯 {}\n\n**Description:** {}\n\n{}",
            skill.name, skill.description, skill.content
        )
    }
}

#[async_trait]
impl SkillLoader for VectorSkillLoader {
    async fn load_matching_skills(
        &self,
        query: &str,
        threshold: f64,
    ) -> Result<Vec<Skill>, anyhow::Error> {
        let results = self.engine.search_skills(query, threshold).await?;
        Ok(results.into_iter().map(|r| r.skill).collect())
    }

    async fn enrich_system_prompt(
        &self,
        query: &str,
        base_prompt: &str,
    ) -> Result<String, anyhow::Error> {
        let skills = self.load_matching_skills(query, self.threshold).await?;

        if skills.is_empty() {
            return Ok(base_prompt.to_string());
        }

        let skills_context = skills
            .iter()
            .map(Self::format_skill_for_prompt)
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        Ok(format!(
            "{}\n\n# 🎯 Active Skills\nThe following skills have been activated based on your request:\n\n{}\n\n---\n",
            base_prompt, skills_context
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_skill_includes_all_fields() {
        let skill = Skill {
            id: Some(1),
            name: "test-skill".to_string(),
            description: "A test skill".to_string(),
            content: "Do this thing.".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            tags: vec![],
            tools: vec![],
            is_active: true,
            created_at: None,
            updated_at: None,
        };

        let formatted = VectorSkillLoader::format_skill_for_prompt(&skill);
        assert!(formatted.contains("test-skill"));
        assert!(formatted.contains("A test skill"));
        assert!(formatted.contains("Do this thing"));
    }
}
