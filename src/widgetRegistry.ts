import React from 'react';
import { WidgetConfig } from '../types';
import SystemStatusWidget from './components/widgets/SystemStatusWidget';
import AgentLogWidget from './components/widgets/AgentLogWidget';

// ── Registry ──────────────────────────────────────────────
// Add new widgets here. That's it.

const registry = new Map<string, WidgetConfig>();

export function registerWidget(config: WidgetConfig) {
  registry.set(config.id, config);
}

export function getWidget(id: string): WidgetConfig | undefined {
  return registry.get(id);
}

export function getAllWidgets(): WidgetConfig[] {
  return Array.from(registry.values());
}

// ── Built-in widgets ─────────────────────────────────────

registerWidget({
  id: 'system-status',
  title: 'System Status',
  component: SystemStatusWidget,
  defaultLayout: { i: 'system-status', x: 0, y: 0, w: 6, h: 4, minW: 3, minH: 2 },
});

registerWidget({
  id: 'agent-log',
  title: 'Agent Log',
  component: AgentLogWidget,
  defaultLayout: { i: 'agent-log', x: 6, y: 0, w: 6, h: 6, minW: 3, minH: 3 },
});
