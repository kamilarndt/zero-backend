/**
 * CareerDashboard - Main dashboard page for Career Management system
 * Layout: Header + Widget Grid + Sidepanel (chat)
 */

import React, { useState } from 'react';
import { Sidepanel } from './Sidepanel';
import { CareerConciergeWidget } from './widgets/CareerConciergeWidget';
import { GraphicDesignerWidget } from './widgets/GraphicDesignerWidget';
import { useConnectionStatus } from '../hooks/useChatMessages';
import '../styles/dashboard.css';

export const CareerDashboard: React.FC = () => {
  const [sidepanelOpen, setSidepanelOpen] = useState(true);
  const [activeAgent, setActiveAgent] = useState<'career-concierge' | 'graphic-designer'>('career-concierge');
  const { isConnected } = useConnectionStatus();

  const toggleSidepanel = () => {
    setSidepanelOpen(!sidepanelOpen);
  };

  const handleAgentSelect = (agent: 'career-concierge' | 'graphic-designer') => {
    setActiveAgent(agent);
    if (!sidepanelOpen) {
      setSidepanelOpen(true);
    }
  };

  const getAgentInfo = (agent: 'career-concierge' | 'graphic-designer') => {
    switch (agent) {
      case 'career-concierge':
        return { title: 'Career Concierge', badge: 'CC' };
      case 'graphic-designer':
        return { title: 'Graphic Designer', badge: 'GD' };
      default:
        return { title: 'Career Concierge', badge: 'CC' };
    }
  };

  const agentInfo = getAgentInfo(activeAgent);

  return (
    <div className="career-dashboard">
      {/* Main Content Area */}
      <div className="dashboard-main">
        {/* Header */}
        <header className="dashboard-header">
          <div className="dashboard-header-left">
            <div className="dashboard-logo">
              <div className="dashboard-logo-icon">ZC</div>
              <h1 className="dashboard-title">Career Dashboard</h1>
            </div>
          </div>

          <div className="dashboard-header-right">
            {/* Connection Status */}
            <div className="connection-status">
              <div className={`connection-status-dot${isConnected ? ' connected' : ''}`} />
              <span>{isConnected ? 'SSE Connected' : 'SSE Disconnected'}</span>
            </div>

            {/* User Profile */}
            <div className="user-profile" title="User profile">
              <div className="user-avatar">JD</div>
              <span className="user-name">John Doe</span>
            </div>

            {/* Sidepanel Toggle */}
            <button
              className="sidepanel-toggle"
              onClick={toggleSidepanel}
              aria-label={sidepanelOpen ? 'Hide sidepanel' : 'Show sidepanel'}
              title={sidepanelOpen ? 'Hide sidepanel' : 'Show sidepanel'}
            >
              {sidepanelOpen ? (
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <polyline points="9 18 15 12 9 6" />
                </svg>
              ) : (
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <polyline points="15 18 9 12 15 6" />
                </svg>
              )}
            </button>
          </div>
        </header>

        {/* Widget Grid */}
        <main className="dashboard-content">
          <div className="career-widget-grid">
            <CareerConciergeWidget
              onChat={(agent) => handleAgentSelect(agent)}
              isActive={activeAgent === 'career-concierge'}
            />
            <GraphicDesignerWidget
              onChat={(agent) => handleAgentSelect(agent)}
              isActive={activeAgent === 'graphic-designer'}
            />
          </div>
        </main>
      </div>

      {/* Sidepanel */}
      <Sidepanel
        isOpen={sidepanelOpen}
        onClose={() => setSidepanelOpen(false)}
        widgetId={activeAgent}
        title={agentInfo.title}
        agentName={agentInfo.badge}
      />
    </div>
  );
};

export default CareerDashboard;
