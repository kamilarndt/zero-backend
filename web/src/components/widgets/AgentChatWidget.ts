/**
 * AgentChatWidget — ZeroClaw chat interface with streaming responses.
 * Adapted from OpenSpace ZeroClawAgentWidget to v0.2 push-based architecture.
 */

import type { WidgetModule, EventPush, Cleanup, SSEEvent } from '../../types';
import { registerWidget } from '../../widgetRegistry';

const GATEWAY_URL = '/api/agent';
const DEFAULT_MODEL = 'zeroclaw-auto-router';

interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  toolCalls?: Array<{
    id: string;
    function?: {
      name: string;
      arguments: string;
    };
  }>;
}

const AgentChatWidget: WidgetModule = {
  id: 'agent-chat',
  title: 'Agent Chat',
  span: 'col-span-2 row-span-2',

  init(container: HTMLElement, push: EventPush): Cleanup {
    const wrapper = document.createElement('div');
    wrapper.style.cssText = 'display:flex;flex-direction:column;gap:12px;padding:12px;height:100%;';

    // Messages container
    const messagesContainer = document.createElement('div');
    messagesContainer.className = 'chat-messages';
    messagesContainer.style.cssText = `
      flex:1;
      overflow-y:auto;
      display:flex;
      flex-direction:column;
      gap:8px;
      min-height:0;
    `;

    // Typing indicator
    const typingIndicator = document.createElement('div');
    typingIndicator.className = 'chat-typing';
    typingIndicator.style.cssText = `
      display:none;
      padding:8px 12px;
      background:var(--bg-mantle);
      border-radius:6px;
      font-size:11px;
      color:var(--overlay);
    `;
    typingIndicator.textContent = 'Agent pisze...';

    // Input form
    const form = document.createElement('form');
    form.className = 'chat-form';
    form.style.cssText = `
      display:flex;
      gap:8px;
      padding-top:8px;
      border-top:1px solid var(--border);
    `;

    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'chat-input';
    input.placeholder = 'Zadaj pytanie agentowi...';
    input.autocomplete = 'off';
    input.style.cssText = `
      flex:1;
      padding:8px 12px;
      border-radius:6px;
      border:1px solid var(--border);
      background:var(--bg-surface);
      color:var(--text);
      font-family:var(--font-mono);
      font-size:12px;
      outline:none;
    `;
    input.addEventListener('focus', () => input.style.borderColor = 'var(--blue)');
    input.addEventListener('blur', () => input.style.borderColor = 'var(--border)');

    const submitBtn = document.createElement('button');
    submitBtn.type = 'submit';
    submitBtn.className = 'chat-submit';
    submitBtn.textContent = '➤';
    submitBtn.style.cssText = `
      padding:8px 16px;
      border-radius:6px;
      border:none;
      background:var(--blue);
      color:var(--bg-base);
      font-size:16px;
      cursor:pointer;
      transition:opacity 0.15s;
    `;
    submitBtn.addEventListener('mouseenter', () => submitBtn.style.opacity = '0.85');
    submitBtn.addEventListener('mouseleave', () => submitBtn.style.opacity = '1');

    form.appendChild(input);
    form.appendChild(submitBtn);

    wrapper.appendChild(messagesContainer);
    wrapper.appendChild(typingIndicator);
    wrapper.appendChild(form);
    container.appendChild(wrapper);

    // State
    let messages: ChatMessage[] = [];
    let isTyping = false;

    // Initial greeting
    const addMessage = (msg: ChatMessage) => {
      messages.push(msg);
      renderMessages();
      scrollToBottom();
    };

    const renderMessages = () => {
      messagesContainer.innerHTML = messages.map((msg) => {
        const isUser = msg.role === 'user';
        const toolCallsHtml = msg.toolCalls?.length
          ? `<div style="margin-top:4px;display:flex;gap:4px;flex-wrap:wrap;">
              ${msg.toolCalls.map(tc => `<span style="padding:2px 8px;background:var(--mauve);color:var(--bg-base);border-radius:4px;font-size:10px;">⚡ ${tc.function?.name || 'tool'}</span>`).join('')}
             </div>`
          : '';

        return `
          <div style="padding:8px 12px;border-radius:6px;background:${isUser ? 'var(--bg-overlay)' : 'var(--bg-mantle)'};border-left:2px solid ${isUser ? 'var(--blue)' : 'var(--green)'};">
            <div style="font-size:10px;font-weight:700;color:var(--subtext);margin-bottom:4px;">${isUser ? 'Ty' : 'Agent'}</div>
            <div style="font-size:12px;line-height:1.4;color:var(--text);white-space:pre-wrap;word-break:break-word;">${escapeHtml(msg.content)}</div>
            ${toolCallsHtml}
          </div>
        `;
      }).join('');
    };

    const scrollToBottom = () => {
      messagesContainer.scrollTop = messagesContainer.scrollHeight;
    };

    const sendMessage = async (text: string) => {
      if (isTyping) return;

      // Add user message
      addMessage({ role: 'user', content: text });

      isTyping = true;
      typingIndicator.style.display = 'block';
      input.disabled = true;
      submitBtn.disabled = true;

      try {
        const response = await fetch(GATEWAY_URL, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            message: text,
            model: DEFAULT_MODEL,
            history: messages.slice(0, -1), // Exclude the user message we just added
          }),
        });

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        // Streaming response
        const reader = response.body?.getReader();
        if (!reader) throw new Error('No response body');

        const decoder = new TextDecoder();
        let assistantMessage = '';
        let messageId = messages.length;

        // Add placeholder for assistant message
        messages.push({ role: 'assistant', content: '' });
        renderMessages();

        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          const chunk = decoder.decode(value, { stream: true });
          assistantMessage += chunk;

          // Update message incrementally
          messages[messageId].content = assistantMessage;
          renderMessages();
          scrollToBottom();
        }

      } catch (error) {
        const message = error instanceof Error ? error.message : 'Unknown error';
        addMessage({
          role: 'system',
          content: `⚠️ Błąd: ${message}`,
        });
      } finally {
        isTyping = false;
        typingIndicator.style.display = 'none';
        input.disabled = false;
        submitBtn.disabled = false;
        input.focus();
      }
    };

    // Form submit
    form.addEventListener('submit', (e) => {
      e.preventDefault();
      const text = input.value.trim();
      if (!text || isTyping) return;

      sendMessage(text);
      input.value = '';
    });

    // SSE push handler (for future events from backend)
    push((evt: SSEEvent) => {
      if (evt.from === 'agent-chat') {
        console.log('[AgentChat] SSE event:', evt);
        // Handle SSE events if needed
      }
    });

    // Initial greeting
    addMessage({
      role: 'assistant',
      content: 'ZeroClaw Agent gotowy. Zadaj mi pytanie lub poproś o wykonanie zadania.',
    });

    return () => {
      container.innerHTML = '';
    };
  },
};

function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

registerWidget(AgentChatWidget);
export default AgentChatWidget;
