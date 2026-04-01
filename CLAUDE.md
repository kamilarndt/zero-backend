# Claude Code - ZeroClaw Development Context

## Development Environment

You are working in the **ZeroClaw Development Environment** at `/home/arndtos/Research/zero-backend`.

**⚠️ IMPORTANT: This is the development workspace. Production installation is separate.**

```
/home/arndtos/Research/zero-backend/   # ZeroClaw DEVELOPMENT workspace
├── .git/                              # github.com/kamilarndt/zeroclaw-backend
├── src/                               # ZeroClaw source code
├── target/release/                    # Development builds
├── scripts/                           # Installation & upgrade scripts
├── .cargo/config.toml                 # Build configuration
└── docs/                              # Documentation

/home/arndtos/.cargo/bin/              # Production installation
├── zeroclaw                           # Production binary (installed via scripts)
└── zeroclaw-tui                       # Production TUI binary

/home/arndtos/.zeroclaw/               # Configuration & data
├── config.toml                        # Global configuration
└── storage/                           # Local data storage
```

## Development vs Production

### 💻 Development (This Workspace)
- **Location**: `~/Research/zero-backend/`
- **Binary**: `./target/release/zeroclaw` (built from source)
- **Use Case**: Active development, testing, debugging
- **Build**: `cargo build --release`
- **Git**: `github.com/kamilarndt/zeroclaw-backend`

### 🚀 Production (Installed Binary)
- **Location**: `~/.cargo/bin/zeroclaw`
- **Binary**: Installed via `./scripts/install-production.sh`
- **Use Case**: Daily use, stable features
- **Install**: `./scripts/install-production.sh`
- **Upgrade**: `./scripts/upgrade-production.sh`

## Development Commands

```bash
# Navigate to development workspace
cd ~/Research/zero-backend

# Build development binary
cargo build --release

# Run development binary
./target/release/zeroclaw --help
./target/release/zeroclaw agent --message "test"
./target/release/zeroclaw-tui

# Run tests
cargo test

# Lint code
cargo clippy

# Format code
cargo fmt

# Clean build artifacts
cargo clean
```

## Production Installation

```bash
# Install to production ( ~/.cargo/bin/ )
cd ~/Research/zero-backend
./scripts/install-production.sh

# Upgrade production installation
./scripts/upgrade-production.sh

# Run production binary
~/.cargo/bin/zeroclaw status
~/.cargo/bin/zeroclaw agent --message "test"
~/.cargo/bin/zeroclaw-tui
```

## Architecture

Trait-based system for ZeroClaw:

```rust
trait Provider    // LLM providers (GLM, OpenRouter, Groq...)
trait Channel     // Communication platforms (CLI, Telegram...)
trait Memory      // Storage backends (SQLite, Qdrant...)
trait Tool        // Function tools
trait Sandbox     // Security sandbox
```

### 3-Layer Routing Architecture

**Layer 1: Task Classification** (Auto-router)
- Classifies queries: coding, vision, reasoning, fast
- Virtual model: `zeroclaw-auto-router`
- Located in: `src/routing/classifier.rs`

**Layer 2: Provider Routing**
- RouterProvider: Hint-based provider selection
- Model aliases: Translate unsupported models
- Located in: `src/providers/router.rs`

**Layer 3: Fallback & Retry**
- ReliableProvider: Multi-provider fallback chains
- Exponential backoff for rate limits
- Error classification (retryable vs non-retryable)
- Located in: `src/providers/reliable.rs`

**Flow Example:**
```
User Query → Task Classifier → Model Hint → RouterProvider → ReliableProvider
                                  (vision)        (OpenRouter)     (Try GLM → Try OpenRouter → Try Groq)
```

## Troubleshooting Build Issues

### Stale Cargo Cache
**Problem:** Build fails with strange errors after significant changes
**Solution:**
```bash
# Clean specific package (faster)
cargo clean -p zeroclaw

# Full clean build (last resort)
cargo clean
cargo build --release
```

### Provider Fallback Not Working
**Problem:** Queries fail without attempting fallback providers
**Debug:**
```bash
# Enable detailed provider logging
RUST_LOG=zeroclaw::providers=warn ./target/release/zeroclaw agent --message "test"

# Check provider status
./target/release/zeroclaw status | grep -A 20 "Provider"
```

**Common Causes:**
- Provider not initialized in `~/.zeroclaw/config.toml`
- API key missing for provider
- Provider not in fallback chain: `[reliability]fallback_providers`

### Configuration Not Taking Effect
**Problem:** Changes to `~/.zeroclaw/config.toml` ignored
**Solution:**
```bash
# Restart ZeroClaw processes
pkill -f zeroclaw
./target/release/zeroclaw gateway --port 42617
```

## Project Structure

```
src/
├── providers/      # LLM implementations (GLM, OpenRouter, Groq...)
├── channels/       # Communication platforms (CLI, Telegram...)
├── memory/         # Storage backends (SQLite, Qdrant...)
├── tools/          # Tool registry
├── agent/          # Agent loop
├── gateway/        # HTTP/WebSocket API
├── routing/        # Task classification and routing
└── main.rs         # Entry point
```

## Related Documentation

**Comprehensive Routing Documentation:**
- `docs/routing/README.md` - Routing system overview
- `docs/routing/ARCHITECTURE.md` - Deep technical architecture
- `docs/routing/OPERATOR_GUIDE.md` - Log interpretation
- `docs/routing/TROUBLESHOOTING.md` - Fallback debugging

**Project Documentation:**
- `README.md` - Project overview and quick start
- `CLAUDE.md` - This file - development context

## Build Configuration

The project includes optimized build settings in `.cargo/config.toml`:

- **Release builds**: Full LTO, stripped, optimized for size
- **Dev builds**: Fast compilation, no optimization
- **Consistent settings**: Applied across all builds

## Development Workflow Rules

1. **DO NOT** modify production binary directly
2. **ALWAYS** develop in `~/Research/zero-backend/`
3. **TEST** thoroughly with `./target/release/zeroclaw`
4. **INSTALL** to production only when stable
5. **COMMIT** to `github.com/kamilarndt/zeroclaw-backend`

## Git Repository

- **Development**: `github.com/kamilarndt/zeroclaw-backend`
- **Branch**: `master` (main development branch)
- **Never force push** to master

## API Keys (Development)

```bash
# GLM Coding Plan (main provider)
export GLM_API_KEY="your-glm-key-here"

# Alternative providers for development
export OPENROUTER_API_KEY="sk-or-v1-..."
export GROQ_API_KEY="gsk_..."
export NVIDIA_API_KEY="nvapi-..."
```

**Note:** Never commit actual API keys to version control!

---

**Version:** 1.0.0
**Last Updated:** 2026-04-01
