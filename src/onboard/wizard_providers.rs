// onboard/wizard_providers.rs — Provider helpers (model lists, fetching, caching)

use crate::providers::{
    canonical_china_provider_name, is_glm_alias, is_glm_cn_alias, is_minimax_alias,
    is_moonshot_alias, is_qianfan_alias, is_qwen_alias, is_qwen_oauth_alias, is_zai_alias,
    is_zai_cn_alias,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;

pub const LIVE_MODEL_MAX_OPTIONS: usize = 120;
pub const MODEL_CACHE_FILE: &str = "models_cache.json";
pub const MODEL_CACHE_TTL_SECS: u64 = 12 * 60 * 60;
pub const CUSTOM_MODEL_SENTINEL: &str = "__custom_model__";

pub fn canonical_provider_name(provider_name: &str) -> &str {
    if is_qwen_oauth_alias(provider_name) {
        return "qwen-code";
    }
    if let Some(canonical) = canonical_china_provider_name(provider_name) {
        return canonical;
    }
    match provider_name {
        "grok" => "xai",
        "together" => "together-ai",
        "google" | "google-gemini" => "gemini",
        "github-copilot" => "copilot",
        "openai_codex" | "codex" => "openai-codex",
        "kimi_coding" | "kimi_for_coding" => "kimi-code",
        "nvidia-nim" | "build.nvidia.com" => "nvidia",
        "aws-bedrock" => "bedrock",
        "llama.cpp" => "llamacpp",
        _ => provider_name,
    }
}

pub fn default_model_for_provider(provider: &str) -> String {
    match canonical_provider_name(provider) {
        "anthropic" => "claude-sonnet-4-5-20250929".into(),
        "openai" => "gpt-5.2".into(),
        "openai-codex" => "gpt-5-codex".into(),
        "venice" => "zai-org-glm-5".into(),
        "groq" => "llama-3.3-70b-versatile".into(),
        "mistral" => "mistral-large-latest".into(),
        "deepseek" => "deepseek-chat".into(),
        "xai" => "grok-4-1-fast-reasoning".into(),
        "perplexity" => "sonar-pro".into(),
        "fireworks" => "accounts/fireworks/models/llama-v3p3-70b-instruct".into(),
        "novita" => "minimax/minimax-m2.5".into(),
        "together-ai" => "meta-llama/Llama-3.3-70B-Instruct-Turbo".into(),
        "cohere" => "command-a-03-2025".into(),
        "moonshot" => "kimi-k2.5".into(),
        "glm" | "zai" => "glm-5".into(),
        "minimax" => "MiniMax-M2.5".into(),
        "qwen" => "qwen-plus".into(),
        "qwen-code" => "qwen3-coder-plus".into(),
        "ollama" => "llama3.2".into(),
        "llamacpp" => "ggml-org/gpt-oss-20b-GGUF".into(),
        "sglang" | "vllm" | "osaurus" => "default".into(),
        "gemini" => "gemini-2.5-pro".into(),
        "kimi-code" => "kimi-for-coding".into(),
        "bedrock" => "anthropic.claude-sonnet-4-5-20250929-v1:0".into(),
        "nvidia" => "meta/llama-3.3-70b-instruct".into(),
        _ => "anthropic/claude-sonnet-4.6".into(),
    }
}

pub fn curated_models_for_provider(provider_name: &str) -> Vec<(String, String)> {
    match canonical_provider_name(provider_name) {
        "openrouter" => vec![
            ("anthropic/claude-sonnet-4.6".to_string(), "Claude Sonnet 4.6 (balanced, recommended)".to_string()),
            ("openai/gpt-5.2".to_string(), "GPT-5.2 (latest flagship)".to_string()),
            ("openai/gpt-5-mini".to_string(), "GPT-5 mini (fast, cost-efficient)".to_string()),
            ("google/gemini-3-pro-preview".to_string(), "Gemini 3 Pro Preview (frontier reasoning)".to_string()),
            ("x-ai/grok-4.1-fast".to_string(), "Grok 4.1 Fast (reasoning + speed)".to_string()),
            ("deepseek/deepseek-v3.2".to_string(), "DeepSeek V3.2 (agentic + affordable)".to_string()),
            ("meta-llama/llama-4-maverick".to_string(), "Llama 4 Maverick (open model)".to_string()),
        ],
        "anthropic" => vec![
            ("claude-sonnet-4-5-20250929".to_string(), "Claude Sonnet 4.5 (balanced, recommended)".to_string()),
            ("claude-opus-4-6".to_string(), "Claude Opus 4.6 (best quality)".to_string()),
            ("claude-haiku-4-5-20251001".to_string(), "Claude Haiku 4.5 (fastest, cheapest)".to_string()),
        ],
        "openai" => vec![
            ("gpt-5.2".to_string(), "GPT-5.2 (latest coding/agentic flagship)".to_string()),
            ("gpt-5-mini".to_string(), "GPT-5 mini (faster, cheaper)".to_string()),
            ("gpt-5-nano".to_string(), "GPT-5 nano (lowest latency/cost)".to_string()),
            ("gpt-5.2-codex".to_string(), "GPT-5.2 Codex (agentic coding)".to_string()),
        ],
        _ => vec![("default".to_string(), "Default model".to_string())],
    }
}

pub fn supports_live_model_fetch(provider_name: &str) -> bool {
    if provider_name.trim().starts_with("custom:") {
        return true;
    }
    matches!(
        canonical_provider_name(provider_name),
        "openrouter" | "openai-codex" | "openai" | "anthropic" | "groq" | "mistral"
            | "deepseek" | "xai" | "together-ai" | "gemini" | "ollama" | "llamacpp"
            | "sglang" | "vllm" | "osaurus" | "astrai" | "venice" | "fireworks"
            | "novita" | "cohere" | "moonshot" | "glm" | "zai" | "qwen" | "nvidia"
    )
}

pub fn allows_unauthenticated_model_fetch(provider_name: &str) -> bool {
    matches!(
        canonical_provider_name(provider_name),
        "openrouter" | "ollama" | "llamacpp" | "sglang" | "vllm" | "osaurus"
            | "venice" | "astrai" | "nvidia"
    )
}

pub fn models_endpoint_for_provider(provider_name: &str) -> Option<&'static str> {
    match canonical_provider_name(provider_name) {
        "openai-codex" | "openai" => Some("https://api.openai.com/v1/models"),
        "venice" => Some("https://api.venice.ai/api/v1/models"),
        "groq" => Some("https://api.groq.com/openai/v1/models"),
        "mistral" => Some("https://api.mistral.ai/v1/models"),
        "deepseek" => Some("https://api.deepseek.com/v1/models"),
        "xai" => Some("https://api.x.ai/v1/models"),
        "together-ai" => Some("https://api.together.xyz/v1/models"),
        "fireworks" => Some("https://api.fireworks.ai/inference/v1/models"),
        "novita" => Some("https://api.novita.ai/openai/v1/models"),
        "cohere" => Some("https://api.cohere.com/compatibility/v1/models"),
        "moonshot" => Some("https://api.moonshot.ai/v1/models"),
        "glm" => Some("https://api.z.ai/api/paas/v4/models"),
        "zai" => Some("https://api.z.ai/api/coding/paas/v4/models"),
        "qwen" => Some("https://dashscope.aliyuncs.com/compatible-mode/v1/models"),
        "nvidia" => Some("https://integrate.api.nvidia.com/v1/models"),
        "astrai" => Some("https://as-trai.com/v1/models"),
        "llamacpp" => Some("http://localhost:8080/v1/models"),
        "sglang" => Some("http://localhost:30000/v1/models"),
        "vllm" => Some("http://localhost:8000/v1/models"),
        "osaurus" => Some("http://localhost:1337/v1/models"),
        _ => None,
    }
}

pub fn provider_env_var(provider_name: &str) -> String {
    match canonical_provider_name(provider_name) {
        "openai" => "OPENAI_API_KEY".into(),
        "anthropic" => "ANTHROPIC_API_KEY".into(),
        "openrouter" => "OPENROUTER_API_KEY".into(),
        "groq" => "GROQ_API_KEY".into(),
        "mistral" => "MISTRAL_API_KEY".into(),
        "deepseek" => "DEEPSEEK_API_KEY".into(),
        "xai" => "XAI_API_KEY".into(),
        "perplexity" => "PERPLEXITY_API_KEY".into(),
        "fireworks" => "FIREWORKS_API_KEY".into(),
        "novita" => "NOVITA_API_KEY".into(),
        "together-ai" => "TOGETHER_API_KEY".into(),
        "cohere" => "COHERE_API_KEY".into(),
        "moonshot" => "MOONSHOT_API_KEY".into(),
        "glm" => "GLM_API_KEY".into(),
        "zai" => "ZAI_API_KEY".into(),
        "minimax" => "MINIMAX_API_KEY".into(),
        "qwen" => "DASHSCOPE_API_KEY".into(),
        "gemini" => "GEMINI_API_KEY".into(),
        "nvidia" => "NVIDIA_API_KEY".into(),
        "astrai" => "ASTRAI_API_KEY".into(),
        "bedrock" => "AWS_ACCESS_KEY_ID".into(),
        _ => format!("{}_API_KEY", provider_name.to_uppercase()),
    }
}

pub fn provider_supports_keyless_local_usage(provider_name: &str) -> bool {
    matches!(canonical_provider_name(provider_name), "ollama" | "llamacpp" | "sglang" | "vllm" | "osaurus")
}

pub fn provider_supports_device_flow(provider_name: &str) -> bool {
    matches!(canonical_provider_name(provider_name), "copilot" | "gemini" | "qwen-code")
}

pub fn local_provider_choices() -> Vec<(&'static str, &'static str)> {
    vec![
        ("ollama", "Ollama — local model runner (recommended)"),
        ("llamacpp", "llama.cpp server — GGUF models"),
        ("sglang", "SGLang — high-throughput serving"),
        ("vllm", "vLLM — high-throughput LLM serving"),
        ("osaurus", "Osaurus — macOS native LLM runner"),
    ]
}

// ── Model fetching ──────────────────────────────────────────────

fn build_model_fetch_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(4))
        .build()
        .context("failed to build model-fetch HTTP client")
}

fn normalize_model_ids(ids: Vec<String>) -> Vec<String> {
    let mut unique = BTreeMap::new();
    for id in ids {
        let trimmed = id.trim();
        if !trimmed.is_empty() {
            unique.entry(trimmed.to_ascii_lowercase()).or_insert_with(|| trimmed.to_string());
        }
    }
    unique.into_values().collect()
}

fn parse_openai_compatible_model_ids(payload: &Value) -> Vec<String> {
    let mut models = Vec::new();
    if let Some(data) = payload.get("data").and_then(Value::as_array) {
        for model in data {
            if let Some(id) = model.get("id").and_then(Value::as_str) {
                models.push(id.to_string());
            }
        }
    } else if let Some(data) = payload.as_array() {
        for model in data {
            if let Some(id) = model.get("id").and_then(Value::as_str) {
                models.push(id.to_string());
            }
        }
    }
    normalize_model_ids(models)
}

fn parse_gemini_model_ids(payload: &Value) -> Vec<String> {
    let Some(models) = payload.get("models").and_then(Value::as_array) else { return Vec::new(); };
    let mut ids = Vec::new();
    for model in models {
        let supports = model.get("supportedGenerationMethods").and_then(Value::as_array)
            .is_none_or(|methods| methods.iter().any(|m| m.as_str() == Some("generateContent")));
        if !supports { continue; }
        if let Some(name) = model.get("name").and_then(Value::as_str) {
            ids.push(name.trim_start_matches("models/").to_string());
        }
    }
    normalize_model_ids(ids)
}

fn parse_ollama_model_ids(payload: &Value) -> Vec<String> {
    let Some(models) = payload.get("models").and_then(Value::as_array) else { return Vec::new(); };
    let mut ids = Vec::new();
    for model in models {
        if let Some(name) = model.get("name").and_then(Value::as_str) {
            ids.push(name.to_string());
        }
    }
    normalize_model_ids(ids)
}

pub fn fetch_openai_compatible_models(endpoint: &str, api_key: Option<&str>, allow_unauthenticated: bool) -> Result<Vec<String>> {
    let client = build_model_fetch_client()?;
    let mut request = client.get(endpoint);
    if let Some(api_key) = api_key {
        request = request.bearer_auth(api_key);
    } else if !allow_unauthenticated {
        bail!("model fetch requires API key for endpoint {endpoint}");
    }
    let payload: Value = request.send().and_then(reqwest::blocking::Response::error_for_status)
        .with_context(|| format!("model fetch failed: GET {endpoint}"))?
        .json().context("failed to parse model list response")?;
    Ok(parse_openai_compatible_model_ids(&payload))
}

pub fn fetch_openrouter_models(api_key: Option<&str>) -> Result<Vec<String>> {
    let client = build_model_fetch_client()?;
    let mut request = client.get("https://openrouter.ai/api/v1/models");
    if let Some(api_key) = api_key { request = request.bearer_auth(api_key); }
    let payload: Value = request.send().and_then(reqwest::blocking::Response::error_for_status)
        .context("model fetch failed: GET https://openrouter.ai/api/v1/models")?
        .json().context("failed to parse OpenRouter model list response")?;
    Ok(parse_openai_compatible_model_ids(&payload))
}

pub fn fetch_anthropic_models(api_key: Option<&str>) -> Result<Vec<String>> {
    let Some(api_key) = api_key else { bail!("Anthropic model fetch requires API key or OAuth token"); };
    let client = build_model_fetch_client()?;
    let mut request = client.get("https://api.anthropic.com/v1/models").header("anthropic-version", "2023-06-01");
    if api_key.starts_with("sk-ant-oat01-") {
        request = request.header("Authorization", format!("Bearer {api_key}")).header("anthropic-beta", "oauth-2025-04-20");
    } else {
        request = request.header("x-api-key", api_key);
    }
    let response = request.send().context("model fetch failed: GET https://api.anthropic.com/v1/models")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        bail!("Anthropic model list request failed (HTTP {status}): {body}");
    }
    let payload: Value = response.json().context("failed to parse Anthropic model list response")?;
    Ok(parse_openai_compatible_model_ids(&payload))
}

pub fn fetch_gemini_models(api_key: Option<&str>) -> Result<Vec<String>> {
    let Some(api_key) = api_key else { bail!("Gemini model fetch requires API key"); };
    let client = build_model_fetch_client()?;
    let payload: Value = client.get("https://generativelanguage.googleapis.com/v1beta/models")
        .query(&[("key", api_key), ("pageSize", "200")])
        .send().and_then(reqwest::blocking::Response::error_for_status)
        .context("model fetch failed: GET Gemini models")?
        .json().context("failed to parse Gemini model list response")?;
    Ok(parse_gemini_model_ids(&payload))
}

pub fn fetch_ollama_models() -> Result<Vec<String>> {
    let client = build_model_fetch_client()?;
    let payload: Value = client.get("http://localhost:11434/api/tags")
        .send().and_then(reqwest::blocking::Response::error_for_status)
        .context("model fetch failed: GET http://localhost:11434/api/tags")?
        .json().context("failed to parse Ollama model list response")?;
    Ok(parse_ollama_model_ids(&payload))
}

pub fn normalize_ollama_endpoint_url(raw_url: &str) -> String {
    let trimmed = raw_url.trim().trim_end_matches('/');
    if trimmed.is_empty() { return String::new(); }
    trimmed.strip_suffix("/api").unwrap_or(trimmed).trim_end_matches('/').to_string()
}

pub fn ollama_endpoint_is_local(endpoint_url: &str) -> bool {
    reqwest::Url::parse(endpoint_url).ok()
        .and_then(|url| url.host_str().map(|host| host.to_ascii_lowercase()))
        .is_some_and(|host| matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1" | "0.0.0.0"))
}

pub fn ollama_uses_remote_endpoint(provider_api_url: Option<&str>) -> bool {
    let Some(endpoint) = provider_api_url else { return false; };
    let normalized = normalize_ollama_endpoint_url(endpoint);
    if normalized.is_empty() { return false; }
    !ollama_endpoint_is_local(&normalized)
}

pub fn resolve_live_models_endpoint(provider_name: &str, provider_api_url: Option<&str>) -> Option<String> {
    if let Some(raw_base) = provider_name.strip_prefix("custom:") {
        let normalized = raw_base.trim().trim_end_matches('/');
        if normalized.is_empty() { return None; }
        if normalized.ends_with("/models") { return Some(normalized.to_string()); }
        return Some(format!("{normalized}/models"));
    }
    if matches!(canonical_provider_name(provider_name), "llamacpp" | "sglang" | "vllm" | "osaurus") {
        if let Some(url) = provider_api_url.map(str::trim).filter(|url| !url.is_empty()) {
            let normalized = url.trim_end_matches('/');
            if normalized.ends_with("/models") { return Some(normalized.to_string()); }
            return Some(format!("{normalized}/models"));
        }
    }
    if canonical_provider_name(provider_name) == "openai-codex" {
        if let Some(url) = provider_api_url.map(str::trim).filter(|url| !url.is_empty()) {
            let normalized = url.trim_end_matches('/');
            if normalized.ends_with("/models") { return Some(normalized.to_string()); }
            return Some(format!("{normalized}/models"));
        }
    }
    models_endpoint_for_provider(provider_name).map(str::to_string)
}

pub fn fetch_live_models_for_provider(provider_name: &str, api_key: &str, provider_api_url: Option<&str>) -> Result<Vec<String>> {
    let requested_provider_name = provider_name;
    let provider_name = canonical_provider_name(provider_name);
    let ollama_remote = provider_name == "ollama" && ollama_uses_remote_endpoint(provider_api_url);
    let api_key = if api_key.trim().is_empty() {
        if provider_name == "ollama" && !ollama_remote {
            None
        } else {
            std::env::var(provider_env_var(provider_name)).ok()
                .or_else(|| {
                    if provider_name == "anthropic" { std::env::var("ANTHROPIC_OAUTH_TOKEN").ok() }
                    else if provider_name == "minimax" { std::env::var("MINIMAX_OAUTH_TOKEN").ok() }
                    else { None }
                })
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        }
    } else {
        Some(api_key.trim().to_string())
    };

    match provider_name {
        "openrouter" => fetch_openrouter_models(api_key.as_deref()),
        "anthropic" => fetch_anthropic_models(api_key.as_deref()),
        "gemini" => fetch_gemini_models(api_key.as_deref()),
        "ollama" => {
            if ollama_remote {
                Ok(vec![
                    "glm-5:cloud".into(), "glm-4.7:cloud".into(), "gpt-oss:20b:cloud".into(),
                    "gpt-oss:120b:cloud".into(), "gemini-3-flash-preview:cloud".into(),
                    "qwen3-coder-next:cloud".into(), "qwen3-coder:480b:cloud".into(),
                    "kimi-k2.5:cloud".into(), "minimax-m2.5:cloud".into(), "deepseek-v3.1:671b:cloud".into(),
                ])
            } else {
                Ok(fetch_ollama_models()?.into_iter().filter(|m| !m.ends_with(":cloud")).collect())
            }
        }
        _ => {
            if let Some(endpoint) = resolve_live_models_endpoint(requested_provider_name, provider_api_url) {
                let allow_unauth = allows_unauthenticated_model_fetch(requested_provider_name);
                fetch_openai_compatible_models(&endpoint, api_key.as_deref(), allow_unauth)
            } else {
                Ok(Vec::new())
            }
        }
    }
}

// ── Model caching ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCacheEntry {
    pub provider: String,
    pub fetched_at_unix: u64,
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCacheState {
    pub entries: Vec<ModelCacheEntry>,
}

#[derive(Debug, Clone)]
pub struct CachedModels {
    pub models: Vec<String>,
    pub age_secs: u64,
}

pub fn model_cache_path(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("state").join(MODEL_CACHE_FILE)
}

pub fn now_unix_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

pub fn humanize_age(age_secs: u64) -> String {
    if age_secs < 60 { format!("{age_secs}s") }
    else if age_secs < 60 * 60 { format!("{}m", age_secs / 60) }
    else { format!("{}h", age_secs / (60 * 60)) }
}

pub async fn load_model_cache_state(workspace_dir: &Path) -> Result<ModelCacheState> {
    let path = model_cache_path(workspace_dir);
    if !path.exists() { return Ok(ModelCacheState::default()); }
    let raw = fs::read_to_string(&path).await.with_context(|| format!("failed to read model cache at {}", path.display()))?;
    match serde_json::from_str::<ModelCacheState>(&raw) {
        Ok(state) => Ok(state),
        Err(_) => Ok(ModelCacheState::default()),
    }
}

pub async fn save_model_cache_state(workspace_dir: &Path, state: &ModelCacheState) -> Result<()> {
    let path = model_cache_path(workspace_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.with_context(|| format!("failed to create model cache directory {}", parent.display()))?;
    }
    let json = serde_json::to_vec_pretty(state).context("failed to serialize model cache")?;
    fs::write(&path, json).await.with_context(|| format!("failed to write model cache at {}", path.display()))?;
    Ok(())
}

pub async fn cache_live_models_for_provider(workspace_dir: &Path, provider_name: &str, models: &[String]) -> Result<()> {
    let normalized_models = normalize_model_ids(models.to_vec());
    if normalized_models.is_empty() { return Ok(()); }
    let mut state = load_model_cache_state(workspace_dir).await?;
    let now = now_unix_secs();
    if let Some(entry) = state.entries.iter_mut().find(|e| e.provider == provider_name) {
        entry.fetched_at_unix = now;
        entry.models = normalized_models;
    } else {
        state.entries.push(ModelCacheEntry { provider: provider_name.to_string(), fetched_at_unix: now, models: normalized_models });
    }
    save_model_cache_state(workspace_dir, &state).await
}

pub async fn load_cached_models_for_provider(workspace_dir: &Path, provider_name: &str, ttl_secs: u64) -> Result<Option<CachedModels>> {
    load_cached_models_for_provider_internal(workspace_dir, provider_name, Some(ttl_secs)).await
}

pub async fn load_any_cached_models_for_provider(workspace_dir: &Path, provider_name: &str) -> Result<Option<CachedModels>> {
    load_cached_models_for_provider_internal(workspace_dir, provider_name, None).await
}

async fn load_cached_models_for_provider_internal(workspace_dir: &Path, provider_name: &str, ttl_secs: Option<u64>) -> Result<Option<CachedModels>> {
    let state = load_model_cache_state(workspace_dir).await?;
    let now = now_unix_secs();
    let Some(entry) = state.entries.into_iter().find(|e| e.provider == provider_name) else { return Ok(None); };
    if entry.models.is_empty() { return Ok(None); }
    let age_secs = now.saturating_sub(entry.fetched_at_unix);
    if ttl_secs.is_some_and(|ttl| age_secs > ttl) { return Ok(None); }
    Ok(Some(CachedModels { models: entry.models, age_secs }))
}
