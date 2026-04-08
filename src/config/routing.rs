// Routing configuration structures for ZeroClaw's intelligent task classification and routing system
use serde::{Deserialize, Serialize};

// ── Routing Configuration ────────────────────────────────────────────────

/// Top-level routing configuration for task classification and agent routing.
///
/// This configuration enables intelligent routing of user requests to appropriate
/// models and agents based on task complexity, modality (text/vision), and custom patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Enable intelligent task routing. Default: `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Token-based thresholds for determining task complexity.
    #[serde(default)]
    pub token_thresholds: TokenThresholds,

    /// Vision/image processing configuration.
    #[serde(default)]
    pub vision: VisionConfig,

    /// Classification rules for pattern-based routing.
    #[serde(default)]
    pub classification: ClassificationConfig,

    /// Sub-agent configurations for specialized task delegation.
    #[serde(default)]
    pub subagents: Vec<SubagentConfig>,

    /// Task complexity analysis configuration.
    #[serde(default)]
    pub complexity: ComplexityConfig,
}

// ── Token Thresholds ─────────────────────────────────────────────────────

/// Token count thresholds used to determine task complexity and routing decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenThresholds {
    /// Maximum tokens for "simple" tasks (fast models). Default: `500`.
    #[serde(default = "default_simple_max_tokens")]
    pub simple_max_tokens: usize,

    /// Maximum tokens for "medium" complexity tasks. Default: `2000`.
    #[serde(default = "default_medium_max_tokens")]
    pub medium_max_tokens: usize,

    /// Minimum tokens to consider task "complex" (powerful models). Default: `2000`.
    #[serde(default = "default_complex_min_tokens")]
    pub complex_min_tokens: usize,

    /// Maximum tokens before triggering chunking/summary. Default: `8000`.
    #[serde(default = "default_chunk_threshold")]
    pub chunk_threshold: usize,
}

// ── Vision Configuration ─────────────────────────────────────────────────

/// Configuration for vision/image processing routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionConfig {
    /// Enable vision modality detection. Default: `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Keywords that indicate vision processing is needed.
    /// Default: `["image", "picture", "photo", "screenshot", "diagram", "chart"]`.
    #[serde(default = "default_vision_keywords")]
    pub keywords: Vec<String>,

    /// Minimum file size (in bytes) to trigger high-resolution vision. Default: `100000` (100KB).
    #[serde(default = "default_vision_min_size")]
    pub min_file_size_bytes: usize,

    /// Maximum image dimension (width/height) for standard vision. Default: `2048`.
    #[serde(default = "default_vision_max_dimension")]
    pub max_dimension: usize,
}

// ── Classification Configuration ─────────────────────────────────────────

/// Pattern-based classification rules for intelligent task routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationConfig {
    /// Enable pattern-based classification. Default: `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Pattern rules evaluated in priority order (higher priority first).
    #[serde(default)]
    pub rules: Vec<PatternRule>,
}

/// A single pattern matching rule for task classification.
///
/// Rules are evaluated in priority order. First matching rule determines routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternRule {
    /// Human-readable rule name.
    #[serde(default)]
    pub name: String,

    /// Routing hint to use when this rule matches.
    pub hint: String,

    /// Priority for rule evaluation (higher evaluated first). Default: `0`.
    #[serde(default)]
    pub priority: i32,

    /// Case-insensitive keyword patterns (substring matches).
    #[serde(default)]
    pub keywords: Vec<String>,

    /// Case-sensitive literal patterns (exact matches).
    #[serde(default)]
    pub patterns: Vec<String>,

    /// Regular expression patterns (stored as strings, compiled at runtime).
    #[serde(default)]
    pub regex_patterns: Vec<String>,

    /// Minimum message length to match this rule.
    #[serde(default)]
    pub min_length: Option<usize>,

    /// Maximum message length to match this rule.
    #[serde(default)]
    pub max_length: Option<usize>,

    /// Required tools for this task type (tool presence can trigger routing).
    #[serde(default)]
    pub required_tools: Vec<String>,

    /// If true, route to specialized sub-agent instead of model hint.
    #[serde(default)]
    pub delegate_to_subagent: bool,

    /// Name of sub-agent to delegate to (if delegate_to_subagent is true).
    #[serde(default)]
    pub subagent_name: Option<String>,
}

impl Default for PatternRule {
    fn default() -> Self {
        Self {
            name: String::new(),
            hint: String::new(),
            priority: 0,
            keywords: Vec::new(),
            patterns: Vec::new(),
            regex_patterns: Vec::new(),
            min_length: None,
            max_length: None,
            required_tools: Vec::new(),
            delegate_to_subagent: false,
            subagent_name: None,
        }
    }
}

// ── Subagent Configuration ───────────────────────────────────────────────

/// Configuration for specialized sub-agents that handle specific task types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentConfig {
    /// Unique identifier for this sub-agent configuration.
    pub name: String,

    /// Human-readable description of this sub-agent's purpose.
    #[serde(default)]
    pub description: String,

    /// LLM provider to use for this sub-agent.
    pub provider: String,

    /// Model to use for this sub-agent.
    pub model: String,

    /// Optional system prompt override.
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Optional API key override for this sub-agent's provider.
    #[serde(default)]
    pub api_key: Option<String>,

    /// Temperature override for this sub-agent.
    #[serde(default)]
    pub temperature: Option<f64>,

    /// Maximum recursion depth for nested delegation. Default: `3`.
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,

    /// Enable agentic mode (multi-turn tool-call loop). Default: `false`.
    #[serde(default)]
    pub agentic: bool,

    /// Tool allowlist for agentic mode.
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Maximum tool-call iterations in agentic mode. Default: `10`.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,

    /// Task hints that should route to this sub-agent.
    #[serde(default)]
    pub task_hints: Vec<String>,

    /// Auto-assign this sub-agent when these tools are requested.
    #[serde(default)]
    pub auto_assign_tools: Vec<String>,

    /// Maximum context window for this sub-agent (in tokens).
    #[serde(default)]
    pub max_context_tokens: Option<usize>,
}

// ── Complexity Configuration ─────────────────────────────────────────────

/// Configuration for analyzing task complexity and determining routing strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityConfig {
    /// Enable complexity analysis. Default: `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Minimum number of tool calls to consider task "complex". Default: `3`.
    #[serde(default = "default_min_tool_calls")]
    pub min_tool_calls_complex: usize,

    /// Minimum number of function calls in code to consider "complex". Default: `5`.
    #[serde(default = "default_min_function_calls")]
    pub min_function_calls_complex: usize,

    /// Threshold for nesting depth to consider "complex". Default: `3`.
    #[serde(default = "default_nesting_depth")]
    pub nesting_depth_threshold: usize,

    /// Keywords indicating multi-step reasoning needed.
    /// Default: `["analyze", "design", "architecture", "refactor", "optimize"]`.
    #[serde(default = "default_reasoning_keywords")]
    pub reasoning_keywords: Vec<String>,

    /// File extensions that trigger code-specialized routing.
    /// Default: `["rs", "py", "js", "ts", "go", "java", "cpp", "c"]`.
    #[serde(default = "default_code_extensions")]
    pub code_extensions: Vec<String>,
}

// ── Default Implementations ──────────────────────────────────────────────

fn default_true() -> bool {
    true
}

fn default_simple_max_tokens() -> usize {
    500
}

fn default_medium_max_tokens() -> usize {
    2000
}

fn default_complex_min_tokens() -> usize {
    2000
}

fn default_chunk_threshold() -> usize {
    8000
}

fn default_vision_keywords() -> Vec<String> {
    vec![
        "image".to_string(),
        "picture".to_string(),
        "photo".to_string(),
        "screenshot".to_string(),
        "diagram".to_string(),
        "chart".to_string(),
    ]
}

fn default_vision_min_size() -> usize {
    100_000 // 100KB
}

fn default_vision_max_dimension() -> usize {
    2048
}

fn default_max_depth() -> u32 {
    3
}

fn default_max_iterations() -> usize {
    10
}

fn default_min_tool_calls() -> usize {
    3
}

fn default_min_function_calls() -> usize {
    5
}

fn default_nesting_depth() -> usize {
    3
}

fn default_reasoning_keywords() -> Vec<String> {
    vec![
        "analyze".to_string(),
        "design".to_string(),
        "architecture".to_string(),
        "refactor".to_string(),
        "optimize".to_string(),
    ]
}

fn default_code_extensions() -> Vec<String> {
    vec![
        "rs".to_string(),
        "py".to_string(),
        "js".to_string(),
        "ts".to_string(),
        "go".to_string(),
        "java".to_string(),
        "cpp".to_string(),
        "c".to_string(),
    ]
}

fn default_min_tool_calls_complex() -> usize {
    3
}

fn default_min_function_calls_complex() -> usize {
    5
}

fn default_nesting_depth_threshold() -> usize {
    3
}

// ── Manual Default Implementations ───────────────────────────────────────

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            token_thresholds: TokenThresholds::default(),
            vision: VisionConfig::default(),
            classification: ClassificationConfig::default(),
            subagents: Vec::new(),
            complexity: ComplexityConfig::default(),
        }
    }
}

impl Default for TokenThresholds {
    fn default() -> Self {
        Self {
            simple_max_tokens: default_simple_max_tokens(),
            medium_max_tokens: default_medium_max_tokens(),
            complex_min_tokens: default_complex_min_tokens(),
            chunk_threshold: default_chunk_threshold(),
        }
    }
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            keywords: default_vision_keywords(),
            min_file_size_bytes: default_vision_min_size(),
            max_dimension: default_vision_max_dimension(),
        }
    }
}

impl Default for ClassificationConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            rules: Vec::new(),
        }
    }
}

impl Default for SubagentConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            provider: String::new(),
            model: String::new(),
            system_prompt: None,
            api_key: None,
            temperature: None,
            max_depth: default_max_depth(),
            agentic: false,
            allowed_tools: Vec::new(),
            max_iterations: default_max_iterations(),
            task_hints: Vec::new(),
            auto_assign_tools: Vec::new(),
            max_context_tokens: None,
        }
    }
}

impl Default for ComplexityConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            min_tool_calls_complex: default_min_tool_calls_complex(),
            min_function_calls_complex: default_min_function_calls_complex(),
            nesting_depth_threshold: default_nesting_depth_threshold(),
            reasoning_keywords: default_reasoning_keywords(),
            code_extensions: default_code_extensions(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_config_default_is_constructible() {
        let config = RoutingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.token_thresholds.simple_max_tokens, 500);
        assert_eq!(config.token_thresholds.medium_max_tokens, 2000);
        assert!(config.vision.enabled);
        assert!(config.classification.enabled);
        assert!(config.complexity.enabled);
    }

    #[test]
    fn token_thresholds_defaults() {
        let thresholds = TokenThresholds::default();
        assert_eq!(thresholds.simple_max_tokens, 500);
        assert_eq!(thresholds.medium_max_tokens, 2000);
        assert_eq!(thresholds.complex_min_tokens, 2000);
        assert_eq!(thresholds.chunk_threshold, 8000);
    }

    #[test]
    fn vision_config_defaults() {
        let vision = VisionConfig::default();
        assert!(vision.enabled);
        assert_eq!(vision.keywords.len(), 6);
        assert!(vision.keywords.contains(&"screenshot".to_string()));
        assert_eq!(vision.min_file_size_bytes, 100_000);
        assert_eq!(vision.max_dimension, 2048);
    }

    #[test]
    fn classification_config_defaults() {
        let classification = ClassificationConfig::default();
        assert!(classification.enabled);
        assert!(classification.rules.is_empty());
    }

    #[test]
    fn pattern_rule_serialization() {
        let rule = PatternRule {
            name: "code_task".to_string(),
            hint: "coding".to_string(),
            priority: 10,
            keywords: vec!["bug".to_string(), "refactor".to_string()],
            patterns: vec!["```".to_string()],
            regex_patterns: vec!["fn\\s+\\w+".to_string()],
            min_length: Some(10),
            max_length: Some(5000),
            required_tools: vec!["file_write".to_string()],
            delegate_to_subagent: false,
            subagent_name: None,
        };

        let toml = toml::to_string(&rule).unwrap();
        let parsed: PatternRule = toml::from_str(&toml).unwrap();
        assert_eq!(parsed.name, "code_task");
        assert_eq!(parsed.priority, 10);
        assert_eq!(parsed.keywords.len(), 2);
    }

    #[test]
    fn subagent_config_defaults() {
        let subagent = SubagentConfig {
            name: "coder".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            ..Default::default()
        };

        assert_eq!(subagent.max_depth, 3);
        assert_eq!(subagent.max_iterations, 10);
        assert!(!subagent.agentic);
        assert!(subagent.allowed_tools.is_empty());
    }

    #[test]
    fn complexity_config_defaults() {
        let complexity = ComplexityConfig::default();
        assert!(complexity.enabled);
        assert_eq!(complexity.min_tool_calls_complex, 3);
        assert_eq!(complexity.min_function_calls_complex, 5);
        assert_eq!(complexity.nesting_depth_threshold, 3);
        assert!(complexity
            .reasoning_keywords
            .contains(&"refactor".to_string()));
        assert!(complexity.code_extensions.contains(&"rs".to_string()));
    }

    #[test]
    fn full_routing_config_toml_roundtrip() {
        let config = RoutingConfig {
            enabled: true,
            token_thresholds: TokenThresholds {
                simple_max_tokens: 1000,
                ..Default::default()
            },
            vision: VisionConfig {
                keywords: vec!["image".to_string()],
                ..Default::default()
            },
            classification: ClassificationConfig {
                enabled: true,
                rules: vec![PatternRule {
                    name: "test".to_string(),
                    hint: "test_hint".to_string(),
                    ..Default::default()
                }],
            },
            subagents: vec![SubagentConfig {
                name: "test_agent".to_string(),
                provider: "test_provider".to_string(),
                model: "test_model".to_string(),
                ..Default::default()
            }],
            complexity: ComplexityConfig::default(),
        };

        let toml = toml::to_string(&config).unwrap();
        let parsed: RoutingConfig = toml::from_str(&toml).unwrap();

        assert_eq!(parsed.enabled, true);
        assert_eq!(parsed.token_thresholds.simple_max_tokens, 1000);
        assert_eq!(parsed.classification.rules.len(), 1);
        assert_eq!(parsed.subagents.len(), 1);
        assert_eq!(parsed.subagents[0].name, "test_agent");
    }
}
