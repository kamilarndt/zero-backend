/**
 * RAF Batching Stress Test — 50 events/second simulation.
 *
 * Run in browser console or via Vite dev server.
 * Measures FPS with and without batching to verify the improvement.
 *
 * Usage:
 *   import { runStressTest } from './sseBus.stress';
 *   runStressTest(); // logs results to console
 */

import { sseBus } from './sseBus';

interface TestResult {
  label: string;
  eventsSent: number;
  eventsReceived: number;
  avgFPS: number;
  minFPS: number;
  maxFPS: number;
  durationMs: number;
  frameDrops: number; // frames below 55
}

function simulateEvents(count: number, intervalMs: number): () => void {
  let sent = 0;
  const timer = setInterval(() => {
    if (sent >= count) return;
    // Inject event directly into the bus via private-ish method
    // We use the public subscribe path by dispatching fake EventSource messages
    const fakeEvent = new MessageEvent('message', {
      data: JSON.stringify({
        from: 'stress-test',
        type: 'telemetry',
        data: { action: 'thinking', content: `tick ${sent}`, status: 'running' },
        timestamp: String(Date.now()),
      }),
    });
    // Access the internal EventSource via reflection (test-only hack)
    const source = (sseBus as any).source;
    if (source?.onmessage) {
      source.onmessage(fakeEvent);
    }
    sent++;
  }, intervalMs);

  return () => clearInterval(timer);
}

/**
 * Run the full stress test suite.
 * Returns results for both "no batching" (individual subscribe) and
 * "batched" (subscribeBatched) modes.
 */
export async function runStressTest(durationSec = 5): Promise<{
  individual: TestResult;
  batched: TestResult;
}> {
  const EVENTS_PER_SEC = 50;
  const totalEvents = EVENTS_PER_SEC * durationSec;
  const intervalMs = 1000 / EVENTS_PER_SEC;

  console.log(`[StressTest] Starting: ${EVENTS_PER_SEC} events/sec for ${durationSec}s (${totalEvents} total)`);

  // ── Test 1: Individual (non-batched) subscribe ────────────
  const individual = await runSingleTest('Individual subscribe', totalEvents, intervalMs, false);

  // Cool down
  await sleep(1000);

  // ── Test 2: Batched subscribe ─────────────────────────────
  const batched = await runSingleTest('Batched subscribe', totalEvents, intervalMs, true);

  // ── Report ────────────────────────────────────────────────
  console.log('\n[StressTest] ═══════════════════════════════════════');
  console.log('[StressTest] RESULTS:');
  console.log('[StressTest] ═══════════════════════════════════════');
  logResult(individual);
  logResult(batched);
  console.log('[StressTest] ═══════════════════════════════════════');

  const fpsImprovement = ((batched.avgFPS - individual.avgFPS) / individual.avgFPS * 100).toFixed(1);
  const dropReduction = individual.frameDrops > 0
    ? ((1 - batched.frameDrops / individual.frameDrops) * 100).toFixed(1)
    : 'N/A';

  console.log(`[StressTest] FPS improvement: ${fpsImprovement}%`);
  console.log(`[StressTest] Frame drop reduction: ${dropReduction}%`);
  console.log(`[StressTest] ═══════════════════════════════════════\n`);

  return { individual, batched };
}

async function runSingleTest(
  label: string,
  totalEvents: number,
  intervalMs: number,
  useBatch: boolean
): Promise<TestResult> {
  return new Promise((resolve) => {
    let eventsReceived = 0;
    const fpsSamples: number[] = [];

    // Subscribe
    let unsub: () => void;
    if (useBatch) {
      unsub = sseBus.subscribeBatched((events) => {
        eventsReceived += events.length;
      });
    } else {
      unsub = sseBus.subscribe(() => {
        eventsReceived++;
      });
    }

    // FPS measurement
    const stopFPS = sseBus.measureFPS((fps) => {
      fpsSamples.push(fps);
    });

    // Fire events
    const stopEvents = simulateEvents(totalEvents, intervalMs);

    // Wait for test duration + buffer
    const testDurationMs = (totalEvents * intervalMs) + 500;
    setTimeout(() => {
      stopEvents();
      unsub();
      stopFPS();

      const avgFPS = fpsSamples.length > 0
        ? Math.round(fpsSamples.reduce((a, b) => a + b, 0) / fpsSamples.length)
        : 0;
      const minFPS = fpsSamples.length > 0 ? Math.min(...fpsSamples) : 0;
      const maxFPS = fpsSamples.length > 0 ? Math.max(...fpsSamples) : 0;
      const frameDrops = fpsSamples.filter((f) => f < 55).length;

      resolve({
        label,
        eventsSent: totalEvents,
        eventsReceived,
        avgFPS,
        minFPS,
        maxFPS,
        durationMs: testDurationMs,
        frameDrops,
      });
    }, testDurationMs);
  });
}

function logResult(r: TestResult) {
  console.log(`\n[StressTest] ── ${r.label} ──`);
  console.log(`[StressTest]   Events sent:     ${r.eventsSent}`);
  console.log(`[StressTest]   Events received: ${r.eventsReceived}`);
  console.log(`[StressTest]   Avg FPS:         ${r.avgFPS}`);
  console.log(`[StressTest]   Min FPS:         ${r.minFPS}`);
  console.log(`[StressTest]   Max FPS:         ${r.maxFPS}`);
  console.log(`[StressTest]   Frame drops:     ${r.frameDrops} (<55fps)`);
  console.log(`[StressTest]   Duration:        ${r.durationMs}ms`);
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

// Auto-run if executed directly
if (typeof window !== 'undefined' && (window as any).__runStressTest) {
  runStressTest();
}
