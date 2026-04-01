/**
 * useChatMessages - Custom hook for managing chat state
 * Handles messages, typing indicators, and SSE integration
 */

import { useState, useCallback, useRef, useEffect } from 'react';
import { sseBus } from '../sseBus';

export interface ChatMessage {
  id: string;
  from: 'user' | 'agent';
  agent?: 'career-concierge' | 'graphic-designer' | string;
  content: string;
  timestamp: Date;
  status: 'sending' | 'sent' | 'delivered' | 'error';
}

interface UseChatMessagesOptions {
  widgetId?: string;
  initialMessages?: ChatMessage[];
}

interface UseChatMessagesReturn {
  messages: ChatMessage[];
  isTyping: boolean;
  addMessage: (content: string, from: 'user' | 'agent', agent?: string) => void;
  updateMessageStatus: (id: string, status: ChatMessage['status']) => void;
  sendMessage: (content: string, endpoint?: string) => Promise<void>;
  clearMessages: () => void;
}

export function useChatMessages(
  options: UseChatMessagesOptions = {}
): UseChatMessagesReturn {
  const { widgetId, initialMessages = [] } = options;

  const [messages, setMessages] = useState<ChatMessage[]>(initialMessages);
  const [isTyping, setIsTyping] = useState(false);
  const pendingMessagesRef = useRef(new Set<string>());

  // Generate unique message ID
  const generateId = useCallback(() => {
    return `msg-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }, []);

  // Add a new message
  const addMessage = useCallback(
    (content: string, from: 'user' | 'agent', agent?: string) => {
      const newMessage: ChatMessage = {
        id: generateId(),
        from,
        agent,
        content,
        timestamp: new Date(),
        status: from === 'user' ? 'sending' : 'delivered',
      };

      setMessages((prev) => [...prev, newMessage]);

      if (from === 'user') {
        pendingMessagesRef.current.add(newMessage.id);
      }

      return newMessage.id;
    },
    [generateId]
  );

  // Update message status
  const updateMessageStatus = useCallback((id: string, status: ChatMessage['status']) => {
    setMessages((prev) =>
      prev.map((msg) => (msg.id === id ? { ...msg, status } : msg))
    );

    if (status === 'sent' || status === 'delivered') {
      pendingMessagesRef.current.delete(id);
    }
  }, []);

  // Send message to backend
  const sendMessage = useCallback(
    async (content: string, endpoint = '/v1/widgets/career-concierge') => {
      const messageId = addMessage(content, 'user');
      setIsTyping(true);

      try {
        const response = await fetch(endpoint, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            data: {
              type: 'chat_message',
              content,
              timestamp: new Date().toISOString(),
            },
          }),
        });

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        updateMessageStatus(messageId, 'sent');
      } catch (error) {
        updateMessageStatus(messageId, 'error');
        console.error('[useChatMessages] Send error:', error);
      }
    },
    [addMessage, updateMessageStatus]
  );

  // Clear all messages
  const clearMessages = useCallback(() => {
    setMessages([]);
    pendingMessagesRef.current.clear();
  }, []);

  // SSE integration - listen for agent responses
  useEffect(() => {
    if (!widgetId) return;

    const unsubscribe = sseBus.subscribeWidget(widgetId, (event) => {
      if (event.type === 'chat_message' || event.type === 'agent_response') {
        const data = event.data as { content?: string; agent?: string };

        if (data.content) {
          setIsTyping(false);
          addMessage(data.content, 'agent', data.agent || widgetId);
        }
      } else if (event.type === 'typing_start') {
        setIsTyping(true);
      } else if (event.type === 'typing_end') {
        setIsTyping(false);
      }
    });

    return unsubscribe;
  }, [widgetId, addMessage]);

  return {
    messages,
    isTyping,
    addMessage,
    updateMessageStatus,
    sendMessage,
    clearMessages,
  };
}

/**
 * useConnectionStatus - Track SSE connection status
 */
export function useConnectionStatus() {
  const [isConnected, setIsConnected] = useState(sseBus.isConnected);
  const [isConnecting, setIsConnecting] = useState(false);

  useEffect(() => {
    let connectTimeout: ReturnType<typeof setTimeout>;

    const unsubscribe = sseBus.subscribe((event) => {
      if (event.from === '__bus') {
        if (event.type === 'connected') {
          setIsConnecting(false);
          setIsConnected(true);
        } else if (event.type === 'disconnected') {
          setIsConnected(false);
          // Try to reconnect
          setIsConnecting(true);
          connectTimeout = setTimeout(() => {
            // Trigger reconnect by accessing any property
            const connected = sseBus.isConnected;
            setIsConnected(connected);
            setIsConnecting(false);
          }, 3000);
        }
      }
    });

    return () => {
      unsubscribe();
      if (connectTimeout) clearTimeout(connectTimeout);
    };
  }, []);

  return { isConnected, isConnecting };
}
