#!/usr/bin/env python3
"""
Test WebSocket concurrency for ZeroClaw
Sprawdza czy wiele WebSocket connections może działać jednocześnie
"""

import asyncio
import websockets
import json
import time
from typing import List

async def test_single_websocket(client_id: int, port: int = 42617) -> float:
    """Test pojedynczej WebSocket sesji"""
    uri = f"ws://localhost:{port}/ws/chat"
    messages_sent = 0
    start_time = time.time()

    try:
        async with websockets.connect(uri) as websocket:
            # Send 10 messages
            for i in range(10):
                message = {
                    "type": "message",
                    "content": f"Client {client_id} - Message {i}"
                }

                try:
                    await websocket.send(json.dumps(message))
                    response = await asyncio.wait_for(
                        websocket.recv(),
                        timeout=10.0
                    )
                    messages_sent += 1
                except asyncio.TimeoutError:
                    print(f"⚠️  Client {client_id}: Timeout on message {i}")
                    break
                except Exception as e:
                    print(f"❌ Client {client_id}: Error on message {i}: {e}")
                    break

    except Exception as e:
        print(f"❌ Client {client_id}: Connection failed: {e}")
        return 0.0

    elapsed = time.time() - start_time
    return elapsed

async def test_sequential_websockets(num_clients: int = 5) -> float:
    """Test sekwencyjnych WebSocket connections"""
    print(f"2️⃣  Testing SEQUENTIAL WebSocket connections ({num_clients} clients)...")

    start_time = time.time()
    for client_id in range(num_clients):
        await test_single_websocket(client_id)
    elapsed = time.time() - start_time

    print(f"   ⏱️  Sequential: {elapsed:.2f}s")
    return elapsed

async def test_parallel_websockets(num_clients: int = 5) -> float:
    """Test równoległych WebSocket connections"""
    print(f"3️⃣  Testing PARALLEL WebSocket connections ({num_clients} clients)...")

    start_time = time.time()
    tasks = [test_single_websocket(client_id) for client_id in range(num_clients)]
    await asyncio.gather(*tasks)
    elapsed = time.time() - start_time

    print(f"   ⏱️  Parallel: {elapsed:.2f}s")
    return elapsed

async def main():
    print("🧪 Testing ZeroClaw WebSocket Concurrency")
    print("=" * 60)
    print()

    # Test sequential
    try:
        sequential_time = await test_sequential_websockets(num_clients=5)
    except Exception as e:
        print(f"❌ Sequential test failed: {e}")
        return

    print()

    # Test parallel
    try:
        parallel_time = await test_parallel_websockets(num_clients=5)
    except Exception as e:
        print(f"❌ Parallel test failed: {e}")
        return

    print()
    print("📊 Results:")
    print(f"   Sequential: {sequential_time:.2f}s")
    print(f"   Parallel:   {parallel_time:.2f}s")

    if parallel_time > 0:
        speedup = sequential_time / parallel_time
        print(f"   Speedup:    {speedup:.2f}x"
)
        print()

        if speedup > 2.0:
            print("✅ WEBSOCKET CONCURRENCY WORKING!")
        else:
            print("❌ WEBSOCKET CONCURRENCY BROKEN!")
            print("   🚨 WebSocket sessions are blocking each other!")
            print("   📋 This confirms the config mutex bottleneck!")

    print()
    print("=" * 60)

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n⚠️  Test interrupted")
