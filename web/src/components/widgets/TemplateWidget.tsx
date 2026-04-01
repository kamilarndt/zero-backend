/**
 * TemplateWidget — Reference implementation for ZeroClaw Dashboard widgets
 *
 * Canonical template demonstrating:
 * - WidgetModule interface: {id, title, span, init(container, push): cleanup}
 * - SSE integration for real-time updates
 * - Fetch /api/v1/connections for connection states
 * - Visual status indicators (green=connected, yellow=degraded, red=failed)
 * - Loading, error, and empty states
 * - Proper cleanup (AbortController, event listeners, RAF)
 * - RAF-batched renders for performance
 * - Responsive CSS with CSS variables
 *
 * Display sections:
 *   1. Provider status cards  2. Channel status list
 *   3. Memory health bars     4. Active sessions counter
 *   5. Gateway heartbeat
 */

import type { WidgetModule, EventPush, Cleanup, SSEEvent } from '../../types';
import { registerWidget } from '../../widgetRegistry';

// ── Types ─────────────────────────────────────────────────────────

interface ConnectionStatus {
  id: string;
  type: 'provider' | 'channel' | 'memory' | 'gateway' | 'session';
  status: 'connected' | 'disconnected' | 'error' | 'rate_limited' | 'healthy' | 'active' | 'idle' | 'degraded';
  message?: string;
  latency?: number;
  details?: Record<string, unknown>;
}

interface ConnectionsResponse {
  success: boolean;
  data: {
    connections: ConnectionStatus[];
    timestamp: string;
    sessions?: { active: number; total: number };
    gateway?: { heartbeat: string; uptime: number };
  };
}

interface WidgetState {
  connections: ConnectionStatus[];
  isLoading: boolean;
  error: string | null;
  lastUpdate: Date | null;
  eventCount: number;
  activeSessions: number;
  totalSessions: number;
  gatewayHeartbeat: string | null;
  gatewayUptime: number;
}

// ── Constants ─────────────────────────────────────────────────────

const API = '/api/v1/connections';
const ID = 'template-widget';
const REFRESH_MS = 30_000;

const CLR: Record<string, string> = {
  connected: 'var(--success)', healthy: 'var(--success)', active: 'var(--success)',
  degraded: 'var(--warning)', disconnected: 'var(--warning)', rate_limited: 'var(--warning)',
  error: 'var(--error)', idle: 'var(--overlay)',
};
const sc = (s: string) => CLR[s] || 'var(--overlay)';

// ── Helpers ───────────────────────────────────────────────────────

function el(tag: string, cls?: string, css?: string): HTMLElement {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (css) e.style.cssText = css;
  return e;
}

function fmtTime(d: Date): string {
  return d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

function fmtUptime(s: number): string {
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
}

// ── DOM Builders ──────────────────────────────────────────────────

function sectionHeader(label: string): HTMLElement {
  const h = el('div', '', 'padding:6px 12px;background:var(--background-secondary);border-bottom:1px solid var(--border);font-weight:600;font-size:11px;color:var(--text);text-transform:uppercase;letter-spacing:.5px');
  h.textContent = label;
  return h;
}

function providerCard(c: ConnectionStatus): HTMLElement {
  const card = el('div', 'tw-card', 'display:flex;flex-direction:column;gap:6px;padding:12px;border-radius:8px;border:1px solid var(--border);background:var(--background-secondary);transition:box-shadow .15s');
  const hdr = el('div', '', 'display:flex;align-items:center;justify-content:space-between');
  const nm = el('span', '', 'font-weight:600;font-size:13px'); nm.textContent = c.id;
  const dot = el('span', '', `width:10px;height:10px;border-radius:50%;background:${sc(c.status)};flex-shrink:0`);
  hdr.append(nm, dot);

  const row = el('div', '', 'display:flex;align-items:center;justify-content:space-between;font-size:11px;color:var(--overlay)');
  const st = el('span', '', `color:${sc(c.status)};font-weight:600;text-transform:uppercase`); st.textContent = c.status;
  const lat = el('span'); lat.textContent = c.latency != null ? `${c.latency}ms` : '';
  row.append(st, lat);

  card.append(hdr, row);
  if (c.message) {
    const msg = el('div', '', 'font-size:11px;color:var(--overlay);overflow:hidden;text-overflow:ellipsis;white-space:nowrap');
    msg.textContent = c.message;
    card.appendChild(msg);
  }
  return card;
}

function channelItem(c: ConnectionStatus): HTMLElement {
  const item = el('div', '', 'display:flex;align-items:center;justify-content:space-between;padding:8px 12px;border-bottom:1px solid var(--border);font-size:13px');
  const left = el('div', '', 'display:flex;align-items:center;gap:8px');
  const dot = el('span', '', `width:8px;height:8px;border-radius:50%;background:${sc(c.status)};flex-shrink:0`);
  const lbl = el('span', '', 'font-weight:500'); lbl.textContent = c.id;
  left.append(dot, lbl);

  const right = el('div', '', 'display:flex;flex-direction:column;align-items:flex-end;gap:2px');
  const st = el('span', '', `font-size:11px;color:${sc(c.status)};text-transform:uppercase;font-weight:600`);
  st.textContent = c.status;
  right.appendChild(st);
  if (c.message) {
    const m = el('span', '', 'font-size:11px;color:var(--overlay);max-width:180px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap');
    m.textContent = c.message;
    right.appendChild(m);
  }
  item.append(left, right);
  return item;
}

function memoryBar(c: ConnectionStatus): HTMLElement {
  const row = el('div', '', 'display:flex;align-items:center;gap:10px;padding:6px 0');
  const lbl = el('span', '', 'font-size:12px;font-weight:500;min-width:80px'); lbl.textContent = c.id;
  const track = el('div', '', 'flex:1;height:6px;border-radius:3px;background:var(--border);overflow:hidden');
  const pct = (c.details?.usage_pct as number) ?? 0;
  const barColor = pct > 85 ? 'var(--error)' : pct > 60 ? 'var(--warning)' : 'var(--success)';
  const bar = el('div', '', `height:100%;width:${Math.min(100, Math.max(0, pct))}%;border-radius:3px;background:${barColor};transition:width .4s`);
  track.appendChild(bar);
  const val = el('span', '', 'font-size:11px;color:var(--overlay);min-width:36px;text-align:right');
  val.textContent = `${Math.round(pct)}%`;
  row.append(lbl, track, val);
  return row;
}

function heartbeatDisplay(hb: string | null, uptime: number): HTMLElement {
  const row = el('div', '', 'display:flex;align-items:center;justify-content:space-between;padding:10px 12px;border-radius:8px;background:var(--background-secondary);border:1px solid var(--border);font-size:12px');
  const left = el('div', '', 'display:flex;align-items:center;gap:8px');
  const pulse = el('span', 'tw-pulse', `width:10px;height:10px;border-radius:50%;background:${hb ? 'var(--success)' : 'var(--error)'};${hb ? 'animation:tw-pulse 2s ease-in-out infinite;' : ''}`);
  const lbl = el('span', '', 'font-weight:500'); lbl.textContent = hb ? `Gateway alive · ${hb}` : 'Gateway unreachable';
  left.append(pulse, lbl);
  const up = el('span', '', 'color:var(--overlay)'); up.textContent = uptime > 0 ? `Uptime: ${fmtUptime(uptime)}` : '';
  row.append(left, up);
  return row;
}

function sessionsBadge(active: number, total: number): HTMLElement {
  const badge = el('div', '', 'display:flex;align-items:center;gap:12px;padding:10px 12px;border-radius:8px;background:var(--background-secondary);border:1px solid var(--border);font-size:13px');
  const icon = el('span', '', 'font-size:18px'); icon.textContent = '🔗';
  const text = el('div', '', 'display:flex;flex-direction:column');
  const cnt = el('span', '', 'font-weight:700;font-size:16px'); cnt.textContent = `${active}`;
  const desc = el('span', '', 'font-size:11px;color:var(--overlay)'); desc.textContent = `active sessions (${total} total)`;
  text.append(cnt, desc);
  badge.append(icon, text);
  return badge;
}

function loadingState(): HTMLElement {
  const w = el('div', '', 'display:flex;flex-direction:column;align-items:center;justify-content:center;gap:12px;padding:40px 20px;color:var(--overlay)');
  const sp = el('div', '', 'width:28px;height:28px;border:3px solid var(--border);border-top-color:var(--primary);border-radius:50%;animation:tw-spin .8s linear infinite');
  const t = el('p', '', 'margin:0;font-size:13px'); t.textContent = 'Loading connection states…';
  w.append(sp, t);
  return w;
}

function errorState(msg: string, onRetry: () => void): HTMLElement {
  const w = el('div', '', 'display:flex;flex-direction:column;align-items:center;justify-content:center;gap:10px;padding:40px 20px;color:var(--error);text-align:center');
  const ic = el('div', '', 'font-size:28px'); ic.textContent = '⚠️';
  const t = el('p', '', 'margin:0;font-size:13px'); t.textContent = msg;
  const btn = document.createElement('button');
  btn.textContent = 'Retry'; btn.className = 'btn btn-secondary btn-small';
  btn.style.cssText = 'padding:4px 14px;font-size:12px;cursor:pointer';
  btn.addEventListener('click', onRetry);
  w.append(ic, t, btn);
  return w;
}

// ── Widget ────────────────────────────────────────────────────────

const TemplateWidget: WidgetModule = {
  id: ID,
  title: 'System Overview',
  span: 'col-span-2',

  init(container: HTMLElement, push: EventPush): Cleanup {
    const state: WidgetState = {
      connections: [], isLoading: true, error: null, lastUpdate: null,
      eventCount: 0, activeSessions: 0, totalSessions: 0,
      gatewayHeartbeat: null, gatewayUptime: 0,
    };

    const ac = new AbortController();
    let timer: ReturnType<typeof setInterval> | null = null;
    let raf: number | null = null;

    /** RAF-batched render — prevents layout thrashing on rapid SSE updates */
    function scheduleRender(): void {
      if (raf != null) return;
      raf = requestAnimationFrame(() => { raf = null; render(); });
    }

    function render(): void {
      container.innerHTML = '';

      // Header bar
      const hdr = el('div', '', 'display:flex;justify-content:space-between;align-items:center;padding:6px 12px;border-bottom:1px solid var(--border);background:var(--background-secondary);font-size:11px;color:var(--overlay)');
      const ts = el('span'); ts.textContent = state.lastUpdate ? `Last update: ${fmtTime(state.lastUpdate)}` : 'Initializing…';
      const ev = el('span'); ev.textContent = `Events: ${state.eventCount}`;
      hdr.append(ts, ev);
      container.appendChild(hdr);

      // Content
      const body = el('div', '', 'max-height:520px;overflow-y:auto');
      if (state.isLoading) {
        body.appendChild(loadingState());
      } else if (state.error) {
        body.appendChild(errorState(state.error, fetchConnections));
      } else if (state.connections.length === 0) {
        const e = el('div', '', 'display:flex;align-items:center;justify-content:center;padding:40px 20px;color:var(--overlay);font-size:13px');
        e.textContent = 'No connection data available';
        body.appendChild(e);
      } else {
        renderSections(body);
      }
      container.appendChild(body);
    }

    function renderSections(body: HTMLElement): void {
      const grouped = state.connections.reduce((acc, c) => {
        (acc[c.type] ??= []).push(c);
        return acc;
      }, {} as Record<string, ConnectionStatus[]>);

      // 1. Provider cards
      if (grouped.provider?.length) {
        body.appendChild(sectionHeader('Providers'));
        const grid = el('div', '', 'display:grid;grid-template-columns:repeat(auto-fill,minmax(180px,1fr));gap:8px;padding:8px 12px');
        for (const c of grouped.provider) grid.appendChild(providerCard(c));
        body.appendChild(grid);
      }

      // 2. Channel list
      if (grouped.channel?.length) {
        body.appendChild(sectionHeader('Channels'));
        for (const c of grouped.channel) body.appendChild(channelItem(c));
      }

      // 3. Memory health
      if (grouped.memory?.length) {
        body.appendChild(sectionHeader('Memory Health'));
        const wrap = el('div', '', 'padding:4px 12px 8px');
        for (const c of grouped.memory) wrap.appendChild(memoryBar(c));
        body.appendChild(wrap);
      }

      // 4. Sessions counter
      if (state.activeSessions > 0 || state.totalSessions > 0) {
        body.appendChild(sectionHeader('Sessions'));
        const w = el('div', '', 'padding:4px 12px 8px');
        w.appendChild(sessionsBadge(state.activeSessions, state.totalSessions));
        body.appendChild(w);
      }

      // 5. Gateway heartbeat
      if (grouped.gateway?.length || state.gatewayHeartbeat) {
        body.appendChild(sectionHeader('Gateway'));
        const w = el('div', '', 'padding:4px 12px 8px');
        w.appendChild(heartbeatDisplay(state.gatewayHeartbeat, state.gatewayUptime));
        body.appendChild(w);
      }
    }

    // ── Data fetching ─────────────────────────────────────────

    async function fetchConnections(): Promise<void> {
      try {
        state.isLoading = true;
        scheduleRender();
        const res = await fetch(API, { signal: ac.signal, headers: { 'Content-Type': 'application/json' } });
        if (!res.ok) throw new Error(`HTTP ${res.status}: ${res.statusText}`);
        const json: ConnectionsResponse = await res.json();
        if (!json.success || !json.data?.connections) throw new Error('Invalid API response');

        state.connections = json.data.connections;
        state.error = null;
        state.lastUpdate = new Date(json.data.timestamp);
        state.activeSessions = json.data.sessions?.active ?? 0;
        state.totalSessions  = json.data.sessions?.total  ?? 0;
        state.gatewayHeartbeat = json.data.gateway?.heartbeat ?? null;
        state.gatewayUptime    = json.data.gateway?.uptime    ?? 0;
      } catch (err) {
        if ((err as Error).name === 'AbortError') return;
        state.error = (err as Error).message || 'Failed to fetch connections';
        console.error(`[${ID}] fetch error:`, err);
      } finally {
        state.isLoading = false;
        scheduleRender();
      }
    }

    // ── SSE handler ───────────────────────────────────────────

    function handleSSE(evt: SSEEvent): void {
      state.eventCount++;
      switch (evt.type) {
        case 'connection_update': {
          const u = evt.data as ConnectionStatus;
          const i = state.connections.findIndex(c => c.id === u.id);
          if (i !== -1) state.connections[i] = u; else state.connections.push(u);
          state.lastUpdate = new Date();
          scheduleRender();
          break;
        }
        case 'session_update': {
          const d = evt.data as { active?: number; total?: number };
          if (d.active != null) state.activeSessions = d.active;
          if (d.total  != null) state.totalSessions  = d.total;
          state.lastUpdate = new Date();
          scheduleRender();
          break;
        }
        case 'gateway_heartbeat': {
          const d = evt.data as { heartbeat?: string; uptime?: number };
          state.gatewayHeartbeat = d.heartbeat ?? state.gatewayHeartbeat;
          state.gatewayUptime    = d.uptime    ?? state.gatewayUptime;
          state.lastUpdate = new Date();
          scheduleRender();
          break;
        }
        case 'error':
          state.error = (evt.data as { message?: string })?.message || 'Unknown error';
          scheduleRender();
          break;
        default:
          console.debug(`[${ID}] event:`, evt.type);
          fetchConnections();
      }
    }

    // ── Lifecycle ─────────────────────────────────────────────

    // Inject CSS once
    if (!document.getElementById('tw-styles')) {
      const s = document.createElement('style');
      s.id = 'tw-styles';
      s.textContent = `@keyframes tw-spin{to{transform:rotate(360deg)}}@keyframes tw-pulse{0%,100%{opacity:1;transform:scale(1)}50%{opacity:.5;transform:scale(.85)}}.tw-card:hover{box-shadow:0 2px 8px rgba(0,0,0,.08)}`;
      document.head.appendChild(s);
    }

    push(handleSSE);
    fetchConnections();
    timer = setInterval(fetchConnections, REFRESH_MS);

    // Cleanup — AbortController, interval, RAF, DOM
    return function cleanup(): void {
      ac.abort();
      if (timer != null) { clearInterval(timer); timer = null; }
      if (raf != null) { cancelAnimationFrame(raf); raf = null; }
      container.innerHTML = '';
    };
  },
};

registerWidget(TemplateWidget);
export default TemplateWidget;
