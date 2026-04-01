#!/bin/bash
#
# ZeroClaw Cleanup Script
# Removes old/confusing binaries and symlinks
#
# Usage: ./scripts/cleanup-old-files.sh
#

echo "🧹 ZeroClaw Cleanup Script"
echo "=========================="

# Remove development symlink from ~/.local/bin
if [[ -L ~/.local/bin/zeroclaw-cli ]]; then
    echo "🗑️  Removing ~/.local/bin/zeroclaw-cli (development symlink)"
    rm ~/.local/bin/zeroclaw-cli
    echo "✅ Removed"
else
    echo "ℹ️  ~/.local/bin/zeroclaw-cli not found"
fi

# Check for old wrapper scripts (optional removal)
echo ""
echo "📋 Found wrapper scripts in ~/.local/bin/:"
ls -1 ~/.local/bin/zeroclaw-* 2>/dev/null | wc -l
echo "These are not removed automatically. Review and delete manually if desired."

# Verify production installation
echo ""
echo "🔍 Checking production installation..."
if [[ -f ~/.cargo/bin/zeroclaw ]]; then
    echo "✅ Production binary found: ~/.cargo/bin/zeroclaw"
    VERSION=$(~/.cargo/bin/zeroclaw --version 2>/dev/null || echo "Unable to determine version")
    echo "   Version: $VERSION"
else
    echo "⚠️  No production binary found"
    echo "   Run: ./scripts/install-production.sh"
fi

echo ""
echo "📝 Summary:"
echo "  - Development builds: ~/Research/zero-backend/target/release/"
echo "  - Production binary: ~/.cargo/bin/zeroclaw"
echo "  - Configuration: ~/.zeroclaw/config.toml"
echo ""
echo "✅ Cleanup complete!"
