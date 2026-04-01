/**
 * BaseWidget — React wrapper for push-based SSE widgets.
 * Uses shared sseBus singleton with RAF batching — single EventSource
 * shared across all widgets, events flushed once per animation frame.
 */

import React, { useEffect, useState, useCallback } from 'react';
import { SSEEvent } from '../types';
import { sseBus } from '../sseBus';

interface BaseWidgetProps {
  widgetId: string;
  title: string;
  children: (props: {
    events: SSEEvent[];
    latestEvent: SSEEvent | null;
    isConnected: boolean;
    send: (type: string, data: unknown) => void;
  }) => React.ReactNode;
  /** Optional CSS class for grid spanning */
  span?: string;
  /** Max events to keep in buffer */
  maxEvents?: number;
}

const BaseWidget: React.FC<BaseWidgetProps> = ({
  widgetId,
  title,
  children,
  maxEvents = 200,
}) => {
  const [events, setEvents] = useState<SSEEvent[]>([]);
  const [latestEvent, setLatestEvent] = useState<SSEEvent | null>(null);
  const [isConnected, setIsConnected] = useState(sseBus.isConnected);

  useEffect(() => {
    // Use RAF-batched subscription — one setState call per frame
    // instead of one per SSE event
    const unsub = sseBus.subscribeWidgetBatched(widgetId, (batch) => {
      setLatestEvent(batch[batch.length - 1]);
      setEvents((prev) => {
        const merged = [...prev, ...batch];
        return merged.length > maxEvents ? merged.slice(-maxEvents) : merged;
      });
    });

    // Track connection status via global bus events
    const unsubConn = sseBus.subscribe((evt) => {
      if (evt.from === '__bus') {
        setIsConnected(sseBus.isConnected);
      }
    });

    return () => {
      unsub();
      unsubConn();
    };
  }, [widgetId, maxEvents]);

  const send = useCallback(
    (type: string, data: unknown) => {
      fetch('/v1/events/send', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ from: widgetId, type, data }),
      }).catch(() => {});
    },
    [widgetId]
  );

  return (
    <div className="widget-base">
      <div className="widget-header">
        <span className="widget-title">{title}</span>
        <span className={`widget-status ${isConnected ? 'connected' : 'disconnected'}`} />
      </div>
      <div className="widget-body">
        {children({ events, latestEvent, isConnected, send })}
      </div>
    </div>
  );
};

export default BaseWidget;
