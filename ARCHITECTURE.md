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
│  (OpenAI-Compatible API + SSE Streaming + Webhooks + TMA Auth)  │
└───────────────────────────┬─────────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────────┐
│                        Agent Loop                                │
│  (Message Processing → Tool Execution → Response Generation)    │
└───┬───────────┬───────────┬───────────┬───────────┬────────────┘
    │           │           │           │           │
┌───▼────┐ ┌───▼────┐ ┌───▼────┐ ┌───▼────┐ ┌────▼─────┐
│Provider│ │Channel │ │ Memory │ │Security│ │  Tools   │
│  Trait │ │  Trait │ │  Trait │ │  Trait │ │ Registry │
└────────┘ └────────┘ └────────┘ └────────┘ └──────────┘
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

## Configuration

**Location:** `~/.zeroclaw/config.toml`

**Schema:** Auto-generated from `struct Config` using `schemars` crate.

**Key Sections:**
```toml
[agent]
model = "claude-sonnet-4-20250514"
temperature = 0.7
max_tool_iterations = 10

[memory]
backend = "sqlite"

[[providers]]
name = "anthropic"
api_key = "..."

[[channels]]
type = "telegram"
bot_token = "..."

[security]
sandbox = "landlock"
policy_strict = true
```

---

## Runtime Architecture

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
│         Agent Loop                  │
│  ┌─────────────────────────────┐   │
│  │ 1. Load Memory (SQLite)     │   │
│  │ 2. Build Prompt             │   │
│  │ 3. Call Provider (LLM)      │   │
│  │ 4. Parse Tool Calls         │   │
│  │ 5. Execute Tools (Sandbox)  │   │
│  │ 6. Store Results            │   │
│  │ 7. Repeat until done        │   │
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

*Generated from ZeroClaw v0.4.0 source code analysis*
