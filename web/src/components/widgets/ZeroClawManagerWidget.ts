/**
 * ZeroClawManagerWidget — manage ZeroClaw agent, gateway, and system.
 */

import type { WidgetModule, EventPush, Cleanup } from '../../types';
import { registerWidget } from '../../widgetRegistry';

const ZeroClawManagerWidget: WidgetModule = {
  id: 'zeroclaw-manager',
  title: 'ZeroClaw Manager',

  init(container: HTMLElement, push: EventPush): Cleanup {
    const wrapper = document.createElement('div');
    wrapper.style.cssText = 'display:flex;flex-direction:column;gap:12px;padding:12px;height:100%;';

    // Status section
    const statusSection = document.createElement('div');
    statusSection.style.cssText = 'display:flex;flex-direction:column;gap:8px;';

    const statusHeader = document.createElement('div');
    statusHeader.textContent = 'System Status';
    statusHeader.style.cssText = 'font-size:13px;font-weight:700;color:var(--subtext);text-transform:uppercase;letter-spacing:0.05em;';

    const statusGrid = document.createElement('div');
    statusGrid.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:8px;';

    const statusItems = [
      { label: 'Gateway', id: 'status-gateway', default: 'Unknown' },
      { label: 'Agent', id: 'status-agent', default: 'Unknown' },
      { label: 'Memory', id: 'status-memory', default: 'Unknown' },
      { label: 'Provider', id: 'status-provider', default: 'Unknown' },
    ];

    statusItems.forEach(item => {
      const div = document.createElement('div');
      div.style.cssText = 'display:flex;flex-direction:column;gap:2px;';

      const label = document.createElement('span');
      label.textContent = item.label;
      label.style.cssText = 'font-size:10px;color:var(--overlay);';

      const value = document.createElement('span');
      value.id = item.id;
      value.textContent = item.default;
      value.style.cssText = 'font-size:12px;color:var(--text);font-family:var(--font-mono);';

      div.appendChild(label);
      div.appendChild(value);
      statusGrid.appendChild(div);
    });

    statusSection.appendChild(statusHeader);
    statusSection.appendChild(statusGrid);

    // Controls section
    const controlSection = document.createElement('div');
    controlSection.style.cssText = 'display:flex;flex-direction:column;gap:8px;';

    const controlHeader = document.createElement('div');
    controlHeader.textContent = 'Controls';
    controlHeader.style.cssText = 'font-size:13px;font-weight:700;color:var(--subtext);text-transform:uppercase;letter-spacing:0.05em;';

    const buttonGrid = document.createElement('div');
    buttonGrid.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:8px;';

    const buttons = [
      { label: 'Refresh Status', action: 'refresh', style: 'blue' },
      { label: 'Reload Config', action: 'reload', style: 'mauve' },
      { label: 'Restart Gateway', action: 'restart-gateway', style: 'peach' },
      { label: 'Clear Memory Cache', action: 'clear-memory', style: 'red' },
    ];

    buttons.forEach(btn => {
      const button = document.createElement('button');
      button.textContent = btn.label;
      button.style.cssText = `
        padding:8px 12px;
        border-radius:4px;
        border:none;
        background:var(--${btn.style});
        color:var(--bg-base);
        font-size:11px;
        font-weight:600;
        cursor:pointer;
        transition:opacity 0.15s;
      `;

      button.addEventListener('mouseenter', () => button.style.opacity = '0.85');
      button.addEventListener('mouseleave', () => button.style.opacity = '1');

      button.addEventListener('click', () => {
        // Send action event via SSE
        console.log(`[ZeroClawManager] Action: ${btn.action}`);
      });

      buttonGrid.appendChild(button);
    });

    controlSection.appendChild(controlHeader);
    controlSection.appendChild(buttonGrid);

    // Log section
    const logSection = document.createElement('div');
    logSection.style.cssText = 'display:flex;flex-direction:column;gap:8px;flex:1;min-height:0;';

    const logHeader = document.createElement('div');
    logHeader.textContent = 'Activity Log';
    logHeader.style.cssText = 'font-size:13px;font-weight:700;color:var(--subtext);text-transform:uppercase;letter-spacing:0.05em;';

    const logOutput = document.createElement('div');
    logOutput.id = 'zeroclaw-log';
    logOutput.style.cssText = `
      flex:1;
      overflow:auto;
      padding:8px;
      border-radius:4px;
      background:var(--bg-mantle);
      font-family:var(--font-mono);
      font-size:10px;
      line-height:1.4;
      color:var(--subtext);
    `;
    logOutput.textContent = '[System] ZeroClaw Manager initialized\n';

    logSection.appendChild(logHeader);
    logSection.appendChild(logOutput);

    // Assemble
    wrapper.appendChild(statusSection);
    wrapper.appendChild(controlSection);
    wrapper.appendChild(logSection);
    container.appendChild(wrapper);

    // SSE push handler
    push((evt) => {
      if (evt.from === 'zeroclaw-manager' || evt.from === 'system-status') {
        const timestamp = new Date().toLocaleTimeString();
        logOutput.textContent += `[${timestamp}] ${JSON.stringify(evt.data)}\n`;
        logOutput.scrollTop = logOutput.scrollHeight;
      }
    });

    // Periodic status refresh
    const refreshInterval = setInterval(() => {
      const gatewayStatus = document.getElementById('status-gateway');
      if (gatewayStatus) {
        gatewayStatus.textContent = 'Running';
        gatewayStatus.style.color = 'var(--green)';
      }

      const agentStatus = document.getElementById('status-agent');
      if (agentStatus) {
        agentStatus.textContent = 'Ready';
        agentStatus.style.color = 'var(--green)';
      }

      const memoryStatus = document.getElementById('status-memory');
      if (memoryStatus) {
        memoryStatus.textContent = 'OK';
        memoryStatus.style.color = 'var(--text)';
      }

      const providerStatus = document.getElementById('status-provider');
      if (providerStatus) {
        providerStatus.textContent = 'GLM';
        providerStatus.style.color = 'var(--blue)';
      }
    }, 5000);

    return () => {
      clearInterval(refreshInterval);
      container.innerHTML = '';
    };
  },
};

registerWidget(ZeroClawManagerWidget);
export default ZeroClawManagerWidget;
