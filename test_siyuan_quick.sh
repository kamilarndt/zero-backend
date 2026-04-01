#!/bin/bash

# Quick SiYuan Integration Test
# Fast validation without running full agent

echo "================================"
echo "Quick SiYuan Integration Test"
echo "================================"
echo ""

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASSED=0
FAILED=0

# Test 1: Binary
echo -n "1. ZeroClaw binary: "
if [ -f "$HOME/Research/zero-backend/target/release/zeroclaw" ]; then
    echo -e "${GREEN}PASS${NC}"
    ((PASSED++))
else
    echo -e "${RED}FAIL${NC}"
    ((FAILED++))
fi

# Test 2: SiYuan API
echo -n "2. SiYuan API connection: "
if curl -s -f "http://localhost:6806/api/notebook/lsNotebooks" > /dev/null 2>&1; then
    echo -e "${GREEN}PASS${NC}"
    ((PASSED++))
else
    echo -e "${RED}FAIL${NC} (Is SiYuan running?)"
    ((FAILED++))
fi

# Test 3: Environment variable
echo -n "3. API token in environment: "
if [ -n "$SIYUAN_API_TOKEN" ]; then
    echo -e "${GREEN}PASS${NC}"
    ((PASSED++))
else
    echo -e "${YELLOW}WARN${NC} (Using fallback)"
    ((PASSED++))
fi

# Test 4: Config file
echo -n "4. Config file [siyuan] section: "
if grep -q "\[siyuan\]" "$HOME/.zeroclaw/config.toml" 2>/dev/null; then
    echo -e "${GREEN}PASS${NC}"
    ((PASSED++))
else
    echo -e "${YELLOW}WARN${NC} (Not configured)"
    ((PASSED++))
fi

# Test 5: API token in config
echo -n "5. API token in config: "
if grep -A 5 "\[siyuan\]" "$HOME/.zeroclaw/config.toml" 2>/dev/null | grep -q "api_token"; then
    echo -e "${GREEN}PASS${NC}"
    ((PASSED++))
else
    echo -e "${YELLOW}WARN${NC} (Will use env var)"
    ((PASSED++))
fi

# Test 6: SiYuan query (direct API)
echo -n "6. Direct API query: "
RESULT=$(curl -s -X POST "http://localhost:6806/api/query/sql" \
    -H "Content-Type: application/json" \
    -d '{"stmt": "SELECT COUNT(*) as count FROM blocks WHERE type='\''d'\''"}' 2>/dev/null)
if echo "$RESULT" | grep -q '"code":0'; then
    echo -e "${GREEN}PASS${NC}"
    ((PASSED++))
else
    echo -e "${RED}FAIL${NC}"
    ((FAILED++))
fi

# Test 7: Documentation
echo -n "7. Documentation exists: "
if [ -f "$HOME/Research/zero-backend/docs/siyuan-channel.md" ]; then
    echo -e "${GREEN}PASS${NC}"
    ((PASSED++))
else
    echo -e "${YELLOW}WARN${NC} (Not found)"
    ((PASSED++))
fi

echo ""
echo "================================"
echo "Results: $PASSED passed, $FAILED failed"
echo "================================"

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All critical tests passed!${NC}"
    echo ""
    echo "Note: Full tool testing requires running the ZeroClaw agent,"
    echo "which takes 15-30 seconds per test. Use test_siyuan_full.sh"
    echo "for comprehensive testing."
    exit 0
else
    echo -e "${RED}Some tests failed${NC}"
    exit 1
fi
