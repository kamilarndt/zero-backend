// Core types for the widget system

export interface SSEEvent {
  from: string;        // widgetId
  type: string;        // event type
  data: unknown;       // payload
  timestamp?: string;
}

export interface WidgetConfig {
  id: string;
  title: string;
  component: React.ComponentType<WidgetProps>;
  defaultLayout: WidgetLayout;
}

export interface WidgetLayout {
  i: string;
  x: number;
  y: number;
  w: number;
  h: number;
  minW?: number;
  minH?: number;
  maxW?: number;
  maxH?: number;
  static?: boolean;
}

export interface WidgetProps {
  widgetId: string;
  events: SSEEvent[];
  latestEvent: SSEEvent | null;
}
