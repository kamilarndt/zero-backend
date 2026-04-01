/**
 * SimpleChatWidget - Minimalny widget do czatu z Career Concierge
 * Tylko: connection status + chat interface
 */

import React, { useState, useRef, useEffect } from 'react';
import { createReactWidget } from '../ReactWidgetAdapter';

interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
}

function SimpleChatWidget(): JSX.Element {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [isConnected, setIsConnected] = useState(false);
  const [isTyping, setIsTyping] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Simulate SSE connection (placeholder - replace with real SSE)
  useEffect(() => {
    // TODO: Connect to ZeroClaw gateway SSE endpoint
    // const eventSource = new EventSource('http://localhost:42617/v1/events/stream');
    setIsConnected(true);
  }, []);

  const handleSend = async () => {
    if (!input.trim()) return;

    const userMessage: Message = {
      id: Date.now().toString(),
      role: 'user',
      content: input,
      timestamp: new Date()
    };

    setMessages(prev => [...prev, userMessage]);
    const userInput = input;
    setInput('');
    setIsTyping(true);

    try {
      // Call ZeroClaw widget endpoint
      const response = await fetch('http://localhost:42617/v1/widgets/simple-chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          data: {
            type: 'chat_message',
            content: userInput
          }
        })
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();
      const agentMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: 'assistant',
        content: data.response || data.message || JSON.stringify(data),
        timestamp: new Date()
      };

      setMessages(prev => [...prev, agentMessage]);

    } catch (err) {
      console.error('Failed to send message:', err);

      // Fallback message
      const errorMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: 'assistant',
        content: `⚠️ Error connecting to Career Concierge:\n${err instanceof Error ? err.message : String(err)}\n\nMake sure ZeroClaw gateway is running on port 42617 and career-concierge skill is installed.`,
        timestamp: new Date()
      };

      setMessages(prev => [...prev, errorMessage]);
    } finally {
      setIsTyping(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div style={{
      height: '100%',
      display: 'flex',
      flexDirection: 'column',
      backgroundColor: '#1e1e2e',
      color: '#cdd6f4',
      fontFamily: 'Inter, system-ui, sans-serif'
    }}>
      {/* Header with connection status */}
      <div style={{
        padding: '16px',
        borderBottom: '1px solid #45475a',
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center'
      }}>
        <h2 style={{ margin: 0, fontSize: '18px', fontWeight: 600 }}>
          Career Concierge
        </h2>
        <div style={{
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
          fontSize: '12px'
        }}>
          <div style={{
            width: '8px',
            height: '8px',
            borderRadius: '50%',
            backgroundColor: isConnected ? '#a6e3a1' : '#f38ba8'
          }} />
          {isConnected ? 'Connected' : 'Connecting...'}
        </div>
      </div>

      {/* Messages area */}
      <div style={{
        flex: 1,
        overflowY: 'auto',
        padding: '16px',
        display: 'flex',
        flexDirection: 'column',
        gap: '12px'
      }}>
        {messages.length === 0 && (
          <div style={{
            textAlign: 'center',
            color: '#6c7086',
            marginTop: '40px'
          }}>
            👋 Start chatting with Career Concierge!
            <br />
            <small>Ask about your career path, skills, or tasks</small>
          </div>
        )}

        {messages.map((msg) => (
          <div key={msg.id} style={{
            display: 'flex',
            justifyContent: msg.role === 'user' ? 'flex-end' : 'flex-start'
          }}>
            <div style={{
              maxWidth: '80%',
              padding: '12px',
              borderRadius: '8px',
              backgroundColor: msg.role === 'user' ? '#89b4fa' : '#313244',
              color: msg.role === 'user' ? '#1e1e2e' : '#cdd6f4',
              boxShadow: '0 2px 4px rgba(0,0,0,0.1)'
            }}>
              {msg.content}
            </div>
          </div>
        ))}

        {isTyping && (
          <div style={{ display: 'flex', justifyContent: 'flex-start' }}>
            <div style={{
              padding: '12px',
              borderRadius: '8px',
              backgroundColor: '#313244',
              display: 'flex',
              gap: '4px'
            }}>
              <span style={{ animation: 'bounce 1s infinite' }}>●</span>
              <span style={{ animation: 'bounce 1s infinite 0.2s' }}>●</span>
              <span style={{ animation: 'bounce 1s infinite 0.4s' }}>●</span>
            </div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input area */}
      <div style={{
        padding: '16px',
        borderTop: '1px solid #45475a',
        display: 'flex',
        gap: '8px'
      }}>
        <textarea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Type your message... (Ctrl+Enter to send)"
          style={{
            flex: 1,
            padding: '12px',
            borderRadius: '8px',
            border: '1px solid #45475a',
            backgroundColor: '#181825',
            color: '#cdd6f4',
            resize: 'none',
            minHeight: '44px',
            maxHeight: '120px',
            fontFamily: 'inherit',
            fontSize: '14px'
          }}
        />
        <button
          onClick={handleSend}
          disabled={!input.trim() || !isConnected}
          style={{
            padding: '0 24px',
            borderRadius: '8px',
            border: 'none',
            backgroundColor: '#89b4fa',
            color: '#1e1e2e',
            fontWeight: 600,
            cursor: input.trim() && isConnected ? 'pointer' : 'not-allowed',
            opacity: input.trim() && isConnected ? 1 : 0.5
          }}
        >
          Send
        </button>
      </div>

      <style>{`
        @keyframes bounce {
          0%, 100% { transform: translateY(0); }
          50% { transform: translateY(-4px); }
        }
      `}</style>
    </div>
  );
}

// Register as widget
createReactWidget({
  id: 'simple-chat',
  title: '💬 Career Concierge Chat',
  span: 'col-span-2 row-span-2',
  component: SimpleChatWidget
});
