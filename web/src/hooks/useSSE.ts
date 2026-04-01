import { useEffect, useRef, useState, useCallback } from 'react';
import { SSEEvent } from '../types';

const DEFAULT_ENDPOINT = '/v1/events/stream';

interface UseSSEOptions {
  endpoint?: string;
  autoReconnect?: boolean;
  reconnectInterval?: number;
}

interface UseSSEReturn {
  events: SSEEvent[];
  latestEvent: SSEEvent | null;
  isConnected: boolean;
  clearEvents: () => void;
}

export function useSSE(options: UseSSEOptions = {}): UseSSEReturn {
  const {
    endpoint = DEFAULT_ENDPOINT,
    autoReconnect = true,
    reconnectInterval = 3000,
  } = options;

  const [events, setEvents] = useState<SSEEvent[]>([]);
  const [latestEvent, setLatestEvent] = useState<SSEEvent | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const eventSourceRef = useRef<EventSource | null>(null);
  const reconnectTimerRef = useRef<number | null>(null);

  const clearEvents = useCallback(() => {
    setEvents([]);
    setLatestEvent(null);
  }, []);

  useEffect(() => {
    if (typeof EventSource === 'undefined') {
      console.warn('[useSSE] EventSource not supported')
      return
    }

    function connect() {
      const es = new EventSource(endpoint);
      eventSourceRef.current = es;

      es.onopen = () => setIsConnected(true);

      es.onmessage = (e) => {
        try {
          const parsed: SSEEvent = JSON.parse(e.data);
          setLatestEvent(parsed);
          setEvents((prev) => [...prev.slice(-199), parsed]); // keep last 200
        } catch {
          // ignore malformed
        }
      };

      es.onerror = () => {
        setIsConnected(false);
        es.close();
        if (autoReconnect) {
          reconnectTimerRef.current = window.setTimeout(connect, reconnectInterval);
        }
      };
    }

    connect();

    return () => {
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
      eventSourceRef.current?.close();
    };
  }, [endpoint, autoReconnect, reconnectInterval]);

  return { events, latestEvent, isConnected, clearEvents };
}

// Shared hook for widgets — filters events by widgetId
export function useWidgetEvents(widgetId: string, options?: UseSSEOptions) {
  const { events, latestEvent, isConnected, clearEvents } = useSSE(options);

  const filteredEvents = events.filter((e) => e.from === widgetId);
  const filteredLatest = latestEvent?.from === widgetId ? latestEvent : null;

  return {
    events: filteredEvents,
    latestEvent: filteredLatest,
    isConnected,
    clearEvents,
  };
}
