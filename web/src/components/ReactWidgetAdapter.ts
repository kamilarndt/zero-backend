/**
 * ReactWidgetAdapter — Bridges React components to the vanilla WidgetModule registry.
 *
 * The widget registry expects init(container, push) → cleanup.
 * React widgets use hooks (useWidgetData) which handle SSE internally,
 * so we just need to createRoot → render → unmount.
 *
 * Usage:
 *   import { createReactWidget } from '../ReactWidgetAdapter';
 *   createReactWidget({ id: 'my-widget', title: 'My Widget', component: MyComponent });
 */

import React from 'react';
import { createRoot, type Root } from 'react-dom/client';
import type { WidgetModule } from '../types';
import { registerWidget } from '../widgetRegistry';

interface ReactWidgetOptions<P = Record<string, unknown>> {
  id: string;
  title: string;
  span?: string;
  /** React component to render */
  component: React.FC<P>;
  /** Optional static props to pass to the component */
  props?: P;
}

/**
 * Create a WidgetModule from a React component and register it.
 * Returns the module for direct use if needed.
 */
export function createReactWidget<P extends Record<string, unknown> = Record<string, unknown>>(options: ReactWidgetOptions<P>): WidgetModule {
  const { id, title, span, component: Component, props = {} as P } = options;

  const module: WidgetModule = {
    id,
    title,
    span,
    init(container: HTMLElement): () => void {
      // Create a wrapper div for React to mount into
      const wrapper = document.createElement('div');
      wrapper.style.height = '100%';
      wrapper.style.display = 'flex';
      wrapper.style.flexDirection = 'column';
      container.appendChild(wrapper);

      // Create React 18 root
      const root: Root = createRoot(wrapper);

      // Render the component with proper typing
      root.render(React.createElement(Component as React.ComponentType<P>, props));

      // Return cleanup function
      return () => {
        // Defer unmount to avoid race condition during React render
        setTimeout(() => {
          root.unmount();
          wrapper.remove();
        }, 0);
      };
    },
  };

  // Auto-register
  registerWidget(module);
  return module;
}
