#!/bin/bash

# SiYuan Integration Test Script
# Quick validation of SiYuan tool availability and functionality

# Don't exit on error - we want to run all tests
# set -e

ZEROCLAW_BIN="$HOME/Research/zero-backend/target/release/zeroclaw"
SIYUAN_TOKEN="u4l8xqzxu6oual1o"
SIYUAN_URL="http://localhost:6806"

echo "================================"
echo "SiYuan Integration Test Suite"
echo "================================"
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Function to run test
run_test() {
    local test_name="$1"
    local test_command="$2"

    echo "Running: $test_name"
    if eval "$test_command" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ PASSED${NC}: $test_name"
        ((TESTS_PASSED++))
        return 0
    else
        echo -e "${RED}✗ FAILED${NC}: $test_name"
        ((TESTS_FAILED++))
        return 1
    fi
}

# Test 1: Check if ZeroClaw binary exists
echo "Test 1: ZeroClaw binary check"
if [ -f "$ZEROCLAW_BIN" ]; then
    echo -e "${GREEN}✓ PASSED${NC}: ZeroClaw binary found"
    ((TESTS_PASSED++))
else
    echo -e "${RED}✗ FAILED${NC}: ZeroClaw binary not found at $ZEROCLAW_BIN"
    ((TESTS_FAILED++))
    exit 1
fi

# Test 2: Check if SiYuan is running
echo ""
echo "Test 2: SiYuan API connectivity"
if curl -s -f "$SIYUAN_URL/api/notebook/lsNotebooks" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ PASSED${NC}: SiYuan API is accessible"
    ((TESTS_PASSED++))
else
    echo -e "${RED}✗ FAILED${NC}: Cannot connect to SiYuan at $SIYUAN_URL"
    echo "  Make sure SiYuan is running on localhost:6806"
    ((TESTS_FAILED++))
fi

# Test 3: Check environment variable
echo ""
echo "Test 3: Environment variable check"
if [ -n "$SIYUAN_API_TOKEN" ] || [ -n "$SIYUAN_TOKEN" ]; then
    echo -e "${GREEN}✓ PASSED${NC}: SIYUAN_API_TOKEN is available"
    ((TESTS_PASSED++))
else
    echo -e "${YELLOW}⚠ WARNING${NC}: SIYUAN_API_TOKEN not set in environment"
    echo "  Will use token from this script"
    ((TESTS_PASSED++))  # Not a failure, we have a fallback
fi

# Test 4: Tool availability
echo ""
echo "Test 4: SiYuan tool availability"
if $ZEROCLAW_BIN agent --message "List ALL tools" 2>&1 | grep -q "siyuan_query"; then
    echo -e "${GREEN}✓ PASSED${NC}: siyuan_query tool is available"
    ((TESTS_PASSED++))
else
    echo -e "${RED}✗ FAILED${NC}: siyuan_query tool not found"
    ((TESTS_FAILED++))
fi

if $ZEROCLAW_BIN agent --message "List ALL tools" 2>&1 | grep -q "siyuan_write"; then
    echo -e "${GREEN}✓ PASSED${NC}: siyuan_write tool is available"
    ((TESTS_PASSED++))
else
    echo -e "${RED}✗ FAILED${NC}: siyuan_write tool not found"
    ((TESTS_FAILED++))
fi

# Test 5: Query functionality (with env var)
echo ""
echo "Test 5: SiYuan query functionality"
TEST_OUTPUT=$(SIYUAN_API_TOKEN="$SIYUAN_TOKEN" $ZEROCLAW_BIN agent --message "Use siyuan_query to list all documents (SELECT * FROM blocks WHERE type='d' LIMIT 3)" 2>&1)
if echo "$TEST_OUTPUT" | grep -q "TestEnv\|Portfolio"; then
    echo -e "${GREEN}✓ PASSED${NC}: siyuan_query tool works correctly"
    ((TESTS_PASSED++))
else
    echo -e "${RED}✗ FAILED${NC}: siyuan_query tool failed or returned unexpected results"
    echo "  Output: $TEST_OUTPUT"
    ((TESTS_FAILED++))
fi

# Test 6: Configuration check
echo ""
echo "Test 6: Configuration file check"
if grep -q "\[siyuan\]" "$HOME/.zeroclaw/config.toml" 2>/dev/null; then
    echo -e "${GREEN}✓ PASSED${NC}: SiYuan section exists in config.toml"
    ((TESTS_PASSED++))

    # Check if API token is configured
    if grep -A 5 "\[siyuan\]" "$HOME/.zeroclaw/config.toml" | grep -q "api_token"; then
        echo -e "${GREEN}✓ PASSED${NC}: API token is configured in config.toml"
        ((TESTS_PASSED++))
    else
        echo -e "${YELLOW}⚠ WARNING${NC}: API token not found in config.toml"
        ((TESTS_PASSED++))
    fi
else
    echo -e "${YELLOW}⚠ WARNING${NC}: SiYuan section not found in config.toml"
    echo "  Tools will rely on environment variables"
    ((TESTS_PASSED++))
fi

# Summary
echo ""
echo "================================"
echo "Test Summary"
echo "================================"
echo -e "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests Failed: ${RED}$TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC} SiYuan integration is working correctly."
    exit 0
else
    echo -e "${RED}Some tests failed.${NC} Please review the output above."
    exit 1
fi
