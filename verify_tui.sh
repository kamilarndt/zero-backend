#!/bin/bash
# Quick TUI verification script

echo "🔍 ZeroClaw TUI Verification"
echo "=============================="
echo ""

# Check binary
echo "1️⃣ Binary Installation:"
if command -v zeroclaw-tui &> /dev/null; then
    echo "   ✅ zeroclaw-tui found in PATH"
    echo "   📍 Location: $(which zeroclaw-tui)"
    echo "   📦 Size: $(du -h $(which zeroclaw-tui) | cut -f1)"
else
    echo "   ❌ zeroclaw-tui NOT found in PATH"
    exit 1
fi

echo ""

# Check version
echo "2️⃣ Version:"
zeroclaw-tui --version

echo ""

# Check help
echo "3️⃣ Help Test:"
zeroclaw-tui --help | head -5

echo ""
echo "=============================="
echo "✅ TUI is ready to use!"
echo ""
echo "To start TUI:"
echo "  zeroclaw-tui                    # Normal mode (connects to gateway)"
echo "  ZEROCLAW_TUI_DEMO=1 zeroclaw-tui  # Demo mode (no connection needed)"
echo ""
