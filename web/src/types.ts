// Core types for the widget system — no framework deps

export interface SSEEvent {
  from: string;        // widgetId
  type: string;        // event type
  data: unknown;       // payload
  timestamp?: string;
}

// ── Push-based widget module interface ─────────────────────
// Each widget exports a WidgetModule — the registry calls init()
// once, and the widget pushes updates to its container element.

export interface WidgetModule {
  id: string;
  title: string;
  /** Optional CSS class for grid spanning (e.g. 'col-span-2 row-span-2') */
  span?: string;
  /** Called once when the widget is mounted into the DOM */
  init(container: HTMLElement, push: EventPush): Cleanup;
}

export type EventPush = (handler: (event: SSEEvent) => void) => void;
export type Cleanup = () => void;

// ── Panel layout config (JSON-serializable) ────────────────
export interface PanelLayoutConfig {
  columns?: number;       // default: auto-fill
  minCell?: string;       // default: 280px
  gap?: string;           // default: 12px
  widgets?: string[];     // ordered widget IDs to show (empty = all)
}

// ── Career Dashboard types ────────────────────────────────

export interface ChatMessage {
  id: string;
  from: 'user' | 'agent';
  agent?: 'career-concierge' | 'graphic-designer' | string;
  content: string;
  timestamp: Date;
  status: 'sending' | 'sent' | 'delivered' | 'error';
}

export interface CareerTask {
  id: string;
  text: string;
  completed: boolean;
  priority: 'high' | 'medium' | 'low';
}

export interface CareerConciergeData {
  tasks: CareerTask[];
  interviewProgress: {
    current: number;
    total: number;
  };
  stats: {
    tasksCompleted: number;
    skillsIdentified: number;
    patternsDetected: number;
  };
  currentAgent: string;
}

export interface ColorSwatch {
  hex: string;
  name: string;
}

export interface DesignDNA {
  colors: ColorSwatch[];
  typography: {
    heading: string;
    body: string;
  };
  spacing: {
    base: number;
    scale: number;
  };
}

export interface CVStatus {
  format: 'pdf' | 'word' | 'latex';
  ready: boolean;
  url?: string;
}

export interface GraphicDesignerData {
  designDNA: DesignDNA | null;
  cvs: CVStatus[];
  portfolioPreview: string | null;
  isGenerating: boolean;
}
