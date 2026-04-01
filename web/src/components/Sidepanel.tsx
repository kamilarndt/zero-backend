/**
 * Sidepanel - Fixed right sidebar with chat interface
 * Priority #1 component for Career Dashboard
 */

import React, { useRef, useEffect, KeyboardEvent } from 'react';
import { useChatMessages, useConnectionStatus, ChatMessage } from '../hooks/useChatMessages';
import '../styles/dashboard.css';

export interface SidepanelProps {
  isOpen: boolean;
  onClose: () => void;
  widgetId?: string;
  title?: string;
  agentName?: string;
}

export const Sidepanel: React.FC<SidepanelProps> = ({
  isOpen,
  onClose,
  widgetId = 'career-concierge',
  title = 'Career Concierge',
  agentName = 'CC',
}) => {
  const { messages, isTyping, sendMessage } = useChatMessages({ widgetId });
  const { isConnected, isConnecting } = useConnectionStatus();
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, isTyping]);

  // Handle keyboard shortcuts
  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      handleSend();
    }
  };

  // Auto-resize textarea
  const handleInput = (e: React.FormEvent<HTMLTextAreaElement>) => {
    const textarea = e.currentTarget;
    textarea.style.height = 'auto';
    textarea.style.height = `${Math.min(textarea.scrollHeight, 120)}px`;
  };

  const handleSend = async () => {
    const textarea = textareaRef.current;
    if (!textarea || !textarea.value.trim()) return;

    const content = textarea.value.trim();
    textarea.value = '';
    textarea.style.height = 'auto';

    await sendMessage(content, `/v1/widgets/${widgetId}`);
  };

  return (
    <div className={`sidepanel ${isOpen ? '' : 'collapsed'}`}>
      <div className="sidepanel-header">
        <div className="sidepanel-title">
          <span>{title}</span>
          <span className="sidepanel-agent-badge">{agentName}</span>
        </div>
        <button
          className="sidepanel-close"
          onClick={onClose}
          aria-label="Close sidepanel"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      </div>

      <div className="sidepanel-content">
        <div className="chat-container">
          <div className="chat-messages">
            {messages.length === 0 && (
              <div className="empty-state">
                <div className="empty-state-icon">💬</div>
                <div className="empty-state-text">
                  Start a conversation with {title}
                </div>
              </div>
            )}

            {messages.map((message) => (
              <MessageBubble key={message.id} message={message} />
            ))}

            {isTyping && (
              <div className="typing-indicator">
                <div className="typing-dots">
                  <div className="typing-dot" />
                  <div className="typing-dot" />
                  <div className="typing-dot" />
                </div>
                <span className="typing-label">
                  {title} is thinking...
                </span>
              </div>
            )}

            <div ref={messagesEndRef} />
          </div>

          <div className="chat-input-container">
            <div className="chat-input-wrapper">
              <textarea
                ref={textareaRef}
                className="chat-textarea"
                placeholder="Type a message..."
                rows={1}
                onKeyDown={handleKeyDown}
                onInput={handleInput}
                disabled={isTyping}
                aria-label="Chat input"
              />
              <button
                className="chat-send-btn"
                onClick={handleSend}
                disabled={isTyping}
                aria-label="Send message"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <line x1="22" y1="2" x2="11" y2="13" />
                  <polygon points="22 2 15 22 11 13 2 9 22 2" />
                </svg>
              </button>
            </div>
            <div className="chat-shortcut">Ctrl + Enter to send</div>
          </div>
        </div>
      </div>

      {/* Connection status indicator in header */}
      <div
        className="connection-status"
        style={{ position: 'absolute', top: '12px', right: '60px' }}
      >
        <div
          className={`connection-status-dot${
            isConnected ? '' : ' connecting'
          }`}
        />
        <span>{isConnected ? 'Connected' : isConnecting ? 'Connecting...' : 'Disconnected'}</span>
      </div>
    </div>
  );
};

interface MessageBubbleProps {
  message: ChatMessage;
}

const MessageBubble: React.FC<MessageBubbleProps> = ({ message }) => {
  const formatTime = (date: Date) => {
    return date.toLocaleTimeString('en-US', {
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  return (
    <div className={`message-bubble ${message.from}`}>
      {message.from === 'agent' && message.agent && (
        <div className="message-agent-label">
          {message.agent === 'career-concierge' ? 'Career Concierge' :
           message.agent === 'graphic-designer' ? 'Graphic Designer' :
           message.agent}
        </div>
      )}
      <div className="message-content">
        {message.content}
      </div>
      <div className="message-timestamp">
        {formatTime(message.timestamp)}
        {message.status === 'error' && (
          <span className="message-status error"> • Failed to send</span>
        )}
        {message.status === 'sending' && (
          <span className="message-status"> • Sending...</span>
        )}
      </div>
    </div>
  );
};

export default Sidepanel;
