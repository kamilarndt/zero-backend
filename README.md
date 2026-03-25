# ZeroClaw Backend

Zero overhead. Zero compromise. 100% Rust. The fastest, smallest AI assistant.

This repository contains the core backend only - minimal setup for building ZeroClaw on any machine.

## Quick Start

```bash
# Install Rust (1.87+)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build
cargo build --release

# Run
./target/release/zeroclaw-tui --help
```

## Configuration

ZeroClaw stores configuration in `~/.zeroclaw/config.toml`. Run `zeroclaw-tui` first to initialize.

## Structure

- `src/` - Core Rust source code
- `crates/robot-kit/` - Hardware integration crate
- `.cargo/config.toml` - Cargo build configuration

## License

MIT OR Apache-2.0
