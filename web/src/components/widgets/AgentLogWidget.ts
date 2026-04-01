/**
 * AgentLogWidget — push-based module.
 * Appends log entries with auto-scroll.
 */

import type { WidgetModule, EventPush, Cleanup } from '../../types';
import { registerWidget } from '../../widgetRegistry';

const MAX_LOG_ENTRIES = 200;

const AgentLogWidget: WidgetModule = {
  id: 'agent-log',
  title: 'Agent Log',
  span: 'row-span-2',

  init(container: HTMLElement, push: EventPush): Cleanup {
    const logContainer = document.createElement('div');
    container.appendChild(logContainer);

    let count = 0;

    function addEntry(data: string, type: string, timestamp?: string) {
      const row = document.createElement('div');
      row.style.cssText = 'margin-bottom:4px;border-bottom:1px solid var(--border);padding-bottom:4px;';

      const ts = document.createElement('span');
      ts.style.color = 'var(--blue)';
      ts.textContent = `[${timestamp ?? '?'}]`;

      const tp = document.createElement('span');
      tp.style.color = 'var(--green)';
      tp.textContent = ` ${type} `;

      const msg = document.createElement('span');
      msg.textContent = typeof data === 'string' ? data : JSON.stringify(data);

      row.appendChild(ts);
      row.appendChild(tp);
      row.appendChild(msg);
      logContainer.appendChild(row);

      // Trim old entries
      while (logContainer.children.length > MAX_LOG_ENTRIES) {
        logContainer.removeChild(logContainer.firstChild!);
      }

      // Auto-scroll
      container.scrollTop = container.scrollHeight;
      count++;
    }

    // Show initial message
    if (count === 0) {
      const p = document.createElement('p');
      p.style.color = 'var(--overlay)';
      p.textContent = 'No log entries yet.';
      logContainer.appendChild(p);
    }

    push((evt) => {
      // Remove "no entries" message on first event
      if (count === 0) {
        logContainer.innerHTML = '';
      }
      addEntry(
        typeof evt.data === 'string' ? evt.data : JSON.stringify(evt.data),
        evt.type,
        evt.timestamp
      );
    });

    return () => { container.innerHTML = ''; };
  },
};

registerWidget(AgentLogWidget);
export default AgentLogWidget;
