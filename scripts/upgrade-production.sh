#!/bin/bash
set -e
#
# ZeroClaw Production Upgrade Script
# Updates ZeroClaw to the latest version from git
#
# Usage: ./scripts/upgrade-production.sh
#

echo "⬆️  ZeroClaw Production Upgrade"
echo "================================"

# Navigate to project root
cd "$(dirname "$0")/.."
PROJECT_ROOT=$(pwd)
echo "📂 Project root: $PROJECT_ROOT"

# Check if we're in the right place
if [[ ! -f "Cargo.toml" ]]; then
    echo "❌ Error: Cargo.toml not found. Are you in the ZeroClaw repository?"
    exit 1
fi

# Pull latest changes
echo "📥 Pulling latest changes from git..."
git pull origin master || git pull origin main

# Install using cargo
echo "📦 Upgrading ZeroClaw to ~/.cargo/bin/..."
cargo install --path . --root ~/.cargo --force

# Verify installation
if [[ -f ~/.cargo/bin/zeroclaw ]]; then
    echo "✅ Upgrade successful!"
    echo ""
    echo "Binary location: ~/.cargo/bin/zeroclaw"
    VERSION=$(~/.cargo/bin/zeroclaw --version 2>/dev/null || echo "Unable to determine version")
    echo "Version: $VERSION"
    echo ""
    echo "🎯 Ready to use!"
else
    echo "❌ Upgrade failed - binary not found"
    exit 1
fi
