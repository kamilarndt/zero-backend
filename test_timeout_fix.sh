#!/bin/bash
# Test timeout fix with cache clearing

echo "🧪 Testing ZeroClaw Timeout Fix"
echo "======================================"
echo ""

# Step 1: Kill any existing processes
echo "1️⃣ Cleaning up..."
pkill -9 -f zeroclaw 2>/dev/null || true
sleep 1

# Step 2: Start gateway
echo "2️⃣ Starting Gateway..."
./target/release/zeroclaw gateway --port 42617 > ~/storage/gateway.log 2>&1 &
GATEWAY_PID=$!
echo $GATEWAY_PID > ~/storage/gateway.pid
echo "   Gateway PID: $GATEWAY_PID"
sleep 2

# Verify gateway is running
if curl -s http://127.0.0.1:42617/health | grep -q "ok"; then
    echo "   ✅ Gateway is healthy"
else
    echo "   ❌ Gateway failed to start"
    cat ~/storage/gateway.log
    exit 1
fi

# Step 3: Test message with timeout (should work now with 180s timeout)
echo ""
echo "3️⃣ Testing message send (180s timeout)..."
START_TIME=$(date +%s)

# Send a test message via API
RESPONSE=$(curl -s -X POST http://127.0.0.1:42617/api/chat \
    -H "Content-Type: application/json" \
    -d '{
        "session_id": "test-timeout",
        "content": "Say hello quickly",
        "agent_hint": "fast"
    }' 2>&1 &)

CURL_PID=$!
echo "   Waiting for response (max 60s)..."
echo "   Started at: $(date +%H:%M:%S)"

# Wait for response with timeout
for i in {1..60}; do
    if [ ! -d /proc/$CURL_PID ] 2>/dev/null; then
        END_TIME=$(date +%s)
        ELAPSED=$((END_TIME - START_TIME))
        echo "   ✅ Response received in ${ELAPSED}s"
        break
    fi
    sleep 1
    echo -n "."
done

# Check if curl is still running (timed out)
if [ -d /proc/$CURL_PID ] 2>/dev/null; then
    echo ""
    echo "   ❌ Request still running after 60s - killing"
    kill $CURL_PID 2>/dev/null
fi

echo ""
echo "4️⃣ Response preview:"
echo "$RESPONSE" | head -10

# Step 5: Cleanup
echo ""
echo "5️⃣ Cleanup..."
kill $GATEWAY_PID 2>/dev/null
rm ~/storage/gateway.pid
echo "   ✅ Cleaned up"

echo ""
echo "======================================"
echo "✅ Test Complete!"
echo ""
echo "Timeout configurations:"
echo "  TUI Client:     180s"
echo "  GLM Provider:   120s"
echo ""
echo "If you still see timeouts, clear the cache by running:"
echo "  cargo clean && cargo build --release"
