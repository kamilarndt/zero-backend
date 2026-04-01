/**
 * WidgetGrid - Reusable CSS Grid widget layout component
 * Auto-fit responsive grid with collapse/expand functionality
 */

import React, { useState, useCallback } from 'react';
import '../styles/dashboard.css';

export interface Widget {
  id: string;
  title: string;
  component: React.ComponentType<any>;
  props?: Record<string, unknown>;
  collapsed?: boolean;
}

export interface WidgetGridProps {
  widgets: Widget[];
  onWidgetAdd?: () => void;
  onWidgetRemove?: (id: string) => void;
  onWidgetReorder?: (fromIndex: number, toIndex: number) => void;
  minColumnWidth?: string;
  gap?: string;
}

export const WidgetGrid: React.FC<WidgetGridProps> = ({
  widgets,
  onWidgetAdd,
  onWidgetRemove,
  minColumnWidth = '320px',
  gap = 'var(--spacing-md)',
}) => {
  const [collapsedWidgets, setCollapsedWidgets] = useState<Set<string>>(new Set());

  const toggleCollapse = useCallback((widgetId: string) => {
    setCollapsedWidgets((prev) => {
      const next = new Set(prev);
      if (next.has(widgetId)) {
        next.delete(widgetId);
      } else {
        next.add(widgetId);
      }
      return next;
    });
  }, []);

  const handleRemove = useCallback(
    (widgetId: string) => {
      onWidgetRemove?.(widgetId);
    },
    [onWidgetRemove]
  );

  return (
    <div
      className="career-widget-grid"
      style={{
        gridTemplateColumns: `repeat(auto-fill, minmax(${minColumnWidth}, 1fr))`,
        gap,
      }}
    >
      {widgets.map((widget) => {
        const WidgetComponent = widget.component;
        const isCollapsed = collapsedWidgets.has(widget.id);

        return (
          <div key={widget.id} className="widget-container">
            <div className="career-widget-card" style={{ height: isCollapsed ? 'auto' : '100%' }}>
              <div className="career-widget-header">
                <div className="career-widget-title">{widget.title}</div>
                <div style={{ display: 'flex', gap: 'var(--spacing-xs)' }}>
                  <button
                    className="btn btn-ghost btn-small"
                    onClick={() => toggleCollapse(widget.id)}
                    aria-label={isCollapsed ? 'Expand' : 'Collapse'}
                    title={isCollapsed ? 'Expand' : 'Collapse'}
                  >
                    {isCollapsed ? (
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16">
                        <polyline points="6 9 12 15 18 9" />
                      </svg>
                    ) : (
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16">
                        <polyline points="18 15 12 9 6 15" />
                      </svg>
                    )}
                  </button>
                  {onWidgetRemove && (
                    <button
                      className="btn btn-ghost btn-small"
                      onClick={() => handleRemove(widget.id)}
                      aria-label="Remove widget"
                      title="Remove widget"
                    >
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16">
                        <line x1="18" y1="6" x2="6" y2="18" />
                        <line x1="6" y1="6" x2="18" y2="18" />
                      </svg>
                    </button>
                  )}
                </div>
              </div>
              {!isCollapsed && (
                <div className="career-widget-body">
                  <WidgetComponent {...(widget.props || {})} />
                </div>
              )}
            </div>
          </div>
        );
      })}

      {/* Add Widget Button */}
      {onWidgetAdd && (
        <div className="widget-container">
          <button
            className="career-widget-card"
            onClick={onWidgetAdd}
            style={{
              height: '200px',
              borderStyle: 'dashed',
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flexDirection: 'column',
              gap: 'var(--spacing-sm)',
              background: 'transparent',
              color: 'var(--overlay)',
            }}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="32" height="32">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            <span style={{ fontSize: 'var(--font-size-sm)' }}>Add Widget</span>
          </button>
        </div>
      )}
    </div>
  );
};

export default WidgetGrid;
