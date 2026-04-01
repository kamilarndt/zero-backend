# ZeroClaw Dashboard - Widget Development Guide

## Overview

The ZeroClaw Dashboard uses a **push-based widget architecture** with Server-Sent Events (SSE) for real-time updates. This guide explains the widget system and provides best practices for creating new widgets.

## Widget Architecture

### Core Components

1. **WidgetModule Interface** - Contract all widgets must implement
2. **WidgetRegistry** - Centralized widget registration system
3. **SSEBus** - Push-based event delivery with RAF batching
4. **Panel** - CSS Grid layout manager with drag & drop, resize handles

### Data Flow

```
Backend Gateway → SSE Stream → SSEBus → Widget Events → Widget Render
```

## Widget Module Pattern

Every widget must implement the `WidgetModule` interface:

```typescript
interface WidgetModule {
  id: string;              // Unique widget identifier
  title: string;           // Display name
  span?: string;           // CSS grid span (e.g., 'col-span-2 row-span-2')
  init(container: HTMLElement, push: EventPush): Cleanup;
}
```

### Template Widget Structure

```typescript
const MyWidget: WidgetModule = {
  id: 'my-widget',
  title: 'My Widget',

  init(container: HTMLElement, push: EventPush): Cleanup {
    // 1. Initialize state
    const state = { /* your state here */ };

    // 2. Define render function
    function render() {
      container.innerHTML = '';
      // Create and append DOM elements
    }

    // 3. Define SSE event handler
    function handleSSEEvent(event: SSEEvent) {
      // Update state based on event
      render();
    }

    // 4. Subscribe to SSE events
    const unsubscribe = push(handleSSEEvent);

    // 5. Fetch initial data (optional)
    fetchData();

    // 6. Return cleanup function
    return () => {
      unsubscribe();
      container.innerHTML = '';
      // Additional cleanup (timers, abort controllers, etc.)
    };
  }
};
```

## Best Practices

### 1. TypeScript Strict Typing

Always define interfaces for your data structures:

```typescript
interface MyWidgetState {
  data: DataType[];
  isLoading: boolean;
  error: string | null;
}

interface ApiResponse {
  success: boolean;
  data: {
    items: DataType[];
    timestamp: string;
  };
}
```

### 2. Error Handling

Implement proper error handling with visual feedback:

```typescript
async function fetchData(): Promise<void> {
  try {
    state.isLoading = true;
    render();

    const response = await fetch('/api/v1/endpoint', {
      signal: abortController.signal,
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const data: ApiResponse = await response.json();
    state.data = data.data.items;
    state.error = null;
  } catch (error) {
    state.error = (error as Error).message;
    console.error(`[${widgetId}] Fetch error:`, error);
  } finally {
    state.isLoading = false;
    render();
  }
}
```

### 3. Cleanup and Memory Management

Always clean up resources to prevent memory leaks:

```typescript
function cleanup(): void {
  abortController.abort();           // Cancel pending requests
  if (refreshTimer) {
    clearInterval(refreshTimer);      // Clear timers
  }
  unsubscribe();                      // Unsubscribe from SSE
  container.innerHTML = '';           // Clear DOM
}
```

### 4. Performance Optimization

- **Use RAF Batching**: SSEBus already batches events per RAF frame
- **Debounce Updates**: For high-frequency events, implement debouncing
- **Virtual Scrolling**: For large lists, implement virtual scrolling
- **Lazy Loading**: Load data on-demand when possible

### 5. Responsive Design

Use CSS variables for consistent styling:

```typescript
element.style.cssText = `
  padding: var(--spacing-md);
  background: var(--background-secondary);
  color: var(--text);
  font-size: 13px;
`;
```

### 6. Loading and Empty States

Always provide feedback for all states:

```typescript
function render(): void {
  if (state.isLoading) {
    container.appendChild(createLoadingElement());
  } else if (state.error) {
    container.appendChild(createErrorElement(state.error));
  } else if (state.data.length === 0) {
    container.appendChild(createEmptyElement());
  } else {
    // Render actual data
  }
}
```

## SSE Integration

### SSE Event Types

The backend sends different event types:

```typescript
interface SSEEvent {
  from: string;        // Widget ID or '__bus' for system events
  type: string;        // Event type ('connection_update', 'error', etc.)
  data: unknown;       // Event payload
  timestamp?: string;  // ISO timestamp
}
```

### Subscribing to Events

```typescript
// Subscribe to all events for this widget
push((event: SSEEvent) => {
  if (event.type === 'my_event_type') {
    handleMyEvent(event.data);
  }
});

// Subscribe to specific event type
push((event: SSEEvent) => {
  switch (event.type) {
    case 'connection_update':
      handleConnectionUpdate(event.data);
      break;
    case 'error':
      handleError(event.data);
      break;
    default:
      console.debug('Unhandled event:', event.type);
  }
});
```

## API Integration

### API Endpoints

The ZeroClaw Gateway provides these endpoints:

- `GET /api/v1/status` - System status
- `GET /api/v1/connections` - Connection states (NEW)
- `GET /api/v1/config` - Configuration
- `GET /api/events` - SSE stream

### Making API Calls

```typescript
async function fetchFromAPI(endpoint: string): Promise<DataType> {
  const response = await fetch(endpoint, {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json',
      // Add authentication if needed
      // 'Authorization': `Bearer ${token}`,
    },
    signal: abortController.signal,
  });

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
  }

  return response.json();
}
```

## Styling Guidelines

### CSS Variables

Use these CSS variables for consistent styling:

- `--spacing-xs`, `--spacing-sm`, `--spacing-md`, `--spacing-lg`, `--spacing-xl`
- `--text`, `--text-secondary`, `--overlay`
- `--background`, `--background-secondary`
- `--border`, `--primary`, `--success`, `--warning`, `--error`

### Widget Structure

```html
<div class="widget">
  <div class="widget-header">
    <span class="widget-title">Widget Title</span>
    <span class="widget-status connected/disconnected"></span>
  </div>
  <div class="widget-body">
    <!-- Widget content -->
  </div>
</div>
```

## Widget Registration

### Register Your Widget

```typescript
import { registerWidget } from '../../widgetRegistry';
import type { WidgetModule, EventPush, Cleanup, SSEEvent } from '../../types';

const MyWidget: WidgetModule = {
  // ... implementation
};

registerWidget(MyWidget);
export default MyWidget;
```

### Import in App.tsx

```typescript
import './components/widgets/MyWidget';
```

## Testing

### Manual Testing

1. Start the ZeroClaw gateway: `zeroclaw gateway --port 42617`
2. Open the dashboard: `http://localhost:42617/`
3. Verify your widget appears and functions correctly
4. Test error scenarios (network failures, API errors)
5. Test SSE event handling
6. Verify cleanup works (widget removal, page navigation)

### Debugging

```typescript
// Enable SSE debug logging
console.debug(`[${widgetId}] SSE Event:`, event);

// Monitor render performance
const renderStart = performance.now();
render();
console.debug(`[${widgetId}] Render took ${performance.now() - renderStart}ms`);
```

## Common Patterns

### Periodic Data Refresh

```typescript
let refreshTimer: ReturnType<typeof setTimeout> | null = null;

function setupRefreshTimer(intervalMs: number): void {
  refreshTimer = setInterval(() => {
    fetchData();
  }, intervalMs);
}

// Cleanup
function cleanup(): void {
  if (refreshTimer) {
    clearInterval(refreshTimer);
  }
}
```

### Data Transformation

```typescript
function transformAPIResponse(apiData: APIResponseType): DisplayDataType {
  return {
    id: apiData.id,
    name: apiData.display_name,
    status: apiData.is_active ? 'active' : 'inactive',
  };
}
```

### Event Filtering

```typescript
function handleSSEEvent(event: SSEEvent): void {
  // Filter relevant events
  if (event.from !== widgetId && event.from !== '__bus') {
    return;
  }

  // Handle event
  switch (event.type) {
    case 'relevant_event':
      // Handle event
      break;
  }
}
```

## Examples

See the TemplateWidget (`/web/src/components/widgets/TemplateWidget.tsx`) for a complete implementation example.

## Migration from Old Widgets

When migrating existing widgets to the new pattern:

1. Extract state management into a typed interface
2. Implement proper error handling
3. Add loading and empty states
4. Ensure proper cleanup
5. Update TypeScript types
6. Add SSE integration (if applicable)
7. Test thoroughly

## Troubleshooting

### Widget Not Appearing

- Check if widget is imported in `App.tsx`
- Verify widget is registered with `registerWidget()`
- Check browser console for errors

### SSE Events Not Received

- Verify SSE endpoint is accessible: `GET /api/events`
- Check authentication (if required)
- Verify widget is subscribed to events via `push()`
- Check browser network tab for SSE connection

### Memory Leaks

- Ensure cleanup function removes all event listeners
- Cancel pending fetch requests with `AbortController`
- Clear all timers and intervals
- Remove DOM elements in cleanup

## Future Enhancements

Planned improvements to the widget system:

1. **Widget Communication** - Inter-widget messaging
2. **Widget Settings** - Per-widget configuration UI
3. **Widget Marketplace** - Share and discover widgets
4. **Widget Templates** - Quick-start widget scaffolding
5. **Performance Monitoring** - Built-in performance metrics

## Support

For questions or issues:
1. Check this guide first
2. Review TemplateWidget implementation
3. Check browser console for errors
4. Review Gateway logs for backend issues
5. Open an issue on GitHub

---

**Version:** 1.0.0
**Last Updated:** 2026-04-01
