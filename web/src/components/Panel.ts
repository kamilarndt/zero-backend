/**
 * Panel — CSS Grid layout manager with resize handles.
 * Replaces DynamicGrid + react-grid-layout with pure CSS Grid.
 *
 * Pattern adapted from: showcase/my-daily-monitor/src/components/Panel.ts
 * - Row resize (drag handle on bottom edge)
 * - Col resize (drag handle on right edge)
 * - LocalStorage persistence for span state
 */

import { mountWidget, getAllWidgets, getWidget } from '../widgetRegistry';
import type { PanelLayoutConfig, Cleanup, WidgetModule } from '../types';

// ── Persistence constants ───────────────────────────────────
const PANEL_ROW_SPANS_KEY = 'zc-panel-row-spans';
const PANEL_COL_SPANS_KEY = 'zc-panel-col-spans';
const PANEL_WIDGET_ORDER_KEY = 'zc-panel-widget-order';
const ROW_RESIZE_STEP_PX = 80;
const COL_RESIZE_STEP_PX = 80;

// ── LocalStorage helpers ────────────────────────────────────

function loadMap(key: string): Record<string, number> {
  try {
    const s = localStorage.getItem(key);
    return s ? JSON.parse(s) : {};
  } catch {
    return {};
  }
}

function saveMap(key: string, id: string, val: number): void {
  const m = loadMap(key);
  m[id] = val;
  localStorage.setItem(key, JSON.stringify(m));
}

function deleteMapKey(key: string, id: string): void {
  const m = loadMap(key);
  delete m[id];
  Object.keys(m).length
    ? localStorage.setItem(key, JSON.stringify(m))
    : localStorage.removeItem(key);
}

function loadWidgetOrder(): string[] {
  try {
    const s = localStorage.getItem(PANEL_WIDGET_ORDER_KEY);
    return s ? JSON.parse(s) : [];
  } catch {
    return [];
  }
}

function saveWidgetOrder(order: string[]): void {
  localStorage.setItem(PANEL_WIDGET_ORDER_KEY, JSON.stringify(order));
}

// ── Span utilities ──────────────────────────────────────────

function getRowSpan(el: HTMLElement): number {
  if (el.classList.contains('row-span-4')) return 4;
  if (el.classList.contains('row-span-3')) return 3;
  if (el.classList.contains('row-span-2')) return 2;
  return 1;
}

function setRowSpanClass(el: HTMLElement, span: number): void {
  el.classList.remove('row-span-1', 'row-span-2', 'row-span-3', 'row-span-4');
  if (span > 1) {
    el.classList.add(`row-span-${span}`);
  }
  el.style.gridRow = span > 1 ? `span ${span}` : '';
}

function deltaToRowSpan(start: number, dy: number): number {
  const d =
    dy > 0
      ? Math.floor(dy / ROW_RESIZE_STEP_PX)
      : Math.ceil(dy / ROW_RESIZE_STEP_PX);
  return Math.max(1, Math.min(4, start + d));
}

function getColSpan(el: HTMLElement): number {
  if (el.classList.contains('col-span-4')) return 4;
  if (el.classList.contains('col-span-3')) return 3;
  if (el.classList.contains('col-span-2')) return 2;
  return 1;
}

function setColSpanClass(el: HTMLElement, span: number): void {
  el.classList.remove('col-span-1', 'col-span-2', 'col-span-3', 'col-span-4');
  if (span > 1) {
    el.classList.add(`col-span-${span}`);
  }
  el.style.gridColumn = span > 1 ? `span ${span}` : '';
}

function getGridColumnCount(el: HTMLElement): number {
  const grid = el.closest('.panel-grid') as HTMLElement | null;
  if (!grid) return 3;
  const style = window.getComputedStyle(grid);
  const tpl = style.gridTemplateColumns;
  if (!tpl || tpl === 'none') return 3;
  // Count actual rendered columns (works for both fixed and auto-fill)
  const cols = tpl.trim().split(/\s+/).filter(Boolean);
  return cols.length > 0 ? cols.length : 3;
}

function getMaxColSpan(el: HTMLElement): number {
  return Math.max(1, getGridColumnCount(el));
}

function clampColSpan(span: number, max: number): number {
  return Math.max(1, Math.min(max, span));
}

function deltaToColSpan(start: number, dx: number, max = 4): number {
  const d =
    dx > 0
      ? Math.floor(dx / COL_RESIZE_STEP_PX)
      : Math.ceil(dx / COL_RESIZE_STEP_PX);
  return clampColSpan(start + d, max);
}

// ── Widget resize state ─────────────────────────────────────

interface WidgetResizeState {
  // Row resize
  resizeHandle: HTMLElement | null;
  isResizing: boolean;
  startY: number;
  startRowSpan: number;
  onRowMouseMove: ((e: MouseEvent) => void) | null;
  onRowMouseUp: (() => void) | null;

  // Col resize
  colResizeHandle: HTMLElement | null;
  isColResizing: boolean;
  startX: number;
  startColSpan: number;
  onColMouseMove: ((e: MouseEvent) => void) | null;
  onColMouseUp: (() => void) | null;

  // Drag & Drop
  isDragging: boolean;
  dragOver: boolean;
}

// ── Swap widgets in DOM ──────────────────────────────────────

function swapWidgets(cell1: HTMLElement, cell2: HTMLElement): void {
  const parent = cell1.parentNode;
  if (!parent || cell1.parentNode !== cell2.parentNode) return;

  const sibling1 = cell1.nextSibling === cell2 ? cell1 : cell1.nextSibling;
  const sibling2 = cell2.nextSibling === cell1 ? cell2 : cell2.nextSibling;

  // Swap in DOM
  parent.insertBefore(cell1, sibling2);
  parent.insertBefore(cell2, sibling1);
}

function createResizeState(): WidgetResizeState {
  return {
    resizeHandle: null,
    isResizing: false,
    startY: 0,
    startRowSpan: 1,
    onRowMouseMove: null,
    onRowMouseUp: null,
    colResizeHandle: null,
    isColResizing: false,
    startX: 0,
    startColSpan: 1,
    onColMouseMove: null,
    onColMouseUp: null,
    isDragging: false,
    dragOver: false,
  };
}

// ── Panel class ─────────────────────────────────────────────

export class Panel {
  private container: HTMLElement;
  private cleanups: Cleanup[] = [];
  private config: PanelLayoutConfig;
  private widgetStates = new Map<string, WidgetResizeState>();

  constructor(root: HTMLElement, config: PanelLayoutConfig = {}) {
    this.config = config;
    this.container = document.createElement('div');
    this.container.className = 'panel-grid';
    root.appendChild(this.container);

    // Apply CSS variable overrides from config
    if (config.columns) {
      this.container.style.setProperty(
        '--grid-columns',
        String(config.columns)
      );
      this.container.style.gridTemplateColumns = `repeat(${config.columns}, 1fr)`;
    }
    if (config.minCell) {
      this.container.style.setProperty('--grid-min-cell', config.minCell);
    }
    if (config.gap) {
      this.container.style.setProperty('--grid-gap', config.gap);
    }

    this.mountWidgets();
  }

  // ── Resize setup ────────────────────────────────────────

  private setupRowResize(
    cell: HTMLElement,
    widgetId: string,
    state: WidgetResizeState
  ): void {
    const handle = document.createElement('div');
    handle.className = 'widget-resize-handle';
    handle.title = 'Drag to resize height';

    // Add handle to widget header instead of entire cell
    const header = cell.querySelector('.widget-header');
    if (header) {
      header.appendChild(handle);
    } else {
      // Fallback if no header found
      cell.appendChild(handle);
    }
    state.resizeHandle = handle;

    state.onRowMouseMove = (e: MouseEvent) => {
      if (!state.isResizing) return;
      const newSpan = deltaToRowSpan(state.startRowSpan, e.clientY - state.startY);
      console.debug(`[Panel] row-resize ${widgetId}: dy=${e.clientY - state.startY} span=${state.startRowSpan}→${newSpan}`);
      setRowSpanClass(cell, newSpan);
    };

    state.onRowMouseUp = () => {
      if (!state.isResizing) return;
      state.isResizing = false;
      cell.classList.remove('resizing');
      document.body.classList.remove('panel-resize-active');
      handle.classList.remove('active');
      document.removeEventListener('mousemove', state.onRowMouseMove!);
      document.removeEventListener('mouseup', state.onRowMouseUp!);
      const finalSpan = getRowSpan(cell);
      console.debug(`[Panel] row-resize ${widgetId}: saved span=${finalSpan}`);
      saveMap(PANEL_ROW_SPANS_KEY, widgetId, finalSpan);
    };

    handle.addEventListener('mousedown', (e: MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      state.isResizing = true;
      state.startY = e.clientY;
      state.startRowSpan = getRowSpan(cell);
      console.debug(`[Panel] row-resize ${widgetId}: start span=${state.startRowSpan} y=${state.startY}`);
      cell.classList.add('resizing');
      document.body.classList.add('panel-resize-active');
      handle.classList.add('active');
      document.addEventListener('mousemove', state.onRowMouseMove!);
      document.addEventListener('mouseup', state.onRowMouseUp!);
    });

    handle.addEventListener('dblclick', () => {
      cell.classList.remove('row-span-1', 'row-span-2', 'row-span-3', 'row-span-4');
      cell.style.gridRow = '';
      deleteMapKey(PANEL_ROW_SPANS_KEY, widgetId);
    });
  }

  private setupColResize(
    cell: HTMLElement,
    widgetId: string,
    state: WidgetResizeState
  ): void {
    const handle = document.createElement('div');
    handle.className = 'widget-col-resize-handle';
    handle.title = 'Drag to resize width';

    // Add handle to widget header instead of entire cell
    const header = cell.querySelector('.widget-header');
    if (header) {
      header.appendChild(handle);
    } else {
      // Fallback if no header found
      cell.appendChild(handle);
    }
    state.colResizeHandle = handle;

    state.onColMouseMove = (e: MouseEvent) => {
      if (!state.isColResizing) return;
      const max = getMaxColSpan(cell);
      setColSpanClass(
        cell,
        deltaToColSpan(state.startColSpan, e.clientX - state.startX, max)
      );
    };

    state.onColMouseUp = () => {
      if (!state.isColResizing) return;
      state.isColResizing = false;
      cell.classList.remove('col-resizing');
      document.body.classList.remove('panel-resize-active');
      handle.classList.remove('active');
      document.removeEventListener('mousemove', state.onColMouseMove!);
      document.removeEventListener('mouseup', state.onColMouseUp!);
      const final = clampColSpan(getColSpan(cell), getMaxColSpan(cell));
      if (final !== state.startColSpan) {
        saveMap(PANEL_COL_SPANS_KEY, widgetId, final);
      }
    };

    handle.addEventListener('mousedown', (e: MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      state.isColResizing = true;
      state.startX = e.clientX;
      state.startColSpan = clampColSpan(
        getColSpan(cell),
        getMaxColSpan(cell)
      );
      cell.classList.add('col-resizing');
      document.body.classList.add('panel-resize-active');
      handle.classList.add('active');
      document.addEventListener('mousemove', state.onColMouseMove!);
      document.addEventListener('mouseup', state.onColMouseUp!);
    });

    handle.addEventListener('dblclick', () => {
      setColSpanClass(cell, 1);
      deleteMapKey(PANEL_COL_SPANS_KEY, widgetId);
    });
  }

  private cleanupResizeState(state: WidgetResizeState): void {
    if (state.onRowMouseMove)
      document.removeEventListener('mousemove', state.onRowMouseMove);
    if (state.onRowMouseUp)
      document.removeEventListener('mouseup', state.onRowMouseUp);
    if (state.onColMouseMove)
      document.removeEventListener('mousemove', state.onColMouseMove);
    if (state.onColMouseUp)
      document.removeEventListener('mouseup', state.onColMouseUp);
  }

  // ── Drag & Drop setup ─────────────────────────────────────

  private setupDragAndDrop(
    cell: HTMLElement,
    widgetId: string,
    state: WidgetResizeState
  ): void {
    cell.classList.add('widget-draggable');

    // Only allow dragging from the header, not the body
    const header = cell.querySelector('.widget-header') as HTMLElement;
    if (header) {
      header.style.cursor = 'grab';
      header.setAttribute('draggable', 'true');

      header.addEventListener('dragstart', (e: DragEvent) => {
        if (!(e.dataTransfer)) return;
        state.isDragging = true;
        cell.classList.add('widget-dragging');
        e.dataTransfer.effectAllowed = 'move';
        e.dataTransfer.setData('text/plain', widgetId);
        header.style.cursor = 'grabbing';
      });

      header.addEventListener('dragend', () => {
        state.isDragging = false;
        cell.classList.remove('widget-dragging');
        header.style.cursor = 'grab';
        // Update saved order
        this.saveCurrentWidgetOrder();
      });
    }

    cell.addEventListener('dragover', (e: DragEvent) => {
      e.preventDefault();
      if (!(e.dataTransfer)) return;
      e.dataTransfer.dropEffect = 'move';
      if (!state.dragOver) {
        state.dragOver = true;
        cell.classList.add('widget-drag-over');
      }
    });

    cell.addEventListener('dragleave', () => {
      state.dragOver = false;
      cell.classList.remove('widget-drag-over');
    });

    cell.addEventListener('drop', (e: DragEvent) => {
      e.preventDefault();
      state.dragOver = false;
      cell.classList.remove('widget-drag-over');

      if (!(e.dataTransfer)) return;
      const draggedId = e.dataTransfer.getData('text/plain');
      if (!draggedId || draggedId === widgetId) return;

      // Find the dragged element
      const allWidgets = Array.from(this.container.children);
      const draggedEl = allWidgets.find((el) => {
        const widgetBody = el.querySelector('.widget-body');
        return widgetBody?.getAttribute('data-widget-id') === draggedId;
      });

      if (!draggedEl || draggedEl === cell) return;

      // Swap widgets
      swapWidgets(draggedEl as HTMLElement, cell);
    });
  }

  private saveCurrentWidgetOrder(): void {
    const currentOrder = Array.from(this.container.children).map((el) => {
      const widgetBody = el.querySelector('.widget-body');
      return widgetBody?.getAttribute('data-widget-id') || '';
    }).filter(Boolean);
    saveWidgetOrder(currentOrder);
  }

  // ── Widget mounting ─────────────────────────────────────

  private mountWidgets(): void {
    const all = getAllWidgets();
    const savedOrder = loadWidgetOrder();

    // Use saved order if available, otherwise use config order or all widgets
    let ordered: typeof all;
    if (savedOrder.length > 0) {
      const orderMap = new Map(savedOrder.map((id, idx) => [id, idx]));
      ordered = all
        .map((mod) => ({ mod, orderIdx: orderMap.get(mod.id) ?? 999 }))
        .sort((a, b) => a.orderIdx - b.orderIdx)
        .map(({ mod }) => mod);
    } else if (this.config.widgets?.length) {
      ordered = this.config.widgets
        .map((id) => getWidget(id))
        .filter((mod): mod is WidgetModule => mod !== undefined);
    } else {
      ordered = all;
    }

    const savedRows = loadMap(PANEL_ROW_SPANS_KEY);
    const savedCols = loadMap(PANEL_COL_SPANS_KEY);

    for (const mod of ordered) {
      if (!mod) continue;

      const cell = document.createElement('div');
      cell.className = 'widget';

      // Apply saved spans first, then module defaults
      const savedRow = savedRows[mod.id];
      if (savedRow && savedRow > 1) {
        setRowSpanClass(cell, savedRow);
      }
      const savedCol = savedCols[mod.id];
      if (typeof savedCol === 'number' && savedCol > 1) {
        setColSpanClass(cell, savedCol);
      }

      // Module default span (only if nothing saved)
      if (!savedRow && !savedCol && mod.span) {
        cell.classList.add(...mod.span.split(' '));
      }

      this.container.appendChild(cell);
      const cleanup = mountWidget(cell, mod.id);
      this.cleanups.push(cleanup);

      // Add resize handles and drag & drop AFTER widget body
      const state = createResizeState();
      this.widgetStates.set(mod.id, state);
      this.setupRowResize(cell, mod.id, state);
      this.setupColResize(cell, mod.id, state);
      this.setupDragAndDrop(cell, mod.id, state);
    }
  }

  // ── Public API ──────────────────────────────────────────

  /** Reconfigure layout at runtime */
  reconfigure(config: PanelLayoutConfig): void {
    this.destroy();
    this.config = config;
    this.mountWidgets();
  }

  /** Clear all saved span state and re-render */
  resetSpans(): void {
    localStorage.removeItem(PANEL_ROW_SPANS_KEY);
    localStorage.removeItem(PANEL_COL_SPANS_KEY);
    this.reconfigure(this.config);
  }

  /** Tear down all widgets */
  destroy(): void {
    for (const [, state] of this.widgetStates) {
      this.cleanupResizeState(state);
    }
    this.widgetStates.clear();
    document.body.classList.remove('panel-resize-active');

    for (const fn of this.cleanups) fn();
    this.cleanups = [];
    this.container.innerHTML = '';
  }
}
