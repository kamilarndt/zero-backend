# ZeroClaw Backend Architecture

> **Version:** 0.4.0
> **Language:** Rust (Edition 2021)
> **Architecture Pattern:** Trait-Based Modular Design

---

## Overview

ZeroClaw is a zero-overhead AI assistant backend built entirely in Rust. The architecture is built around **trait-based abstractions** that enable pluggable providers, channels, memory backends, and security sandboxes.

```
┌─────────────────────────────────────────────────────────────────┐
│                         Gateway Layer                            │
│  (OpenAI-Compatible API + Multimodal + SSE + TMA Auth)          │
└───────────────────────────┬─────────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────────┐
│                   Intelligent Routing                             │
│  (Classifier → RateAwareRouter → SubAgentManager)               │
└───────────────────────────┬─────────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────────┐
│                        Agent Loop                                │
│  (A2A Swarm • Tool Execution • Response Generation)             │
└───┬───────────┬───────────┬───────────┬───────────┬────────────┘
    │           │           │           │           │
┌───▼────┐ ┌───▼────┐ ┌───▼────┐ ┌───▼────┐ ┌────▼─────┐
│Provider│ │Channel │ │ Memory │ │Security│ │  Tools   │
│  Trait │ │  Trait │ │  Trait │ │  Trait │ │ Registry │
└────────┘ └────────┘ └───┬────┘ └────────┘ └─────┬─────┘
                           │                      │
                  ┌──────────▼──────────┐        │
                  │   Skills v2.0       │        │
                  │ (VectorSkillLoader) │        │
                  └─────────────────────┘        │
                           │                      │
                  ┌──────────▼─────────────────────▼─────┐
                  │      SOP Engine • Qdrant Memory    │
                  └──────────────────────────────────────┘
```

---

## Core Modules

### 1. Provider Layer (`src/providers/`)

**Core Trait:** `Provider` in `src/providers/traits.rs`

The provider layer abstracts LLM API interactions. All LLM providers implement the same async interface.

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn capabilities(&self) -> ProviderCapabilities;
    async fn chat_with_system(&self, system: Option<&str>, message: &str, model: &str, temperature: f64) -> anyhow::Result<String>;
    async fn chat_with_tools(&self, messages: &[ChatMessage], tools: &[serde_json::Value], model: &str, temperature: f64) -> anyhow::Result<ChatResponse>;
    fn supports_streaming(&self) -> bool;
    // ... streaming methods
}
```

**Capabilities:**
- `native_tool_calling`: API-native function calling support
- `vision`: Multimodal image input support

**Implemented Providers:**
| Provider | Native Tools | Vision | Streaming |
|----------|-------------|--------|-----------|
| Anthropic | ✅ | ✅ | ✅ |
| OpenAI | ✅ | ✅ | ✅ |
| Gemini | ✅ | ✅ | ✅ |
| GLM | ✅ | ❌ | ✅ |
| Ollama | ✅ | ✅ | ✅ |
| Bedrock | ✅ | ✅ | ✅ |
| OpenRouter | ✅ | ✅ | ✅ |
| Copilot | ✅ | ❌ | ❌ |

**Key Types:**
- `ChatMessage`: Role-based message (system/user/assistant/tool)
- `ChatResponse`: Text + tool_calls + usage + reasoning_content
- `ToolCall`: LLM-requested tool invocation
- `StreamChunk`: Streaming response delta

---

### 2. Channel Layer (`src/channels/`)

**Core Trait:** `Channel` in `src/channels/traits.rs`

The channel layer abstracts messaging platforms. ZeroClaw can communicate through any platform implementing this trait.

```rust
#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()>;
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()>;
    async fn health_check(&self) -> bool;
    async fn start_typing(&self, recipient: &str) -> anyhow::Result<()>;
    async fn stop_typing(&self, recipient: &str) -> anyhow::Result<()>;

    // Draft updates (progressive rendering)
    fn supports_draft_updates(&self) -> bool;
    async fn send_draft(&self, message: &SendMessage) -> anyhow::Result<Option<String>>;
    async fn update_draft(&self, recipient: &str, message_id: &str, text: &str) -> anyhow::Result<()>;
    async fn finalize_draft(&self, recipient: &str, message_id: &str, text: &str) -> anyhow::Result<()>;

    // Reactions
    async fn add_reaction(&self, channel_id: &str, message_id: &str, emoji: &str) -> anyhow::Result<()>;
    async fn remove_reaction(&self, channel_id: &str, message_id: &str, emoji: &str) -> anyhow::Result<()>;
}
```

**Implemented Channels:**
- **CLI**: Interactive terminal interface
- **Telegram**: Full bot API with inline keyboards, threads, webhooks
- **Discord**: Slash commands, embeds, reactions
- **Slack**: Webhooks, threaded conversations
- **WhatsApp**: Business API + WhatsApp Web native client
- **Matrix**: E2EE support
- **Email**: IMAP/SMTP
- **Signal**, **IRC**, **Mattermost**, **Nextcloud Talk**, **Nostr**, **DingTalk**, **Lark**, **iMessage**

**Key Types:**
- `ChannelMessage`: Incoming message (id, sender, content, thread_ts, active_skills)
- `SendMessage`: Outgoing message (content, recipient, thread_ts)

---

### 3. Memory Layer (`src/memory/`)

**Core Trait:** `Memory` in `src/memory/traits.rs`

The memory layer provides persistent storage with multiple backend implementations.

```rust
#[async_trait]
pub trait Memory: Send + Sync {
    fn name(&self) -> &str;
    async fn store(&self, key: &str, content: &str, category: MemoryCategory, session_id: Option<&str>) -> anyhow::Result<()>;
    async fn recall(&self, query: &str, limit: usize, session_id: Option<&str>) -> anyhow::Result<Vec<MemoryEntry>>;
    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>>;
    async fn forget(&self, key: &str) -> anyhow::Result<bool>;
    async fn count(&self) -> anyhow::Result<usize>;
    async fn health_check(&self) -> bool;
    fn as_any(&self) -> &dyn std::any::Any;  // For downcasting
}
```

**Memory Categories:**
- `Core`: Long-term facts, preferences, decisions
- `Daily`: Session logs
- `Conversation`: Chat context
- `Sop`: Standard Operating Procedures
- `Custom(String)`: User-defined categories

**Implemented Backends:**
| Backend | Description | Features |
|---------|-------------|----------|
| `SQLite` | Embedded database | Default, zero-config |
| `Qdrant` | Vector DB | Semantic search, embeddings |
| `PostgreSQL` | Relational DB | Optional feature |
| `Hybrid` | SQLite + Qdrant | Combined keyword + vector |
| `None` | No-op | Testing mode |

**Key Types:**
- `MemoryEntry`: { id, key, content, category, timestamp, session_id, score }

---

### 4. Security Layer (`src/security/`)

**Core Trait:** `Sandbox` in `src/security/traits.rs`

The security layer provides OS-level process isolation for tool execution.

```rust
#[async_trait]
pub trait Sandbox: Send + Sync {
    fn wrap_command(&self, cmd: &mut Command) -> std::io::Result<()>;
    fn is_available(&self) -> bool;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
}
```

**Sandbox Implementations:**
| Backend | Platform | Description |
|---------|----------|-------------|
| `Landlock` | Linux | Linux kernel ABI isolation |
| `Firejail` | Linux | seccomp + namespaces |
| `Bubblewrap` | Linux | user namespaces |
| `NoopSandbox` | All | No isolation (dev mode) |

**Security Components:**
- **Policy**: Domain allowlists, command restrictions
- **Prompt Guard**: Malicious input detection
- **Leak Detector**: Credential/redaction scanning
- **Pairing**: Secure device pairing (constant-time comparison)
- **OTP**: One-time password generation
- **Audit**: Security event logging

---

### 5. Gateway Layer (`src/gateway/`)

The gateway provides an HTTP API compatible with OpenAI's Chat Completions API.

**Features:**
- OpenAI-compatible `/v1/chat/completions` endpoint
- SSE streaming support
- WebSocket connections
- Telegram webhooks (TMA auth support)
- Rate limiting (sliding window)
- Request body limits (64KB)
- Request timeouts (30s)
- CORS support

**Key Files:**
- `openai_compat.rs`: OpenAI API compatibility layer
- `openai_sse_types.rs`: SSE event types
- `sse.rs`: Server-Sent Events utilities
- `tma_auth.rs`: Telegram Mini App authentication
- `ws.rs`: WebSocket handler

---

### 6. Agent Loop (`src/agent/loop_/`)

The agent loop is the core execution engine that processes messages through the LLM and executes tools.

**Flow:**
```
1. Receive message → 2. Load memory → 3. Build prompt → 4. Call LLM
                                                    ↓
5. Parse response → 6. Execute tools → 7. Store results → 8. Loop back to 4
                                                    ↓
9. Final response → 10. Send to channel
```

**Submodules:**
- `context.rs`: Conversation context management
- `execution.rs`: Tool execution with sandboxing
- `parsing.rs`: Tool call extraction from LLM responses
- `security.rs`: Credential scrubbing
- `streaming.rs`: Streaming response handling

**Key Functions:**
- `process_message()`: Main message processing entry point
- `run_tool_call_loop()`: Tool execution loop
- `build_tool_instructions()`: Prompt-guided tool calling fallback

---

### 7. Tools System (`src/tools/`)

Tools are invocable functions that the LLM can call. Each tool implements:
- Name and description
- JSON Schema for parameters
- Async execution handler

**Tool Registry:**
Uses the `inventory` crate for compile-time registration via the `inventory::submit!` macro.

**Example Tools:**
- `shell`: Execute shell commands (with sandboxing)
- `file_read`, `file_write`, `file_edit`: File operations
- `memory_store`, `memory_recall`, `memory_forget`: Memory access
- `web_search`, `web_fetch`: Web operations
- `git_operations`: Git commands
- `http_request`: HTTP client

---

### 8. Agent Swarm & A2A Communication (`src/agent/a2a.rs`)

Multi-agent orchestration system for complex task decomposition and parallel execution.

**Agent Roles:**
```rust
pub enum AgentRole {
    Planner,    // Decomposes tasks, creates dependencies
    Executor,   // Performs assigned tasks
    Reviewer,   // Validates results and quality
}
```

**A2A Message Types:**
```rust
pub enum A2AMessageType {
    TaskAssignment { task_id, instructions, dependencies },
    TaskProgress { task_id, percentage, current_step },
    TaskCompletion { task_id, result_json, artifacts },
    ClarificationRequest { task_id, question },
}
```

**Use Cases:**
- Multi-step task decomposition
- Parallel execution with dependency tracking
- Quality assurance with reviewer agents
- Hierarchical agent orchestration

---

### 9. Intelligent Routing Module (`src/routing/`)

Advanced request routing with rate-aware provider selection and cost optimization.

**Components:**

| Component | File | Purpose |
|-----------|------|---------|
| RateAwareRouter | `router.rs` | Provider selection based on rate limits |
| UsageMonitor | `usage_monitor.rs` | Real-time usage tracking & sync |
| Classifier | `classifier.rs` | Request type classification |
| SubAgentManager | `subagent.rs` | Parallel subagent delegation |

**Routing Configuration:**
```toml
[routing]
enable_monitoring = true
enable_classification = true
enable_delegation = true
fallback_threshold = 0.9  # Preemptive fallback at 90%
sync_interval_secs = 300   # Usage sync every 5 minutes
```

**Features:**
- Preemptive provider switching before rate limits
- Request classification (code/creative/analytical)
- Cost-aware routing decisions
- Parallel subagent execution (configurable depth)

---

### 10. Skills v2.0 System (`src/skills/`)

Modern skill loading with vector-based discovery and community repository integration.

**Components:**

| Component | Purpose |
|-----------|---------|
| SkillsEngine | Skill search & lifecycle management |
| VectorSkillLoader | Vector-based skill similarity search |
| SkillEvaluator | Security validation & benchmarking |

**Skill Manifest (SKILL.toml):**
```toml
[skill]
name = "my_custom_skill"
description = "Performs specialized task"
version = "0.1.0"
tags = ["automation", "productivity"]

[[tools]]
name = "analyze_data"
kind = "http"
command = "https://api.example.com/analyze"
description = "Analyzes data via external API"

[[prompts]]
"You are an expert data analyst. Use the analyze_data tool..."
```

**Open-Skills Integration:**
- Repository: https://github.com/besoeasy/open-skills
- Auto-sync every 7 days
- Configurable directory: `~/.zeroclaw/open-skills/`

---

### 11. Qdrant Vector Memory (`src/memory/qdrant.rs`)

Semantic search with vector embeddings for intelligent memory retrieval.

**Features:**
- REST API integration with Qdrant
- Pluggable embedding providers
- Lazy collection initialization
- Support for Qdrant Cloud (API key auth)

**Embedding Providers:**
| Provider | Models |
|----------|--------|
| OpenAI | text-embedding-3-small/large |
| Ollama | nomic-embed-text, mxbai-embed-large |
| Cohere | embed-english-v3.0 |

**Configuration:**
```toml
[memory.qdrant]
url = "http://localhost:6333"
collection = "zeroclaw_memories"
api_key = "optional-key"  # For Qdrant Cloud
embedder = "openai"       # or "ollama", "cohere"
```

---

### 12. SOP Workflows (`src/sop/`)

Standard Operating Procedure engine for workflow automation with approval gates.

**Components:**
- `engine.rs` - SOP execution state machine
- `types.rs` - SOP definitions (gates, conditions)
- `dispatch.rs` - Request routing to SOP handlers
- `gates.rs` - Conditional execution (auto/manual)
- `audit.rs` - Execution audit trail

**SOP Tools:**
- `sop_execute` - Run a SOP workflow
- `sop_status` - Check execution state
- `sop_approve` - Approve a manual gate
- `sop_list` - List available SOPs

**SOP Definition:**
```toml
[name]
sop_name = "Deployment Checklist"

[[steps]]
name = "Run tests"
command = "cargo test"
gate = "manual"  # Requires approval

[[steps]]
name = "Build release"
command = "cargo build --release"
gate = "auto"    # Automatic
```

---

### 13. Multimodal Gateway (`src/gateway/openai_compat.rs`)

Image and multimodal content support in OpenAI-compatible API.

**Message Format:**
```rust
pub struct ChatCompletionsMessage {
    pub role: String,
    pub content: String,
    pub image_urls: Vec<String>,  // base64 data URLs
    // ... other fields
}
```

**Features:**
- Base64 image data URL support
- Extracted from multimodal content (tldraw agent, etc.)
- Automatic vision provider routing

---

### 14. Hooks System (`src/hooks/`)

Event-based extension system for runtime customization without code changes.

**Core Trait:** `HookHandler` in `src/hooks/traits.rs`

```rust
#[async_trait]
pub trait HookHandler: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> i32 { 0 }

    // Void hooks (parallel, fire-and-forget)
    async fn on_gateway_start(&self, _host: &str, _port: u16) {}
    async fn on_session_start(&self, _session_id: &str, _channel: &str) {}
    async fn on_llm_input(&self, _messages: &[ChatMessage], _model: &str) {}
    async fn on_llm_output(&self, _response: &ChatResponse) {}
    async fn on_after_tool_call(&self, _tool: &str, _result: &ToolResult, _duration: Duration) {}

    // Modifying hooks (sequential by priority, can cancel)
    async fn before_model_resolve(&self, provider: String, model: String) -> HookResult<(String, String)> {
        HookResult::Continue((provider, model))
    }
    async fn before_prompt_build(&self, prompt: String) -> HookResult<String> {
        HookResult::Continue(prompt)
    }
}
```

**Hook Types:**
- **Void hooks**: Fire-and-forget, run in parallel
- **Modifying hooks**: Sequential by priority, can modify or cancel operations

**Built-in Hooks:**
- `command_logger`: Logs all shell commands
- Custom hooks via `~/.zeroclaw/hooks/`

---

### 15. Observability (`src/observability/`)

Monitoring and tracing system for production visibility.

**Observer Events:**
```rust
pub enum ObserverEvent {
    AgentStart { provider, model },
    LlmRequest { provider, model, messages_count },
    LlmResponse { provider, model, duration, success, input_tokens, output_tokens },
    AgentEnd { provider, model, duration, tokens_used, cost_usd },
    ToolCallStart { tool },
    ToolCall { tool, duration, success },
    TurnComplete,
    ChannelMessage { channel, direction },
    HeartbeatTick,
}
```

**Backends:**
- `noop` - No-op (default)
- `verbose` - Console logging
- `log` - Structured logging
- `prometheus` - Prometheus metrics
- `otel` - OpenTelemetry traces
- `multi` - Multiple observers

---

### 16. Runtime Adapter (`src/runtime/`)

Platform abstraction for porting ZeroClaw to different environments.

**Core Trait:** `RuntimeAdapter` in `src/runtime/traits.rs`

```rust
pub trait RuntimeAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn has_shell_access(&self) -> bool;
    fn has_filesystem_access(&self) -> bool;
    fn storage_path(&self) -> PathBuf;
    fn supports_long_running(&self) -> bool;
    fn memory_budget(&self) -> u64;
}
```

**Implemented Runtimes:**
| Runtime | Shell | FS | Long-Running | Description |
|---------|-------|-------|--------------|-------------|
| Native | ✅ | ✅ | ✅ | Standard Linux/macOS/Windows |
| Docker | ✅ | ✅ | ✅ | Container environment |
| WASM | ❌ | ❌ | ❌ | Browser/Edge (limited) |

---

### 17. Cron/Tasks (`src/cron/`)

Scheduled task execution system.

**Features:**
- Cron expression scheduling (5-field format)
- One-shot scheduled tasks (RFC3339 timestamp)
- Fixed-interval recurring tasks
- Delayed one-shot tasks ("30m", "2h", "1d")
- Job types: `Shell`, `Agent`
- Session targets: `Isolated`, `Main`

**Configuration:**
```toml
[[jobs]]
id = "daily_report"
expression = "0 9 * * *"
timezone = "America/New_York"
job_type = "agent"
command = "Generate daily usage report"
```

**CLI Tools:**
- `cron_add`, `cron_list`, `cron_remove`
- `cron_update`, `cron_pause`, `cron_resume`
- `cron_runs` - execution history

---

### 18. Tunnel Providers (`src/tunnel/`)

Reverse tunneling for exposing local gateway publicly.

**Core Trait:** `Tunnel` in `src/tunnel/mod.rs`

```rust
#[async_trait]
pub trait Tunnel: Send + Sync {
    fn name(&self) -> &str;
    async fn start(&self, local_host: &str, local_port: u16) -> Result<String>;
    async fn stop(&self) -> Result<()>;
    async fn health_check(&self) -> bool;
    fn public_url(&self) -> Option<String>;
}
```

**Providers:**
| Provider | Binary | Description |
|----------|---------|-------------|
| Cloudflare | `cloudflared` | Cloudflare Tunnel |
| Ngrok | `ngrok` | Ngrok tunnel |
| Tailscale | `tailscale` | Tailscale funnels |
| Custom | Custom command | Arbitrary tunnel binary |

---

### 19. Cost Tracking (`src/cost/`)

Token usage and cost monitoring per provider.

**Components:**
- `tracker.rs` - Real-time cost accumulation
- `types.rs` - Cost calculation types

**Features:**
- Per-request token tracking
- Provider-specific pricing
- Session-level aggregation
- Budget alerts

---

### 20. Peripherals (`src/peripherals/`)

Hardware integration for robotics/IoT applications.

**Supported Boards:**
- Arduino Uno (via serial)
- STM32 Nucleo (via ST-Link)
- ESP32 (via serial)
- Raspberry Pi GPIO

**Tools:**
- `hardware_board_info` - Board information
- `hardware_memory_map` - Memory map
- `hardware_memory_read` - Read memory via probe-rs
- `peripheral_flash` - Flash firmware

---

### 21. Hardware Discovery (`src/hardware/`)

USB device enumeration and identification.

**Features:**
- USB VID/PID scanning
- Board type detection (STM32 Nucleo, Arduino, ESP32)
- Serial port discovery
- Chip info via probe-rs

---

### 22. Authentication (`src/auth/`)

OAuth and JWT authentication for providers.

**Components:**
- `anthropic_token.rs` - Anthropic token exchange
- `gemini_oauth.rs` - Google OAuth flow
- `openai_oauth.rs` - OpenAI OAuth flow
- `jwt.rs` - JWT utilities
- `profiles.rs` - Credential profiles

---

### 23. Onboarding (`src/onboard/`)

First-run setup wizard.

**Features:**
- Interactive TUI wizard
- Config file generation
- Provider setup
- Channel configuration
- Memory backend selection

---

### 24. Heartbeat (`src/heartbeat/`)

Keep-alive system for long-running sessions.

**Features:**
- Periodic ticks to prevent timeouts
- Session health monitoring
- Observer event emission

---

### 25. Doctor (`src/doctor/`)

Diagnostic and troubleshooting system.

**Features:**
- Health checks for all components
- Configuration validation
- Connectivity tests
- Remediation suggestions

---

### 26. Monitoring (`src/monitoring/`)

System resource monitoring.

**Features:**
- CPU usage tracking
- Memory usage tracking
- Temperature monitoring (where available)

---

### 27. Approval (`src/approval/`)

User approval workflow for sensitive operations.

**Features:**
- Interactive approval prompts
- Approval history
- Auto-approval rules

---

### 28. Integrations (`src/integrations/`)

Third-party service integrations registry.

**Features:**
- Integration discovery
- Capability querying
- Configuration management

---

### 29. SkillForge (`src/skillforge/`)

Skill scouting and integration tools.

**Components:**
- `scout.rs` - Find community skills
- `integrate.rs` - Integrate new skills
- `evaluate.rs` - Skill quality assessment

---

### 30. Health (`src/health/`)

Health check endpoints and monitoring.

---

### 31. Migration (`src/migration.rs`)

Data schema migrations and upgrades.

---

### 32. Diagnostic (`src/diagnostic.rs`)

System diagnostics and troubleshooting information.

### 14. Hooks System (`src/hooks/`)

Event-based extension system for runtime customization without code changes.

**Core Trait:** `HookHandler` in `src/hooks/traits.rs`

```rust
#[async_trait]
pub trait HookHandler: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> i32 { 0 }

    // Void hooks (parallel, fire-and-forget)
    async fn on_gateway_start(&self, _host: &str, _port: u16) {}
    async fn on_session_start(&self, _session_id: &str, _channel: &str) {}
    async fn on_llm_input(&self, _messages: &[ChatMessage], _model: &str) {}
    async fn on_llm_output(&self, _response: &ChatResponse) {}
    async fn on_after_tool_call(&self, _tool: &str, _result: &ToolResult, _duration: Duration) {}

    // Modifying hooks (sequential by priority, can cancel)
    async fn before_model_resolve(&self, provider: String, model: String) -> HookResult<(String, String)> {
        HookResult::Continue((provider, model))
    }
    async fn before_prompt_build(&self, prompt: String) -> HookResult<String> {
        HookResult::Continue(prompt)
    }
}
```

**Hook Types:**
- **Void hooks**: Fire-and-forget, run in parallel
- **Modifying hooks**: Sequential by priority, can modify or cancel operations

**Built-in Hooks:**
- `command_logger`: Logs all shell commands
- Custom hooks via `~/.zeroclaw/hooks/`

---

### 15. Observability (`src/observability/`)

Monitoring and tracing system for production visibility.

**Observer Events:**
```rust
pub enum ObserverEvent {
    AgentStart { provider, model },
    LlmRequest { provider, model, messages_count },
    LlmResponse { provider, model, duration, success, input_tokens, output_tokens },
    AgentEnd { provider, model, duration, tokens_used, cost_usd },
    ToolCallStart { tool },
    ToolCall { tool, duration, success },
    TurnComplete,
    ChannelMessage { channel, direction },
    HeartbeatTick,
}
```

**Backends:**
- `noop` - No-op (default)
- `verbose` - Console logging
- `log` - Structured logging
- `prometheus` - Prometheus metrics
- `otel` - OpenTelemetry traces
- `multi` - Multiple observers

---

### 16. Runtime Adapter (`src/runtime/`)

Platform abstraction for porting ZeroClaw to different environments.

**Core Trait:** `RuntimeAdapter` in `src/runtime/traits.rs`

```rust
pub trait RuntimeAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn has_shell_access(&self) -> bool;
    fn has_filesystem_access(&self) -> bool;
    fn storage_path(&self) -> PathBuf;
    fn supports_long_running(&self) -> bool;
    fn memory_budget(&self) -> u64;
}
```

**Implemented Runtimes:**
| Runtime | Shell | FS | Long-Running | Description |
|---------|-------|-------|--------------|-------------|
| Native | ✅ | ✅ | ✅ | Standard Linux/macOS/Windows |
| Docker | ✅ | ✅ | ✅ | Container environment |
| WASM | ❌ | ❌ | ❌ | Browser/Edge (limited) |

---

### 17. Cron/Tasks (`src/cron/`)

Scheduled task execution system.

**Features:**
- Cron expression scheduling (5-field format)
- One-shot scheduled tasks (RFC3339 timestamp)
- Fixed-interval recurring tasks
- Delayed one-shot tasks ("30m", "2h", "1d")
- Job types: `Shell`, `Agent`
- Session targets: `Isolated`, `Main`

**Configuration:**
```toml
[[jobs]]
id = "daily_report"
expression = "0 9 * * *"
timezone = "America/New_York"
job_type = "agent"
command = "Generate daily usage report"
```

**CLI Tools:**
- `cron_add`, `cron_list`, `cron_remove`
- `cron_update`, `cron_pause`, `cron_resume`
- `cron_runs` - execution history

---

### 18. Tunnel Providers (`src/tunnel/`)

Reverse tunneling for exposing local gateway publicly.

**Core Trait:** `Tunnel` in `src/tunnel/mod.rs`

```rust
#[async_trait]
pub trait Tunnel: Send + Sync {
    fn name(&self) -> &str;
    async fn start(&self, local_host: &str, local_port: u16) -> Result<String>;
    async fn stop(&self) -> Result<()>;
    async fn health_check(&self) -> bool;
    fn public_url(&self) -> Option<String>;
}
```

**Providers:**
| Provider | Binary | Description |
|----------|---------|-------------|
| Cloudflare | `cloudflared` | Cloudflare Tunnel |
| Ngrok | `ngrok` | Ngrok tunnel |
| Tailscale | `tailscale` | Tailscale funnels |
| Custom | Custom command | Arbitrary tunnel binary |

---

### 19. Cost Tracking (`src/cost/`)

Token usage and cost monitoring per provider.

**Components:**
- `tracker.rs` - Real-time cost accumulation
- `types.rs` - Cost calculation types

**Features:**
- Per-request token tracking
- Provider-specific pricing
- Session-level aggregation
- Budget alerts

---

### 20. Peripherals (`src/peripherals/`)

Hardware integration for robotics/IoT applications.

**Supported Boards:**
- Arduino Uno (via serial)
- STM32 Nucleo (via ST-Link)
- ESP32 (via serial)
- Raspberry Pi GPIO

**Tools:**
- `hardware_board_info` - Board information
- `hardware_memory_map` - Memory map
- `hardware_memory_read` - Read memory via probe-rs
- `peripheral_flash` - Flash firmware

---

### 21. Hardware Discovery (`src/hardware/`)

USB device enumeration and identification.

**Features:**
- USB VID/PID scanning
- Board type detection (STM32 Nucleo, Arduino, ESP32)
- Serial port discovery
- Chip info via probe-rs

---

### 22. Authentication (`src/auth/`)

OAuth and JWT authentication for providers.

**Components:**
- `anthropic_token.rs` - Anthropic token exchange
- `gemini_oauth.rs` - Google OAuth flow
- `openai_oauth.rs` - OpenAI OAuth flow
- `jwt.rs` - JWT utilities
- `profiles.rs` - Credential profiles

---

### 23. Onboarding (`src/onboard/`)

First-run setup wizard.

**Features:**
- Interactive TUI wizard
- Config file generation
- Provider setup
- Channel configuration
- Memory backend selection

---

### 24. Heartbeat (`src/heartbeat/`)

Keep-alive system for long-running sessions.

**Features:**
- Periodic ticks to prevent timeouts
- Session health monitoring
- Observer event emission

---

### 25. Doctor (`src/doctor/`)

Diagnostic and troubleshooting system.

**Features:**
- Health checks for all components
- Configuration validation
- Connectivity tests
- Remediation suggestions

---

### 26. Monitoring (`src/monitoring/`)

System resource monitoring.

**Features:**
- CPU usage tracking
- Memory usage tracking
- Temperature monitoring (where available)

---

### 27. Approval (`src/approval/`)

User approval workflow for sensitive operations.

**Features:**
- Interactive approval prompts
- Approval history
- Auto-approval rules

---

### 28. Integrations (`src/integrations/`)

Third-party service integrations registry.

**Features:**
- Integration discovery
- Capability querying
- Configuration management

---

### 29. SkillForge (`src/skillforge/`)

Skill scouting and integration tools.

**Components:**
- `scout.rs` - Find community skills
- `integrate.rs` - Integrate new skills
- `evaluate.rs` - Skill quality assessment

---

### 30. Health (`src/health/`)

Health check endpoints and monitoring.

---

### 31. Migration (`src/migration.rs`)

Data schema migrations and upgrades.

---

### 32. Diagnostic (`src/diagnostic.rs`)

System diagnostics and troubleshooting information.

---

### 33. Agent Core Modules (`src/agent/`)

Core agent implementation modules beyond the loop.

**Modules:**
- `agent.rs` - Main Agent struct with orchestration logic
- `classifier.rs` - Request classification for routing
- `dispatcher.rs` - Request dispatching to appropriate handlers
- `hands.rs` - "Hands" functionality for computer control
- `interruption.rs` - User interruption handling
- `memory_loader.rs` - Memory loading for context
- `prompt.rs` - System prompt construction
- `streaming.rs` - Agent streaming response handling
- `tasks_section.rs` - Tasks section in prompts
- `workspace.rs` - Workspace management

---

### 34. Channel Implementations (`src/channels/`)

Individual channel implementations beyond the core trait.

**Messaging Platforms:**
| Channel | File | Features |
|---------|------|----------|
| ClawdTalk | `clawdtalk.rs` | Custom ClawdTalk protocol |
| DingTalk | `dingtalk.rs` | Alibaba DingTalk integration |
| iMessage | `imessage.rs` | Apple iMessage via Applescript |
| IRC | `irc.rs` | Internet Relay Chat |
| Lark | `lark.rs` | Feishu/Lark (ByteDance) |
| Linq | `linq.rs` | Linq protocol |
| Mattermost | `mattermost.rs` | Mattermost integration |
| MQTT | `mqtt.rs` | MQTT broker connection |
| Nextcloud Talk | `nextcloud_talk.rs` | Nextcloud Talk |
| Nostr | `nostr.rs` | Nostr protocol |
| QQ | `qq.rs` | Tencent QQ |
| Signal | `signal.rs` | Signal messenger |
| Slack | `slack.rs` | Slack workspace |
| Wati | `wati.rs` | Wati WhatsApp API |

**WhatsApp Modules:**
- `whatsapp.rs` - Business API client
- `whatsapp_storage.rs` - SQLite storage for messages
- `whatsapp_web.rs` - Native WhatsApp Web client

**Telegram Extensions:**
- `telegram_circuit_breaker.rs` - Rate limiting
- `telegram_inline_keyboard.rs` - Inline keyboards
- `telegram_keyboard_ext.rs` - Keyboard extensions
- `telegram_menu_button.rs` - Menu button
- `telegram_patch.rs` - API patches/workarounds

**Utilities:**
- `transcription.rs` - Audio/video transcription

---

### 35. Config System (`src/config/`)

Configuration schema and validation.

**Modules:**
- `mod.rs` - Main Config struct
- `schema.rs` - Auto-generated JSON schema
- `routing.rs` - Routing-specific config
- `schemas/llm_schema.rs` - LLM provider schemas
- `schemas/memory_schema.rs` - Memory backend schemas
- `schemas/security_schema.rs` - Security policy schemas
- `traits.rs` - Config validation traits

---

### 36. Memory Sub-Modules (`src/memory/`)

Memory backend implementations and utilities.

**Backends:**
- `sqlite.rs` - SQLite embedded database
- `qdrant.rs` - Qdrant vector database
- `postgres.rs` - PostgreSQL backend
- `hybrid.rs` - Combined SQLite + Qdrant
- `none.rs` - No-op testing backend
- `lucid.rs` - Lucid dream/memory state
- `cpu.rs` - CPU-based memory operations

**Utilities:**
- `backend.rs` - Backend selection and creation
- `chunker.rs` - Text chunking for embeddings
- `cli.rs` - Memory CLI commands
- `embeddings.rs` - Embedding provider interface
- `hygiene.rs` - Memory cleanup and maintenance
- `markdown.rs` - Markdown formatting for memory
- `response_cache.rs` - LLM response caching
- `snapshot.rs` - Memory snapshot/restore
- `tasks.rs` - Task-specific memory
- `vector.rs` - Vector operations

---

### 37. Provider Implementations (`src/providers/`)

Individual LLM provider implementations.

| Provider | File | Features |
|----------|------|----------|
| Anthropic | `anthropic.rs` | Claude, native tools, vision |
| OpenAI | `openai.rs` | GPT models, native tools |
| Gemini | `gemini.rs` | Google Gemini, function calling |
| GLM | `glm.rs` | Zhipu GLM-4 |
| Ollama | `ollama.rs` | Local models |
| Bedrock | `bedrock.rs` | AWS Bedrock |
| OpenRouter | `openrouter.rs` | API aggregator |
| Copilot | `copilot.rs` | GitHub Copilot |
| Telnyx | `telnyx.rs` | Telnyx communications |

**Shared:**
- `common/` - HTTP client, SSE parser
- `compatible.rs` - Compatibility shims
- `reliable.rs` - Reliability features
- `router.rs` - Virtual provider (auto-router)

---

### 38. Gateway Sub-Modules (`src/gateway/`)

Gateway implementation details.

| Module | Purpose |
|--------|---------|
| `api.rs` - Main API routes and handlers |
| `openai_compat.rs` - OpenAI compatibility layer |
| `openai_sse_types.rs` - SSE event type definitions |
| `openai_streaming.rs` - Streaming implementation |
| `sse.rs` - Server-Sent Events utilities |
| `static_files.rs` - Embedded static file serving |
| `telegram_threads.rs` - Telegram thread management |
| `telegram_webhook.rs` - Telegram webhook handler |
| `tma_auth.rs` - Telegram Mini App JWT auth |
| `ws.rs` - WebSocket connection handler |
| `tests/` - Integration tests |

---

### 39. Tools Catalog (`src/tools/`)

Complete tool implementation catalog (50+ tools).

**File Operations:** `file_read`, `file_write`, `file_edit`, `code_structure`
**Shell & Execution:** `shell`, `delegate`, `subagent_spawn`
**Memory:** `memory_store`, `memory_recall`, `memory_forget`
**Web:** `web_search_tool`, `web_fetch`, `http_request`, `browser`, `browser_open`
**Git:** `git_operations`
**Hardware:** `hardware_board_info`, `hardware_memory_map`, `hardware_memory_read`
**Cron:** `cron_add`, `cron_list`, `cron_remove`, `cron_run`, `cron_runs`, `cron_update`
**SOP:** `sop_execute`, `sop_status`, `sop_approve`, `sop_list`, `sop_advance`
**Utilities:** `cli_discovery`, `content_search`, `composio`, `examples`, `glob_search`, `image_info`, `pdf_read`, `proxy_config`, `pushover`, `screenshot`, `task_plan`
**Registry:** `registry`, `traits`, `macros`, `schema`, `schema_builder`

---

### 40. Security Modules (`src/security/`)

Security implementations beyond sandboxing.

| Module | Purpose |
|--------|---------|
| `audit.rs` - Security audit logging |
| `detect.rs` - Threat detection |
| `docker.rs` - Docker-specific security |
| `domain_matcher.rs` - Domain allowlist matching |
| `estop.rs` - Emergency stop functionality |
| `firejail.rs` - Firejail sandbox wrapper |
| `landlock.rs` - Landlock sandbox wrapper |
| `leak_detector.rs` - Credential leak detection |
| `otp.rs` - One-time password generation |
| `pairing.rs` - Device pairing |
| `policy.rs` - Security policy enforcement |
| `prompt_guard.rs` - Malicious prompt detection |
| `secrets.rs` - Secret storage |

---

### 41. TUI Modules (`src/bin/tui/`)

Terminal User Interface implementation.

| Module | Purpose |
|--------|---------|
| `main.rs` - TUI entry point |
| `app.rs` - Main TUI application |
| `ui.rs` - UI rendering |
| `events.rs` - Event handling |
| `agents.rs` - Agent management UI |
| `sessions.rs` - Session management UI |

---

### 42. Runtime Implementations (`src/runtime/`)

Platform-specific runtime implementations.

| Runtime | File | Purpose |
|---------|------|--------|
| Native | `native.rs` | Standard OS execution |
| Docker | `docker.rs` | Container environment |
| WASM | `wasm.rs` | Browser/Edge (limited) |

---

### 43. Observability Backends (`src/observability/`)

Observer implementation backends.

| Backend | File | Purpose |
|---------|------|--------|
| No-op | `noop.rs` | Disabled observability |
| Verbose | `verbose.rs` | Console output |
| Log | `log.rs` | Structured logging |
| Prometheus | `prometheus.rs` - Prometheus metrics |
| OTEL | `otel.rs` - OpenTelemetry traces |
| Multi | `multi.rs` - Multiple observers |

**Supporting:**
- `traits.rs` - Observer trait definitions
- `runtime_trace.rs` - Runtime tracing

---

### 44. SOP Sub-Modules (`src/sop/`)

SOP workflow implementation details.

| Module | Purpose |
|--------|---------|
| `condition.rs` - Condition evaluation |
| `dispatch.rs` - Request dispatch to SOPs |
| `gates.rs` - Gate execution (auto/manual) |
| `metrics.rs` - SOP execution metrics |
| `audit.rs` - SOP audit trail |

---

### 45. Skills Sub-Modules (`src/skills/`)

Skills system implementation details.

| Module | Purpose |
|--------|---------|
| `audit.rs` - Skill security audit |
| `engine.rs` - Skills engine core |
| `evaluator.rs` - Skill quality evaluator |
| `loader.rs` - Skill loader interface |
| `symlink_tests.rs` - Skill symlink tests |

---

### 46. Cron Sub-Modules (`src/cron/`)

| Module | Purpose |
|--------|---------|
| `schedule.rs` - Cron expression parsing |
| `scheduler.rs` - Job scheduler |
| `store.rs` - Job persistence (SQLite) |
| `types.rs` - Job type definitions |

---

### 47. Other Core Modules

| Module | Purpose | Location |
|--------|---------|----------|
| `approval/mod.rs` | Approval workflow | `src/approval/` |
| `daemon/mod.rs` | Daemon/service management | `src/daemon/` |
| `diagnostic.rs` | Diagnostics output | `src/diagnostic.rs` |
| `doctor/mod.rs` | Health check doctor | `src/doctor/` |
| `health/mod.rs` | Health check endpoints | `src/health/` |
| `heartbeat/engine.rs` | Keep-alive engine | `src/heartbeat/` |
| `identity.rs` | Identity management | `src/identity.rs` |
| `integrations/mod.rs` | Third-party integrations | `src/integrations/` |
| `integrations/registry.rs` | Integration registry | `src/integrations/registry.rs` |
| `lib.rs` | Library exports | `src/lib.rs` |
| `main.rs` | Binary entry point | `src/main.rs` |
| `migration.rs` | Data migrations | `src/migration.rs` |
| `multimodal.rs` | Multimodal content processing | `src/multimodal.rs` |
| `onboard/mod.rs` | Onboarding wizard | `src/onboard/` |
| `onboard/wizard.rs` | Wizard UI | `src/onboard/wizard.rs` |
| `rag/mod.rs` | Retrieval-Augmented Generation | `src/rag/` |
| `service/mod.rs` | Service management | `src/service/` |
| `tunnel/mod.rs` | Tunnel providers | `src/tunnel/` |
| `util.rs` | Utility functions | `src/util.rs` |

---

## Configuration

**Daemon Mode:** ZeroClaw can run as a system service (systemd/svc).

**Ports:**
- Production: `42000-42999` (gateway: `42618`)
- Evolution/Test: `52000-52999` (gateway: `52618`)
- Qdrant: `6333` (prod), `7333` (evo)

**Async Runtime:** Tokio with `rt-multi-thread`

**Concurrency:**
- Channel listeners run concurrently
- Tool execution sandboxed per invocation
- Rate limiting per client IP

---

## Data Flow Diagram

```
┌─────────┐
│ Channel │ (Telegram/Discord/etc)
└────┬────┘
     │ ChannelMessage
     ▼
┌─────────────────────────────────────┐
│      Intelligent Routing             │
│  ┌─────────────────────────────┐   │
│  │ 1. Classify Request Type    │   │
│  │ 2. Check Rate Limits        │   │
│  │ 3. Select Optimal Provider  │   │
│  │ 4. Optionally Spawn Subagent│   │
│  └─────────────────────────────┘   │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│         Agent Loop (A2A Swarm)      │
│  ┌─────────────────────────────┐   │
│  │ 1. Load Memory (Qdrant/SQLite)│  │
│  │ 2. Load Skills (v2.0)       │   │
│  │ 3. Build Prompt             │   │
│  │ 4. Call Provider (LLM)      │   │
│  │ 5. Parse Tool Calls         │   │
│  │ 6. Execute Tools (Sandbox)  │   │
│  │ 7. SOP Check (if applicable)│   │
│  │ 8. Store Results            │   │
│  │ 9. A2A Sync (if multi-agent)│  │
│  │ 10. Repeat until done       │   │
│  └─────────────────────────────┘   │
└──────────────┬──────────────────────┘
               │ ChatResponse
               ▼
        ┌──────────────┐
        │   Channel    │
        │   send()     │
        └──────────────┘
```

---

## Extension Points

To extend ZeroClaw:

1. **New Provider**: Implement `Provider` trait in `src/providers/`
2. **New Channel**: Implement `Channel` trait in `src/channels/`
3. **New Memory**: Implement `Memory` trait in `src/memory/`
4. **New Sandbox**: Implement `Sandbox` trait in `src/security/`
5. **New Tool**: Add module in `src/tools/` with `inventory::submit!`

---

## Build Profiles

```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = "fat"          # Maximum cross-crate optimization
codegen-units = 1    # Serialized codegen (low memory)
strip = true          # Remove debug symbols
panic = "abort"      # Reduce binary size

[profile.release-fast]
inherits = "release"
codegen-units = 8    # Parallel codegen (faster builds)
```

---

## Dependencies Highlights

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime |
| `axum` | HTTP server |
| `reqwest` | HTTP client |
| `rusqlite` | SQLite backend |
| `serde` | Serialization |
| `async-trait` | Async trait support |
| `anyhow` | Error handling |
| `parking_lot` | Fast mutexes |
| `inventory` | Compile-time registration |

---

## Zero-Bloat Principles

- **No Docker**: Native binary only
- **No PostgreSQL**: SQLite + optional Qdrant
- **No Go/Node.js**: Rust-only backend
- **Minimal RAM**: < 500MB per service
- **Static linking**: musl targets for standalone binaries

---

## Unique Enhancements

This ZeroClaw backend includes these **unique features** beyond standard ZeroClaw:

### Core Architecture
| Feature | Description | Location |
|---------|-------------|----------|
| **Agent Swarm** | Multi-agent orchestration (Planner/Executor/Reviewer) | `src/agent/a2a.rs` |
| **Intelligent Routing** | Rate-aware provider selection with classification | `src/routing/` |
| **Skills v2.0** | VectorSkillLoader with open-skills sync | `src/skills/` |
| **Qdrant Memory** | Semantic search with embedding providers | `src/memory/qdrant.rs` |
| **SOP Engine** | Workflow automation with approval gates | `src/sop/` |
| **Multimodal Gateway** | Image URL support in chat completions | `src/gateway/openai_compat.rs` |
| **Auto-Router** | Virtual model for intelligent provider routing | `src/providers/router.rs` |
| **Subagent Delegation** | Parallel task execution with depth limits | `src/routing/subagent.rs` |

### Extension Systems
| Feature | Description | Location |
|---------|-------------|----------|
| **Hooks System** | Event-based extension (void + modifying hooks) | `src/hooks/` |
| **Observability** | Observer events (log/prometheus/otel) | `src/observability/` |
| **Runtime Adapter** | Platform abstraction (Native/Docker/WASM) | `src/runtime/` |
| **Cron/Tasks** | Scheduled task execution (cron/one-shot/interval) | `src/cron/` |
| **Tunnel Providers** | Cloudflare/Ngrok/Tailscale tunneling | `src/tunnel/` |

### Hardware & IoT
| Feature | Description | Location |
|---------|-------------|----------|
| **Peripherals** | Arduino/Nucleo/ESP32/RPi GPIO integration | `src/peripherals/` |
| **Hardware Discovery** | USB VID/PID scanning, board detection | `src/hardware/` |

### Developer Tools
| Feature | Description | Location |
|---------|-------------|----------|
| **Cost Tracking** | Token usage and cost monitoring | `src/cost/` |
| **SkillForge** | Skill scouting and integration tools | `src/skillforge/` |
| **Doctor** | Diagnostic and troubleshooting | `src/doctor/` |
| **Onboarding** | First-run setup wizard | `src/onboard/` |

### Operations
| Feature | Description | Location |
|---------|-------------|----------|
| **Heartbeat** | Keep-alive system for long sessions | `src/heartbeat/` |
| **Health** | Health check endpoints | `src/health/` |
| **Monitoring** | CPU/memory usage tracking | `src/monitoring/` |
| **Migration** | Data schema migrations | `src/migration.rs` |
| **Diagnostic** | System diagnostics | `src/diagnostic.rs` |

### Security & Auth
| Feature | Description | Location |
|---------|-------------|----------|
| **OAuth Providers** | Anthropic/Gemini/OpenAI OAuth flows | `src/auth/` |
| **Approval** | User approval workflow | `src/approval/` |
| **Integrations** | Third-party service registry | `src/integrations/` |

---

*Generated from ZeroClaw v0.4.0 source code analysis*
