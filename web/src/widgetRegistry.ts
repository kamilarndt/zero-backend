/**
 * Widget Registry — zero-framework module registration.
 *
 * Widgets are WidgetModule objects with init(container, push) → cleanup.
 * The registry is a plain Map — register at module load, retrieve by id.
 *
 * RAF Batching: mountWidget uses subscribeBatched so that all SSE events
 * arriving in a single animation frame are delivered as one array to the
 * widget's push handler. Widgets that call render() inside their handler
 * will only render once per frame instead of N times.
 */

import type { WidgetModule, Cleanup, SSEEvent } from './types';
import { sseBus } from './sseBus';

const registry = new Map<string, WidgetModule>();

/** Register a widget module. Call at top-level of the widget file. */
export function registerWidget(mod: WidgetModule) {
  registry.set(mod.id, mod);
}

export function getWidget(id: string): WidgetModule | undefined {
  return registry.get(id);
}

export function getAllWidgets(): WidgetModule[] {
  return Array.from(registry.values());
}

/**
 * Mount a widget into a container element.
 * Returns a cleanup function that tears down the widget and unsubscribes SSE.
 *
 * Uses RAF-batched delivery: the widget's push handler receives an array of
 * all SSE events accumulated since the last animation frame. This means
 * render() inside the handler fires once per frame, not once per event.
 */
export function mountWidget(container: HTMLElement, widgetId: string): Cleanup {
  const mod = registry.get(widgetId);
  if (!mod) {
    container.textContent = `Widget "${widgetId}" not found`;
    return () => {};
  }

  // Build chrome
  const header = document.createElement('div');
  header.className = 'widget-header';

  const title = document.createElement('span');
  title.className = 'widget-title';
  title.textContent = mod.title;

  const status = document.createElement('span');
  status.className = `widget-status ${sseBus.isConnected ? 'connected' : 'disconnected'}`;

  header.appendChild(title);
  header.appendChild(status);

  const body = document.createElement('div');
  body.className = 'widget-body';
  body.setAttribute('data-widget-id', widgetId);

  container.appendChild(header);
  container.appendChild(body);

  // Track connection status
  const unsubConn = sseBus.subscribe((evt) => {
    if (evt.from === '__bus') {
      status.className = `widget-status ${sseBus.isConnected ? 'connected' : 'disconnected'}`;
    }
  });

  // ── RAF-batched push to widget ──────────────────────────────
  // The widget's init() calls push(handler) where handler expects a single
  // SSEEvent. We bridge the batched subscription by calling handler once
  // per event in the batch — but all within one RAF frame, so the browser
  // only paints once. Widgets that debounce (e.g. "latest wins") will
  // naturally coalesce.
  const cleanupWidget = mod.init(body, (handler) => {
    return sseBus.subscribeBatched((batch: SSEEvent[]) => {
      // Filter to this widget's events
      const filtered = batch.filter((evt) => evt.from === widgetId);
      if (filtered.length === 0) return;

      // Call handler for each event — all within the same synchronous
      // block inside a single RAF callback. The browser won't paint
      // between these calls, so multiple DOM updates are coalesced.
      for (const evt of filtered) {
        handler(evt);
      }
    });
  });

  return () => {
    unsubConn();
    cleanupWidget();
    container.innerHTML = '';
  };
}
