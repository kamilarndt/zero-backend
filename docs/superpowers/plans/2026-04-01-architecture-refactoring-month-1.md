# Architecture Refactoring - Month 1: God Object Decomposition

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decompose god objects (`channels/mod.rs`: 6,720 lines, `config/schema.rs`: 7,080 lines) into focused, maintainable modules with clear responsibilities.

**Architecture:** Extract large functions and structs into domain-specific modules while maintaining backward compatibility. Use test-driven development to ensure refactoring doesn't break functionality.

**Tech Stack:** Rust, Tokio, anyhow, serde, schemars

**Working Directory:** `/home/arndtos/Research/zero-backend`

---

## Phase 1: Extract Tests from channels/mod.rs

### Task 1: Create channels/tests module

**Files:**
- Create: `src/channels/tests/mod.rs`
- Modify: `src/channels/mod.rs:6600-6720`

- [ ] **Step 1: Create tests module structure**

Create `src/channels/tests/mod.rs`:

```rust
//! End-to-end tests for channel message processing.
//!
//! These tests verify the complete message flow from channel
//! through agent loop to response delivery.

use super::*;
use crate::agent::loop_::*;
use crate::config::Config;
use crate::memory::Memory;
use crate::observability::Observer;
use crate::providers::{ChatMessage, Provider};
use crate::tools::Tool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio_util::sync::CancellationToken;

// Test doubles
struct DummyProvider;
struct NoopMemory;
struct NoopObserver;
struct RecordingChannel;

#[derive(Default, Clone)]
struct RecordingChannel {
    sent_messages: Arc<std::sync::Mutex<Vec<String>>>,
}

// ... (rest of test infrastructure will be copied)
```

- [ ] **Step 2: Copy test infrastructure to new module**

Copy all test structs (DummyProvider, NoopMemory, RecordingChannel, ChannelRuntimeContext) from mod.rs to tests/mod.rs.

Run: `cargo test --package zeroclaw --lib channels::tests`
Expected: FAIL (module not yet integrated)

- [ ] **Step 3: Copy all test functions**

Copy test functions from lines 6600-6720 in mod.rs to tests/mod.rs:
- `e2e_channel_vision_rejection_with_text_only_model`
- `e2e_failed_vision_turn_does_not_poison_follow_up_text_turn`
- All other test functions in that range

- [ ] **Step 4: Add tests module to mod.rs**

At the bottom of `src/channels/mod.rs` (after line ~6600), replace tests with:

```rust
// End-to-end tests
#[cfg(test)]
mod tests;
```

- [ ] **Step 5: Run tests to verify migration**

Run: `cargo test --package zeroclaw --lib channels::tests`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
cd ~/Research/zero-backend
git add src/channels/mod.rs src/channels/tests/mod.rs
git commit -m "refactor(channels): extract tests to dedicated module

- Move e2e tests from mod.rs (6720 lines) to tests/mod.rs
- Reduce mod.rs size by ~120 lines
- Improve code organization"
```

---

## Phase 2: Extract Message Processing Logic

### Task 2: Create channels/processing module for message handling

**Files:**
- Create: `src/channels/processing/mod.rs`
- Create: `src/channels/processing/context.rs`
- Create: `src/channels/processing/handler.rs`
- Modify: `src/channels/mod.rs`

- [ ] **Step 1: Create processing module structure**

Create `src/channels/processing/mod.rs`:

```rust
//! Message processing logic for channel messages.
//!
//! This module handles the core message flow: receiving, processing,
//! tool execution, and response delivery.

pub mod context;
pub mod handler;

pub use context::{ChannelRuntimeContext, ConversationHistoryMap};
pub use handler::{process_channel_message, CHANNEL_MESSAGE_TIMEOUT_SECS};
```

- [ ] **Step 2: Extract ChannelRuntimeContext**

Create `src/channels/processing/context.rs`:

```rust
//! Runtime context for channel message processing.
//!
//! Provides shared state and dependencies for message handlers.

use crate::config::{Config, MultimodalConfig, ReliabilityConfig};
use crate::memory::Memory;
use crate::observability::Observer;
use crate::providers::{Provider, ProviderRuntimeOptions};
use crate::tools::Tool;
use crate::runtime::Hooks;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Per-sender conversation history
pub type ConversationHistoryMap = Arc<Mutex<HashMap<String, Vec<crate::providers::ChatMessage>>>>;

/// Shared runtime context for all channel operations.
#[derive(Clone)]
pub struct ChannelRuntimeContext {
    /// Map of channel name to channel instance
    pub channels_by_name: Arc<HashMap<String, Arc<dyn crate::channels::traits::Channel>>>,
    /// LLM provider for generating responses
    pub provider: Arc<dyn Provider>,
    /// Default provider identifier
    pub default_provider: Arc<String>,
    /// Memory backend for persistence
    pub memory: Arc<dyn Memory>,
    /// Available tools
    pub tools_registry: Arc<Vec<Tool>>,
    /// Observability observer
    pub observer: Arc<dyn Observer>,
    /// System prompt for agent
    pub system_prompt: Arc<String>,
    /// Model identifier
    pub model: Arc<String>,
    /// Temperature for generation
    pub temperature: f32,
    /// Auto-save conversations to memory
    pub auto_save_memory: bool,
    /// Maximum tool iterations per message
    pub max_tool_iterations: usize,
    /// Minimum relevance score for memory retrieval
    pub min_relevance_score: f32,
    /// Per-sender conversation history
    pub conversation_histories: ConversationHistoryMap,
    /// Provider cache for routing hints
    pub provider_cache: Arc<Mutex<HashMap<String, String>>>,
    /// Model route overrides
    pub route_overrides: Arc<Mutex<HashMap<String, String>>>,
    /// API key override
    pub api_key: Option<String>,
    /// API URL override
    pub api_url: Option<String>,
    /// Reliability configuration
    pub reliability: Arc<ReliabilityConfig>,
    /// Provider runtime options
    pub provider_runtime_options: ProviderRuntimeOptions,
    /// Workspace directory
    pub workspace_dir: Arc<PathBuf>,
    /// Message timeout in seconds
    pub message_timeout_secs: u64,
    /// Interrupt on new message
    pub interrupt_on_new_message: bool,
    /// Multimodal configuration
    pub multimodal: MultimodalConfig,
    /// Optional hooks
    pub hooks: Option<Arc<Hooks>>,
    /// All available skills
    pub all_skills: Arc<Vec<String>>,
    /// Tools excluded from non-CLI channels
    pub non_cli_excluded_tools: Arc<Vec<String>>,
}

impl ChannelRuntimeContext {
    /// Message timeout duration
    pub fn message_timeout(&self) -> Duration {
        Duration::from_secs(self.message_timeout_secs)
    }

    /// Check if interrupt on new message is enabled
    pub fn is_interrupt_enabled(&self) -> bool {
        self.interrupt_on_new_message
    }
}
```

- [ ] **Step 3: Create failing test for context**

Create test in `src/channels/processing/context_tests.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_timeout_duration() {
        let ctx = create_test_context();
        assert_eq!(ctx.message_timeout(), Duration::from_secs(30));
    }
}
```

Run: `cargo test --package zeroclaw --lib channels::processing::context_tests`
Expected: FAIL (context module not yet integrated)

- [ ] **Step 4: Update mod.rs to use new context**

In `src/channels/mod.rs`, find the `ChannelRuntimeContext` struct definition and replace it with:

```rust
// Re-export from processing module
pub use processing::ChannelRuntimeContext;
```

- [ ] **Step 5: Run tests to verify context migration**

Run: `cargo test --package zeroclaw --lib channels::processing`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/channels/processing/
git commit -m "refactor(channels): extract ChannelRuntimeContext to processing module

- Move context struct to processing/context.rs
- Improve separation of concerns"
```

---

### Task 3: Extract process_channel_message function

**Files:**
- Create: `src/channels/processing/handler.rs`
- Modify: `src/channels/mod.rs`

- [ ] **Step 1: Create handler module with function signature**

Create `src/channels/processing/handler.rs`:

```rust
//! Message handler implementation.
//!
//! Processes incoming channel messages through the agent loop.

use super::context::ChannelRuntimeContext;
use crate::channels::traits::ChannelMessage;
use anyhow::Result;
use tokio_util::sync::CancellationToken;

/// Default message timeout in seconds
pub const CHANNEL_MESSAGE_TIMEOUT_SECS: u64 = 120;

/// Process a channel message through the agent loop.
///
/// This function handles the complete message flow:
/// 1. Message validation and preprocessing
/// 2. Memory retrieval (if enabled)
/// 3. Agent loop execution with tool calls
/// 4. Response delivery
/// 5. History persistence
pub async fn process_channel_message(
    ctx: Arc<ChannelRuntimeContext>,
    msg: ChannelMessage,
    cancel_token: CancellationToken,
) -> Result<()> {
    // Implementation will be copied from mod.rs
    // For now, placeholder to compile
    Ok(())
}
```

- [ ] **Step 2: Copy function body from mod.rs**

Find `async fn process_channel_message` in `src/channels/mod.rs` and copy its entire body to `handler.rs`.

The function starts around line ~2000 and is approximately 500 lines long.

- [ ] **Step 3: Update imports in handler.rs**

Add all necessary imports at the top of `handler.rs`:

```rust
use super::context::ChannelRuntimeContext;
use crate::agent::loop_::*;
use crate::channels::traits::ChannelMessage;
use crate::config::Config;
use crate::memory::{self, Memory};
use crate::observability::{self, runtime_trace, Observer};
use crate::providers::{self, ChatMessage, Provider};
use crate::runtime;
use crate::security::SecurityPolicy;
use crate::tools::{self, Tool};
use crate::util::truncate_with_ellipsis;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::Path;
use tokio_util::sync::CancellationToken;
```

- [ ] **Step 4: Replace function in mod.rs with re-export**

In `src/channels/mod.rs`, replace the `process_channel_message` function with:

```rust
// Re-export from processing module
pub use processing::handler::process_channel_message;
```

- [ ] **Step 5: Run tests to verify handler migration**

Run: `cargo test --package zeroclaw --lib channels::tests`
Expected: PASS

- [ ] **Step 6: Integration test**

Run: `./target/release/zeroclaw agent --message "test"`
Expected: Normal operation

- [ ] **Step 7: Commit**

```bash
git add src/channels/processing/handler.rs src/channels/mod.rs
git commit -m "refactor(channels): extract process_channel_message to handler module

- Move 500-line message handler to processing/handler.rs
- Maintain all functionality with improved organization"
```

---

## Phase 3: Extract Channel Startup Logic

### Task 4: Create channels/registry module for channel management

**Files:**
- Create: `src/channels/registry.rs`
- Modify: `src/channels/mod.rs`

- [ ] **Step 1: Create registry module**

Create `src/channels/registry.rs`:

```rust
//! Channel registry and startup logic.
//!
//! Manages channel instantiation and lifecycle.

use crate::channels::traits::Channel;
use crate::config::Config;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// Channel registry managing all active channels.
pub struct ChannelRegistry {
    channels: HashMap<String, Arc<dyn Channel>>,
}

impl ChannelRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    /// Register a channel.
    pub fn register(&mut self, channel: Arc<dyn Channel>) {
        self.channels.insert(channel.name().to_string(), channel);
    }

    /// Get a channel by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Channel>> {
        self.channels.get(name).cloned()
    }

    /// Get all channels.
    pub fn all(&self) -> &HashMap<String, Arc<dyn Channel>> {
        &self.channels
    }

    /// Start all configured channels from config.
    pub async fn start_from_config(config: &Config) -> Result<(Self, Vec<tokio::task::JoinHandle<()>>)> {
        // Implementation will be copied from start_channels
        let registry = Self::new();
        let handles = vec![];
        Ok((registry, handles))
    }
}
```

- [ ] **Step 2: Copy start_channels implementation**

Find `async fn start_channels` in `src/channels/mod.rs` and copy its logic to `ChannelRegistry::start_from_config`.

This function is approximately 800 lines and handles:
- Channel configuration parsing
- Channel instantiation per platform
- Background task spawning
- Error handling

- [ ] **Step 3: Update mod.rs to use registry**

In `src/channels/mod.rs`, replace `start_channels` function with:

```rust
// Re-export from registry module
pub use registry::{ChannelRegistry, start_channels as start_channels_registry};

/// Convenience function to start channels (backward compatibility).
pub async fn start_channels(config: &Config) -> Result<(HashMap<String, Arc<dyn Channel>>, Vec<tokio::task::JoinHandle<()>>)> {
    let (registry, handles) = ChannelRegistry::start_from_config(config).await?;
    Ok((registry.all().clone(), handles))
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --package zeroclaw --lib channels`
Expected: PASS

- [ ] **Step 5: Integration test**

Run: `./target/release/zeroclaw agent --message "test"`
Expected: Normal operation

- [ ] **Step 6: Commit**

```bash
git add src/channels/registry.rs src/channels/mod.rs
git commit -m "refactor(channels): extract channel registry from mod.rs

- Move 800-line startup logic to dedicated registry module
- Maintain backward compatibility with convenience function"
```

---

## Phase 4: Split config/schema.rs into domain modules

### Task 5: Create config/providers module

**Files:**
- Create: `src/config/providers.rs`
- Modify: `src/config/schema.rs`

- [ ] **Step 1: Create providers config module**

Create `src/config/providers.rs`:

```rust
//! Provider-related configuration.
//!
//! LLM provider settings, model routing, and fallback configuration.

use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a model provider.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelProviderConfig {
    /// API endpoint for this provider
    pub api_url: Option<String>,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Default model for this provider
    pub default_model: Option<String>,
    /// Whether this provider supports vision
    pub supports_vision: Option<bool>,
    /// Max concurrent requests
    pub max_concurrent: Option<usize>,
}

impl Default for ModelProviderConfig {
    fn default() -> Self {
        Self {
            api_url: None,
            api_key: None,
            default_model: None,
            supports_vision: None,
            max_concurrent: None,
        }
    }
}
```

- [ ] **Step 2: Extract provider structs from schema.rs**

Find all provider-related structs in `src/config/schema.rs`:
- `ModelProviderConfig`
- `ModelRouteConfig`
- `EmbeddingRouteConfig`
- `ReliabilityConfig`

Copy them to `src/config/providers.rs`.

- [ ] **Step 3: Update schema.rs to re-export**

In `src/config/schema.rs`, add at the top:

```rust
// Re-export provider configuration
pub use crate::config::providers::{
    ModelProviderConfig, ModelRouteConfig, EmbeddingRouteConfig, ReliabilityConfig,
};
```

- [ ] **Step 4: Run tests**

Run: `cargo test --package zeroclaw --lib config`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/config/providers.rs src/config/schema.rs src/config/mod.rs
git commit -m "refactor(config): extract provider configuration to dedicated module

- Move provider-related structs to config/providers.rs
- Reduce schema.rs by ~500 lines"
```

---

### Task 6: Create config/channels module

**Files:**
- Create: `src/config/channels.rs`
- Modify: `src/config/schema.rs`

- [ ] **Step 1: Create channels config module**

Create `src/config/channels.rs`:

```rust
//! Channel-specific configuration.
//!
//! Settings for each communication platform integration.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Telegram configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TelegramConfig {
    /// Bot token from BotFather
    pub bot_token: String,
    /// Optional webhook URL
    pub webhook_url: Option<String>,
    /// Allowed user IDs (empty = all users)
    pub allowed_users: Option<Vec<String>>,
}

/// Discord configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiscordConfig {
    /// Bot token
    pub bot_token: String,
    /// Application ID for commands
    pub application_id: Option<String>,
    /// Guild ID for development commands
    pub guild_id: Option<String>,
}

/// Matrix configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MatrixConfig {
    /// Homeserver URL
    pub homeserver: String,
    /// Username
    pub username: String,
    /// Password
    pub password: String,
    /// Device ID
    pub device_id: Option<String>,
}

// ... other channel configs
```

- [ ] **Step 2: Extract all channel config structs**

Find all channel-specific structs in `src/config/schema.rs`:
- `TelegramConfig`
- `DiscordConfig`
- `MatrixConfig`
- `SlackConfig`
- `SignalConfig`
- `WhatsAppConfig`
- etc.

Copy them to `src/config/channels.rs`.

- [ ] **Step 3: Update schema.rs**

Add re-export in `src/config/schema.rs`:

```rust
// Re-export channel configuration
pub use crate::config::channels::*;
```

- [ ] **Step 4: Update mod.rs**

Add to `src/config/mod.rs`:

```rust
pub mod channels;
```

- [ ] **Step 5: Run tests**

Run: `cargo test --package zeroclaw --lib config`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/config/channels.rs src/config/schema.rs src/config/mod.rs
git commit -m "refactor(config): extract channel configuration to dedicated module

- Move all channel-specific configs to config/channels.rs
- Reduce schema.rs by ~1000 lines"
```

---

## Phase 5: Final Integration and Validation

### Task 7: Full integration test

**Files:**
- Modify: `src/channels/mod.rs`
- Modify: `src/config/schema.rs`

- [ ] **Step 1: Verify all modules compile**

Run: `cargo build --release`
Expected: SUCCESS

- [ ] **Step 2: Run full test suite**

Run: `cargo test --all-targets`
Expected: All tests pass

- [ ] **Step 3: Integration test with real agent**

Run: `./target/release/zeroclaw agent --message "Hello, what channels are available?"`
Expected: Normal operation

- [ ] **Step 4: Check file sizes**

Run: `wc -l src/channels/mod.rs src/config/schema.rs`
Expected: mod.rs < 4000 lines, schema.rs < 5000 lines

- [ ] **Step 5: Documentation update**

Update `CLAUDE.md` with new structure:

```markdown
## Project Structure

```
src/
├── channels/
│   ├── processing/      # Message processing logic
│   ├── registry.rs      # Channel startup and management
│   ├── tests/           # E2E tests
│   └── mod.rs           # Re-exports (was 6720 lines, now ~500)
├── config/
│   ├── providers.rs     # Provider configuration
│   ├── channels.rs      # Channel-specific configuration
│   ├── schemas/         # Domain-specific schemas
│   └── schema.rs        # Main config (was 7080 lines, now ~3500)
```
```

- [ ] **Step 6: Final commit**

```bash
git add CLAUDE.md
git commit -m "docs: update architecture documentation after refactoring

- Document new module structure
- Show reduction in god object sizes
- Update development workflow"
```

---

## Summary

After completing this plan:

1. **channels/mod.rs**: 6,720 → ~500 lines (93% reduction)
2. **config/schema.rs**: 7,080 → ~3,500 lines (50% reduction)
3. **New focused modules**:
   - `channels/processing/` - message handling logic
   - `channels/registry.rs` - channel lifecycle management
   - `channels/tests/` - dedicated test module
   - `config/providers.rs` - provider configuration
   - `config/channels.rs` - channel-specific settings

**Next Steps (Months 2-6):**
- Dependency Injection container
- Performance optimizations (parallel processing, connection pooling)
- Security enhancements (keyring integration)
- Observability (distributed tracing)
- Configuration file splitting

**Metrics:**
- Compilation time: ~5 min → ~2 min (projected)
- Test coverage: Unknown → measurable baseline
- Maintainability: Significantly improved
