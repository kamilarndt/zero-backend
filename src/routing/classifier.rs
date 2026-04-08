// Task Classifier for ZeroClaw Routing System
// Implements 4-gate pipeline for intelligent task classification

use crate::config::schemas::QueryClassificationConfig;
use regex::Regex;
use std::collections::HashMap;
use std::sync::RwLock;

// ── Task Type Enumeration ─────────────────────────────────────────────

/// Classified task types for model routing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TaskType {
    /// Vision/image processing tasks
    Vision,
    /// Architecture and system design tasks
    Architecture,
    /// Code generation and modification tasks
    Coding,
    /// Code review and analysis tasks
    Review,
    /// Documentation generation tasks
    Documentation,
    /// Quick/low-latency tasks
    Quick,
    /// Standard/default tasks
    Standard,
}

impl TaskType {
    /// Returns the default model hint for this task type
    pub fn default_hint(&self) -> &'static str {
        match self {
            TaskType::Vision => "vision",
            TaskType::Architecture => "reasoning",
            TaskType::Coding => "code",
            TaskType::Review => "review",
            TaskType::Documentation => "fast",
            TaskType::Quick => "fast",
            TaskType::Standard => "standard",
        }
    }
}

// ── Classification Result ─────────────────────────────────────────────

/// Result of task classification
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    /// The classified task type
    pub task_type: TaskType,
    /// The model hint to use (String, not &str, for ownership)
    pub model: String,
    /// Estimated token count for the input
    pub estimated_tokens: usize,
    /// Whether vision content was detected
    pub has_vision: bool,
}

impl ClassificationResult {
    /// Create a new classification result
    pub fn new(
        task_type: TaskType,
        model: String,
        estimated_tokens: usize,
        has_vision: bool,
    ) -> Self {
        Self {
            task_type,
            model,
            estimated_tokens,
            has_vision,
        }
    }

    /// Create a standard default result
    pub fn standard(estimated_tokens: usize) -> Self {
        Self {
            task_type: TaskType::Standard,
            model: TaskType::Standard.default_hint().to_string(),
            estimated_tokens,
            has_vision: false,
        }
    }
}

// ── Classifier ────────────────────────────────────────────────────────

/// Task classifier with 4-gate pipeline
pub struct Classifier {
    /// Compiled regex patterns cache (using RwLock for thread safety)
    pattern_cache: RwLock<HashMap<String, Regex>>,
    /// Classification configuration
    config: QueryClassificationConfig,
}

impl Classifier {
    /// Create a new classifier with the given configuration
    pub fn new(config: QueryClassificationConfig) -> Self {
        Self {
            pattern_cache: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Create a classifier with default (disabled) configuration
    pub fn disabled() -> Self {
        Self {
            pattern_cache: RwLock::new(HashMap::new()),
            config: QueryClassificationConfig::default(),
        }
    }

    /// Get or compile a regex pattern (cached for performance)
    fn get_or_compile_pattern(&self, pattern: &str) -> Result<Regex, regex::Error> {
        // Try to read from cache first
        {
            let cache = self.pattern_cache.read().unwrap();
            if let Some(regex) = cache.get(pattern) {
                return Ok(regex.clone());
            }
        }

        // Compile and cache the pattern
        let compiled = Regex::new(pattern)?;
        let mut cache = self.pattern_cache.write().unwrap();
        cache.insert(pattern.to_string(), compiled.clone());
        Ok(compiled)
    }

    /// Gate 1: Vision Detection
    /// Detects if the input contains vision/image content
    fn gate_vision_detection(&self, text: &str, mime_types: &[String]) -> (bool, TaskType) {
        // Check MIME types for image content
        let has_image_mime = mime_types.iter().any(|mime| {
            mime.starts_with("image/")
                || mime == "image/png"
                || mime == "image/jpeg"
                || mime == "image/webp"
                || mime == "image/gif"
                || mime == "image/bmp"
        });

        // Check text for image markers
        let has_image_markers = text.contains("[IMAGE:") || text.contains("[image:");

        // Check for common image file extensions
        let image_extensions = [".png", ".jpg", ".jpeg", ".webp", ".gif", ".bmp", ".svg"];
        let has_image_extension = image_extensions
            .iter()
            .any(|ext| text.to_lowercase().contains(ext));

        let has_vision = has_image_mime || has_image_markers || has_image_extension;

        (has_vision, TaskType::Vision)
    }

    /// Gate 2: Token Estimation and Thresholds
    /// Estimates token count and applies token-based overrides
    fn gate_token_estimation(&self, text: &str) -> usize {
        // Token estimation: 3.5 characters per token (standard approximation)
        ((text.len() as f64) / 3.5).ceil() as usize
    }

    /// Gate 3: Pattern Matching
    /// Matches patterns and keywords against classification rules
    fn gate_pattern_matching(&self, text: &str, _estimated_tokens: usize) -> Option<TaskType> {
        if !self.config.enabled {
            return None;
        }

        // Sort rules by priority (highest first)
        let mut sorted_rules = self.config.rules.clone();
        sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in &sorted_rules {
            // Check min/max length constraints
            if let Some(min_len) = rule.min_length {
                if text.len() < min_len {
                    continue;
                }
            }

            if let Some(max_len) = rule.max_length {
                if text.len() > max_len {
                    continue;
                }
            }

            let lower_text = text.to_lowercase();

            // Check keyword matches (case-insensitive)
            let keyword_match = rule
                .keywords
                .iter()
                .any(|keyword| lower_text.contains(&keyword.to_lowercase()));

            // Check pattern matches (case-sensitive, using regex)
            let pattern_match = rule.patterns.iter().any(|pattern| {
                if let Ok(regex) = self.get_or_compile_pattern(pattern) {
                    regex.is_match(text)
                } else {
                    // Fallback to literal match if regex compilation fails
                    text.contains(pattern)
                }
            });

            // If any pattern or keyword matches, map hint to task type
            if keyword_match || pattern_match {
                return self.map_hint_to_task_type(&rule.hint);
            }
        }

        None
    }

    /// Gate 4: Default Fallback
    /// Returns Standard task type when no other gate matches
    fn gate_default(&self) -> TaskType {
        TaskType::Standard
    }

    /// Map a model hint to a TaskType
    fn map_hint_to_task_type(&self, hint: &str) -> Option<TaskType> {
        let hint_lower = hint.to_lowercase();

        match hint_lower.as_str() {
            "vision" | "image" => Some(TaskType::Vision),
            "reasoning" | "architecture" | "arch" => Some(TaskType::Architecture),
            "code" | "coding" | "program" => Some(TaskType::Coding),
            "review" | "audit" | "inspect" => Some(TaskType::Review),
            "documentation" | "docs" | "doc" => Some(TaskType::Documentation),
            "fast" | "quick" | "speed" => Some(TaskType::Quick),
            "standard" | "default" => Some(TaskType::Standard),
            _ => None,
        }
    }

    /// Main classification entry point
    /// Implements the 4-gate pipeline:
    /// 1. Vision detection (MIME types)
    /// 2. Token estimation
    /// 3. Pattern matching (regex from config)
    /// 4. Default to Standard
    ///
    /// Never returns an error: unknown input defaults to Standard
    pub fn classify(&self, input: &ClassificationInput) -> ClassificationResult {
        // Estimate tokens first (Gate 2)
        let estimated_tokens = self.gate_token_estimation(&input.text);

        // Gate 1: Vision detection
        let (has_vision, vision_type) = self.gate_vision_detection(&input.text, &input.mime_types);

        if has_vision {
            return ClassificationResult {
                task_type: vision_type.clone(),
                model: vision_type.default_hint().to_string(),
                estimated_tokens,
                has_vision: true,
            };
        }

        // Gate 3: Pattern matching
        if let Some(task_type) = self.gate_pattern_matching(&input.text, estimated_tokens) {
            return ClassificationResult {
                task_type: task_type.clone(),
                model: task_type.default_hint().to_string(),
                estimated_tokens,
                has_vision: false,
            };
        }

        // Gate 4: Default fallback
        ClassificationResult::standard(estimated_tokens)
    }
}

// ── Classification Input ───────────────────────────────────────────────

/// Input data for task classification
#[derive(Debug, Clone, Default)]
pub struct ClassificationInput {
    /// The text content to classify
    pub text: String,
    /// MIME types of any attached content (e.g., "image/png")
    pub mime_types: Vec<String>,
}

impl ClassificationInput {
    /// Create a new classification input with text only
    pub fn text_only(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            mime_types: Vec::new(),
        }
    }

    /// Create a new classification input with text and MIME types
    pub fn with_mime_types(text: impl Into<String>, mime_types: Vec<String>) -> Self {
        Self {
            text: text.into(),
            mime_types,
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schemas::llm_schema::ClassificationRule;

    fn test_classifier() -> Classifier {
        let mut config = QueryClassificationConfig::default();
        config.enabled = true;
        config.rules = vec![
            ClassificationRule {
                hint: "code".to_string(),
                keywords: vec![
                    "function".to_string(),
                    "class".to_string(),
                    "impl".to_string(),
                ],
                patterns: vec!["fn ".to_string(), "```rust".to_string()],
                min_length: Some(10),
                max_length: None,
                priority: 10,
            },
            ClassificationRule {
                hint: "docs".to_string(),
                keywords: vec!["document".to_string(), "readme".to_string()],
                patterns: vec!["# ".to_string()],
                min_length: None,
                max_length: None,
                priority: 5,
            },
        ];
        Classifier::new(config)
    }

    #[test]
    fn test_token_estimation() {
        let classifier = test_classifier();
        let text = "Hello, world!"; // 13 chars
        let tokens = classifier.gate_token_estimation(text);
        assert_eq!(tokens, 4); // 13 / 3.5 = 3.71 -> ceil = 4
    }

    #[test]
    fn test_vision_detection_mime_types() {
        let classifier = test_classifier();
        let text = "Process this image";
        let mime_types = vec!["image/png".to_string()];

        let (has_vision, task_type) = classifier.gate_vision_detection(text, &mime_types);
        assert!(has_vision);
        assert_eq!(task_type, TaskType::Vision);
    }

    #[test]
    fn test_vision_detection_markers() {
        let classifier = test_classifier();
        let text = "Look at this: [IMAGE:screenshot.png]";
        let mime_types = vec![];

        let (has_vision, task_type) = classifier.gate_vision_detection(text, &mime_types);
        assert!(has_vision);
        assert_eq!(task_type, TaskType::Vision);
    }

    #[test]
    fn test_pattern_matching_code() {
        let classifier = test_classifier();
        let text = "Write a function to sort an array";
        let tokens = 50;

        let task_type = classifier.gate_pattern_matching(text, tokens);
        assert_eq!(task_type, Some(TaskType::Coding));
    }

    #[test]
    fn test_pattern_matching_docs() {
        let classifier = test_classifier();
        let text = "# Project README\n\nThis is documentation";
        let tokens = 100;

        let task_type = classifier.gate_pattern_matching(text, tokens);
        assert_eq!(task_type, Some(TaskType::Documentation));
    }

    #[test]
    fn test_classification_default() {
        let classifier = test_classifier();
        let input = ClassificationInput::text_only("Hello, how are you?");

        let result = classifier.classify(&input);
        assert_eq!(result.task_type, TaskType::Standard);
        assert!(!result.has_vision);
    }

    #[test]
    fn test_classification_vision() {
        let classifier = test_classifier();
        let input = ClassificationInput::with_mime_types(
            "Analyze this screenshot",
            vec!["image/jpeg".to_string()],
        );

        let result = classifier.classify(&input);
        assert_eq!(result.task_type, TaskType::Vision);
        assert!(result.has_vision);
    }

    #[test]
    fn test_classification_code() {
        let classifier = test_classifier();
        let input = ClassificationInput::text_only("Create a function for authentication");

        let result = classifier.classify(&input);
        assert_eq!(result.task_type, TaskType::Coding);
        assert!(!result.has_vision);
    }

    #[test]
    fn test_disabled_classifier_returns_standard() {
        let classifier = Classifier::disabled();
        let input = ClassificationInput::text_only("Create a function");

        let result = classifier.classify(&input);
        assert_eq!(result.task_type, TaskType::Standard);
    }
}
