/**
 * Push-based SSE client — single shared EventSource, fan-out to subscribers.
 * No React dependency. Pure vanilla JS module.
 *
 * RAF Batching: events are queued and flushed once per animation frame,
 * preventing frame drops at high event rates (50+ events/second).
 */

export interface SSEEvent {
  from: string;
  type: string;
  data: unknown;
  timestamp?: string;
}

type Handler = (event: SSEEvent) => void;
type BatchHandler = (events: SSEEvent[]) => void;

const DEFAULT_ENDPOINT = '/v1/events/stream';

class SSEBus {
  private source: EventSource | null = null;
  private subscribers = new Set<Handler>();
  private batchSubscribers = new Set<BatchHandler>();
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private _connected = false;

  // ── RAF Batching state ──────────────────────────────────────
  private eventQueue: SSEEvent[] = [];
  private pendingRender = false;
  private rafId: number | null = null;

  get isConnected() {
    return this._connected;
  }

  /** Subscribe to all events. Returns unsubscribe function. */
  subscribe(handler: Handler): () => void {
    this.subscribers.add(handler);
    this.ensureConnected();
    return () => {
      this.subscribers.delete(handler);
      if (this.subscribers.size === 0 && this.batchSubscribers.size === 0) this.disconnect();
    };
  }

  /**
   * Subscribe with RAF batching — handler receives an array of events
   * accumulated since the last frame. Use this for render-heavy handlers.
   * Returns unsubscribe function.
   */
  subscribeBatched(handler: BatchHandler): () => void {
    this.batchSubscribers.add(handler);
    this.ensureConnected();
    return () => {
      this.batchSubscribers.delete(handler);
      if (this.subscribers.size === 0 && this.batchSubscribers.size === 0) this.disconnect();
    };
  }

  /** Subscribe only to events from a specific widget. */
  subscribeWidget(widgetId: string, handler: Handler): () => void {
    return this.subscribe((evt) => {
      if (evt.from === widgetId) handler(evt);
    });
  }

  /** Subscribe widget with RAF batching. */
  subscribeWidgetBatched(widgetId: string, handler: BatchHandler): () => void {
    return this.subscribeBatched((events) => {
      const filtered = events.filter((evt) => evt.from === widgetId);
      if (filtered.length > 0) handler(filtered);
    });
  }

  private ensureConnected() {
    if (this.source) return;

    const es = new EventSource(DEFAULT_ENDPOINT);
    this.source = es;

    es.onopen = () => {
      this._connected = true;
      this.notifyConnectionChange();
    };

    es.onmessage = (e) => {
      try {
        const parsed: SSEEvent = JSON.parse(e.data);
        this.enqueueEvent(parsed);
      } catch {
        // ignore malformed
      }
    };

    es.onerror = () => {
      this._connected = false;
      this.notifyConnectionChange();
      es.close();
      this.source = null;
      this.reconnectTimer = setTimeout(() => this.ensureConnected(), 3000);
    };
  }

  // ── RAF Batching internals ──────────────────────────────────

  /** Queue an event and schedule a flush on the next animation frame. */
  private enqueueEvent(event: SSEEvent): void {
    this.eventQueue.push(event);

    if (!this.pendingRender) {
      this.pendingRender = true;
      this.rafId = requestAnimationFrame(() => this.flushQueue());
    }
  }

  /** Flush all queued events to subscribers in a single batch. */
  private flushQueue(): void {
    this.pendingRender = false;
    this.rafId = null;

    if (this.eventQueue.length === 0) return;

    // Swap queue — events that arrive during flush go to next frame
    const batch = this.eventQueue;
    this.eventQueue = [];

    // 1. Notify batch subscribers with the full array (one call per subscriber)
    for (const sub of this.batchSubscribers) {
      try {
        sub(batch);
      } catch {
        // isolate subscriber errors
      }
    }

    // 2. Notify individual subscribers (one call per event per subscriber)
    for (const evt of batch) {
      for (const sub of this.subscribers) {
        try {
          sub(evt);
        } catch {
          // isolate subscriber errors
        }
      }
    }
  }

  private notifyConnectionChange() {
    // Connection changes are not batched — dispatch immediately
    const evt: SSEEvent = {
      from: '__bus',
      type: this._connected ? 'connected' : 'disconnected',
      data: null,
      timestamp: new Date().toISOString(),
    };
    for (const sub of this.subscribers) sub(evt);
    // Also notify batch subscribers with a single-element array
    for (const sub of this.batchSubscribers) sub([evt]);
  }

  // ── FPS Measurement ───────────────────────────────────────
  private fpsFrames = 0;
  private fpsLastTime = performance.now();
  private fpsValue = 60;
  private fpsCallbacks: Set<(fps: number) => void> = new Set();
  private fpsRafId: number | null = null;

  /** Get current measured FPS. */
  get currentFPS(): number {
    return this.fpsValue;
  }

  /** Start measuring FPS. Returns stop function. */
  measureFPS(onUpdate?: (fps: number) => void): () => void {
    if (onUpdate) this.fpsCallbacks.add(onUpdate);

    if (this.fpsRafId === null) {
      const tick = () => {
        this.fpsFrames++;
        const now = performance.now();
        const delta = now - this.fpsLastTime;
        if (delta >= 1000) {
          this.fpsValue = Math.round((this.fpsFrames * 1000) / delta);
          this.fpsFrames = 0;
          this.fpsLastTime = now;
          for (const cb of this.fpsCallbacks) {
            try { cb(this.fpsValue); } catch {}
          }
        }
        this.fpsRafId = requestAnimationFrame(tick);
      };
      this.fpsRafId = requestAnimationFrame(tick);
    }

    return () => {
      if (onUpdate) this.fpsCallbacks.delete(onUpdate);
      if (this.fpsCallbacks.size === 0 && this.fpsRafId !== null) {
        cancelAnimationFrame(this.fpsRafId);
        this.fpsRafId = null;
      }
    };
  }

  /** Get queue stats for debugging. */
  getQueueStats(): { queueSize: number; pendingRender: boolean } {
    return {
      queueSize: this.eventQueue.length,
      pendingRender: this.pendingRender,
    };
  }

  private disconnect() {
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    if (this.rafId !== null) {
      cancelAnimationFrame(this.rafId);
      this.rafId = null;
    }
    this.pendingRender = false;
    this.eventQueue = [];
    this.source?.close();
    this.source = null;
    this._connected = false;
  }
}

// Singleton
export const sseBus = new SSEBus();
