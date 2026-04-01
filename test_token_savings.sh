#!/bin/bash
# Test script to demonstrate token savings from lazy tool loading

echo "=== ZeroClaw Token Savings Test ==="
echo ""
echo "This test demonstrates the token optimization implemented in lazy_loader.rs"
echo ""

# Set the zeroclaw binary path
ZEROCLAW="./target/release/zeroclaw"

# Check if binary exists
if [ ! -f "$ZEROCLAW" ]; then
    echo "Error: zeroclaw binary not found at $ZEROCLAW"
    echo "Please run: cargo build --release"
    exit 1
fi

echo "✓ Binary found: $ZEROCLAW"
echo ""

# Test 1: Simple query (should use 0 tools)
echo "Test 1: Simple Query (hello)"
echo "Expected: 0 tools selected → 100% token savings"
echo "Command: $ZEROCLAW agent --message 'hello'"
echo ""

# Test 2: File operation (should use ~5 tools)
echo ""
echo "Test 2: File Operation (read config)"
echo "Expected: 5 tools selected → 83% token savings"
echo "Command: $ZEROCLAW agent --message 'read the file Cargo.toml'"
echo ""

# Test 3: Web search (should use ~4 tools)
echo ""
echo "Test 3: Web Search"
echo "Expected: 4 tools selected → 87% token savings"
echo "Command: $ZEROCLAW agent --message 'search the web for rust programming'"
echo ""

# Test 4: Math question (should use 0 tools)
echo ""
echo "Test 4: Math Question"
echo "Expected: 0 tools selected → 100% token savings"
echo "Command: $ZEROCLAW agent --message 'what is 2+2'"
echo ""

echo "=== Manual Testing Required ==="
echo ""
echo "To see actual token savings, run the commands above and check the logs:"
echo ""
echo "  tail -f ~/.zeroclaw/logs/zeroclaw.log | grep 'Lazy tool loading'"
echo ""
echo "Look for log entries like:"
echo "  DEBUG Lazy tool loading: selected 0 out of 30 tools"
echo "  DEBUG Lazy tool loading: selected 5 out of 30 tools, token_savings_percent=83"
echo ""
echo "Or enable RUST_LOG=debug to see all debug output:"
echo ""
echo "  RUST_LOG=debug $ZEROCLAW agent --message 'hello'"
echo ""

# Show implementation details
echo "=== Implementation Details ==="
echo ""
echo "Modified files:"
echo "  - src/tools/lazy_loader.rs (NEW)"
echo "  - src/tools/mod.rs (exports)"
echo "  - src/agent/loop_.rs (integration)"
echo ""
echo "Key functions:"
echo "  - get_tool_inventory(): Creates lightweight metadata"
echo "  - select_relevant_tool_names(): Intelligently selects tools"
echo "  - ToolSelector::select(): Scores and filters tools"
echo ""
echo "Token savings formula:"
echo "  Savings% = ((TotalTools - SelectedTools) / TotalTools) × 100"
echo ""
echo "Estimated token savings per query type:"
echo "  - Simple queries (hello, status):    90K → 0K   (100%)"
echo "  - File operations (read, write):     90K → 15K  (83%)"
echo "  - Web searches (search, fetch):      90K → 12K  (87%)"
echo "  - Complex tasks (multi-step):        90K → 30K  (67%)"
echo ""
