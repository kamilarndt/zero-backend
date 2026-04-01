#!/bin/bash
set -e
#
# ZeroClaw Production Installation Script
# Installs ZeroClaw to ~/.cargo/bin for production use
#
# Usage: ./scripts/install-production.sh
#

echo "🚀 ZeroClaw Production Installation"
echo "===================================="

# Navigate to project root
cd "$(dirname "$0")/.."
PROJECT_ROOT=$(pwd)
echo "📂 Project root: $PROJECT_ROOT"

# Check if we're in the right place
if [[ ! -f "Cargo.toml" ]]; then
    echo "❌ Error: Cargo.toml not found. Are you in the ZeroClaw repository?"
    exit 1
fi

# Install using cargo
echo "📦 Installing ZeroClaw to ~/.cargo/bin/..."
cargo install --path . --root ~/.cargo --force

# Verify installation
if [[ -f ~/.cargo/bin/zeroclaw ]]; then
    echo "✅ Installation successful!"
    echo ""
    echo "Binary location: ~/.cargo/bin/zeroclaw"
    echo "Version: $(~/.cargo/bin/zeroclaw --version 2>/dev/null || echo 'Unable to determine version')"
    echo ""
    echo "🎯 Next steps:"
    echo "  1. Run: ~/.cargo/bin/zeroclaw status"
    echo "  2. Initialize: ~/.cargo/bin/zeroclaw onboard"
    echo "  3. Start gateway: ~/.cargo/bin/zeroclaw gateway --port 42617"
    echo ""
    echo "📝 Note: Make sure ~/.cargo/bin is in your PATH"
else
    echo "❌ Installation failed - binary not found"
    exit 1
fi
