/**
 * SystemStatusWidget — push-based module.
 * Displays raw JSON from SSE events.
 */

import type { WidgetModule, EventPush, Cleanup, SSEEvent } from '../../types';
import { registerWidget } from '../../widgetRegistry';

const SystemStatusWidget: WidgetModule = {
  id: 'system-status',
  title: 'System Status',

  init(container: HTMLElement, push: EventPush): Cleanup {
    let latest: SSEEvent | null = null;
    let count = 0;

    function render() {
      container.innerHTML = '';

      if (latest) {
        const pre = document.createElement('pre');
        pre.style.cssText = 'margin:0;white-space:pre-wrap;';
        pre.textContent = JSON.stringify(latest.data, null, 2);
        container.appendChild(pre);
      } else {
        const p = document.createElement('p');
        p.style.cssText = 'color:var(--overlay);';
        p.textContent = 'Waiting for data...';
        container.appendChild(p);
      }

      const counter = document.createElement('div');
      counter.style.cssText = 'margin-top:8px;color:var(--overlay);font-size:11px;';
      counter.textContent = `Events: ${count}`;
      container.appendChild(counter);
    }

    push((evt) => {
      latest = evt;
      count++;
      render();
    });

    render();
    return () => { container.innerHTML = ''; };
  },
};

registerWidget(SystemStatusWidget);
export default SystemStatusWidget;
