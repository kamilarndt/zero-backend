/**
 * App entry — mounts Panel with CSS Grid layout or Career Dashboard
 * Simple routing based on URL hash for now
 * No react-router-dom dependency to keep it lightweight
 */

import { useEffect, useRef, useState } from 'react';
import { Panel } from './components/Panel';
import { CareerDashboard } from './components/CareerDashboard';
import './styles/main.css';
import './styles/dashboard.css';

// Import widget modules (side-effect: registers them)
// TEMPORARY: Only Template Widget enabled during refactor
import './components/widgets/TemplateWidget';

// DISABLED WIDGETS - Will be re-enabled after template widget is complete
// import './components/widgets/SystemStatusWidget';
// import './components/widgets/AgentLogWidget';
// import './components/widgets/CommandCenterWidget';
// import './components/widgets/ZeroClawManagerWidget';
// import './components/widgets/AgentChatWidget';
// import './components/widgets/LiveActivityWidget';
// import './components/widgets/KnowledgeWidget';
// import './components/widgets/GenericDataWidgets';
// import './components/widgets/SimpleChatWidget';

type Route = 'main' | 'career';

const App: React.FC = () => {
  const [route, setRoute] = useState<Route>(() => {
    const hash = window.location.hash.slice(1);
    return hash === 'career' ? 'career' : 'main';
  });

  const rootRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<Panel | null>(null);

  // Handle hash changes for routing
  useEffect(() => {
    const handleHashChange = () => {
      const hash = window.location.hash.slice(1);
      setRoute(hash === 'career' ? 'career' : 'main');
    };

    window.addEventListener('hashchange', handleHashChange);
    return () => window.removeEventListener('hashchange', handleHashChange);
  }, []);

  // Initialize main dashboard panel
  useEffect(() => {
    if (route !== 'main' || !rootRef.current) return;

    const panel = new Panel(rootRef.current, {
      widgets: ['template-widget'],
    });

    panelRef.current = panel;

    return () => panel.destroy();
  }, [route]);

  if (route === 'career') {
    return <CareerDashboard />;
  }

  return (
    <div className="dashboard">
      <header style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        padding: 'var(--spacing-lg)',
        borderBottom: '1px solid var(--border)',
        marginBottom: 'var(--spacing-md)',
      }}>
        <h1 className="dashboard-title" style={{ margin: 0 }}>ZeroClaw Dashboard</h1>
        <nav style={{ display: 'flex', gap: 'var(--spacing-sm)' }}>
          <a
            href="#"
            className="btn btn-secondary btn-small"
            style={{
              textDecoration: 'none',
              background: 'transparent',
              color: 'var(--text)',
            }}
          >
            Main Dashboard
          </a>
          <a
            href="#career"
            className="btn btn-primary btn-small"
            style={{
              textDecoration: 'none',
            }}
          >
            Career Dashboard
          </a>
        </nav>
      </header>
      <div ref={rootRef} />
    </div>
  );
};

export default App;
