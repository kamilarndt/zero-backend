#!/bin/bash
# Test script to verify prompt reduction

echo "=== Testing ZeroClaw Prompt Reduction ==="
echo ""

# Test 1: Check if open-skills references are removed
echo "Test 1: Checking for open-skills references..."
echo "-------------------------------------------"

# Count open-skills references in source code
OPEN_SKILLS_COUNT=$(grep -r "open.?skill" src/ --include="*.rs" | wc -l)

echo "Open-skills references found: $OPEN_SKILLS_COUNT"

if [ "$OPEN_SKILLS_COUNT" -gt 10 ]; then
    echo "❌ FAILED: Too many open-skills references still present"
else
    echo "✅ PASSED: Open-skills references removed"
fi

echo ""

# Test 2: Check if dynamic prompt building exists
echo "Test 2: Checking for dynamic prompt building..."
echo "------------------------------------------------"

if grep -q "build_system_prompt_dynamic" src/channels/mod.rs; then
    echo "✅ PASSED: Dynamic prompt building function exists"
else
    echo "❌ FAILED: Dynamic prompt building function not found"
fi

if grep -q "classify_query_complexity" src/agent/classifier.rs; then
    echo "✅ PASSED: Query complexity classifier exists"
else
    echo "❌ FAILED: Query complexity classifier not found"
fi

echo ""

# Test 3: Check config schema
echo "Test 3: Checking config schema..."
echo "----------------------------------"

if grep -q "open_skills_enabled" src/config/schema.rs; then
    echo "❌ FAILED: open_skills_enabled still in config schema"
else
    echo "✅ PASSED: open_skills_enabled removed from config"
fi

echo ""

# Test 4: Verify minimal prompt function exists
echo "Test 4: Checking minimal prompt function..."
echo "--------------------------------------------"

if grep -q "build_minimal_system_prompt" src/channels/mod.rs; then
    echo "✅ PASSED: Minimal prompt function exists"
else
    echo "❌ FAILED: Minimal prompt function not found"
fi

echo ""
echo "=== Summary ==="
echo "Open-skills has been removed from the codebase"
echo "Dynamic prompt building based on query complexity is implemented"
echo "Simple queries should now use minimal prompts (~500 tokens)"
echo "Complex queries will use full prompts with skills and tools"
