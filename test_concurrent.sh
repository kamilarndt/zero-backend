#!/bin/bash
# Test concurrency ZeroClaw Gateway
# Sprawdza czy gateway obsługuje jednoczesne requesty

set -e

GATEWAY_PORT=42617
GATEWAY_URL="http://localhost:${GATEWAY_PORT}"
NUM_REQUESTS=10

echo "🧪 Testing ZeroClaw Multi-Session Support"
echo "=========================================="
echo ""

# Check if gateway is running
echo "1️⃣  Checking gateway status..."
if ! curl -s "${GATEWAY_URL}/health" > /dev/null 2>&1; then
    echo "❌ Gateway not running on port ${GATEWAY_PORT}"
    exit 1
fi
echo "✅ Gateway running on port ${GATEWAY_PORT}"
echo ""

# Test 1: Sequential requests (baseline)
echo "2️⃣  Testing sequential requests..."
time_start=$(date +%s%N)
for i in $(seq 1 $NUM_REQUESTS); do
    curl -s "${GATEWAY_URL}/api/status" > /dev/null 2>&1
done
time_end=$(date +%s%N)
sequential_time=$(( ($time_end - $time_start) / 1000000 ))
echo "   ⏱️  Sequential: ${sequential_time}ms"
echo ""

# Test 2: Parallel requests
echo "3️⃣  Testing parallel requests..."
time_start=$(date +%s%N)
for i in $(seq 1 $NUM_REQUESTS); do
    (curl -s "${GATEWAY_URL}/api/status" > /dev/null 2>&1) &
done
wait
time_end=$(date +%s%N)
parallel_time=$(( ($time_end - $time_start) / 1000000 ))
echo "   ⏱️  Parallel: ${parallel_time}ms"
echo ""

# Calculate speedup
if [ $parallel_time -gt 0 ]; then
    speedup=$(echo "scale=2; $sequential_time / $parallel_time" | bc)
    echo "📊 Results:"
    echo "   Sequential: ${sequential_time}ms"
    echo "   Parallel:   ${parallel_time}ms"
    echo "   Speedup:    ${speedup}x"
    echo ""

    # Check if concurrency is working
    if (( $(echo "$speedup > 2.0" | bc -l) )); then
        echo "✅ CONCURRENCY WORKING! Speedup > 2x"
    else
        echo "❌ CONCURRENCY BROKEN! Speedup < 2x"
        echo "   🚨 Gateway processes requests sequentially!"
    fi
else
    echo "❌ Test failed - parallel time was 0ms"
fi

echo ""
echo "=========================================="
echo "Test complete!"
