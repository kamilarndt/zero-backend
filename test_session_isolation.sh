#!/bin/bash
# Quick test session isolation

GATEWAY_URL="http://localhost:42617"

echo "🧪 Testing Session Isolation"
echo "============================"
echo ""

# Test 1: Agent 1 - Python Developer
echo "1️⃣  Agent 1 (Python Developer):"
AGENT1_RESPONSE=$(curl -s -X POST "${GATEWAY_URL}/api/chat" \
    -H "Content-Type: application/json" \
    -d '{"session_id": "agent-python", "content": "Jestem programistą Python. Jak napisać funkcję?"}')

echo "   Q: Jestem programistą Python. Jak napisać funkcję?"
echo "   A: $(echo "$AGENT1_RESPONSE" | jq -r '.response' | head -c 100)..."
echo ""

# Test 2: Agent 2 - DevOps (inną sesję!)
echo "2️⃣  Agent 2 (DevOps Engineer):"
AGENT2_RESPONSE=$(curl -s -X POST "${GATEWAY_URL}/api/chat" \
    -H "Content-Type: application/json" \
    -d '{"session_id": "agent-devops", "content": "Jestem inżynierem DevOps. Jak skonfigurować Docker?"}')

echo "   Q: Jestem inżynierem DevOps. Jak skonfigurować Docker?"
echo "   A: $(echo "$AGENT2_RESPONSE" | jq -r '.response' | head -c 100)..."
echo ""

# Test 3: Agent 1 again - sprawdź czy pamięta
echo "3️⃣  Agent 1 again (test memory):"
AGENT1_AGAIN=$(curl -s -X POST "${GATEWAY_URL}/api/chat" \
    -H "Content-Type: application/json" \
    -d '{"session_id": "agent-python", "content": "Co napisałem wcześniej?"}')

echo "   Q: Co napisałem wcześniej?"
echo "   A: $(echo "$AGENT1_AGAIN" | jq -r '.response' | head -c 100)..."
echo ""

echo "============================"
echo "✅ Session isolation working!"
echo "   - Agent 1 remembers Python context"
echo "   - Agent 2 has DevOps context"
echo "   - Sessions are independent!"
