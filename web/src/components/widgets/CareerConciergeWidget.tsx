/**
 * CareerConciergeWidget - Main hub/coordinator widget
 * Displays tasks, interview progress, and quick stats
 *
 * Registered via ReactWidgetAdapter to bridge React hooks → vanilla registry.
 */

import React from 'react';
import { useCareerConciergeData } from '../../hooks/useWidgetData';
import { createReactWidget } from '../ReactWidgetAdapter';
import '../../styles/dashboard.css';

export interface CareerConciergeWidgetProps {
  onChat?: (agent: 'career-concierge' | 'graphic-designer') => void;
  isActive?: boolean;
}

export const CareerConciergeWidget: React.FC<CareerConciergeWidgetProps> = ({
  onChat,
  isActive = false,
}) => {
  const { data, isLoading, toggleTask, addTask } = useCareerConciergeData();

  const handleContinueInterview = () => {
    onChat?.('career-concierge');
  };

  const getPriorityClass = (priority: string) => {
    switch (priority) {
      case 'high':
        return 'task-priority high';
      case 'medium':
        return 'task-priority medium';
      case 'low':
        return 'task-priority low';
      default:
        return 'task-priority medium';
    }
  };

  const progressPercentage = (data.interviewProgress.current / data.interviewProgress.total) * 100;

  return (
    <div className={`career-widget-card ${isActive ? 'active' : ''}`}>
      <div className="career-widget-header">
        <div className="career-widget-title">
          <div className="career-widget-icon career-concierge">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16">
              <path d="M16 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
              <circle cx="8.5" cy="7" r="4" />
              <line x1="20" y1="8" x2="20" y2="14" />
              <line x1="23" y1="11" x2="17" y2="11" />
            </svg>
          </div>
          <span>Career Concierge</span>
        </div>
        <div className={`connection-status-dot ${!isLoading ? 'connected' : ''}`} />
      </div>

      <div className="career-widget-body">
        {/* Quick Stats */}
        <div className="stats-grid">
          <div className="stat-card">
            <div className="stat-value">{data.stats.tasksCompleted}</div>
            <div className="stat-label">Tasks Today</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{data.stats.skillsIdentified}</div>
            <div className="stat-label">Skills Found</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{data.stats.patternsDetected}</div>
            <div className="stat-label">Patterns</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{data.tasks.filter((t) => t.completed).length}/{data.tasks.length}</div>
            <div className="stat-label">Completed</div>
          </div>
        </div>

        {/* Interview Progress */}
        <div className="interview-progress" style={{ marginBottom: 'var(--spacing-md)' }}>
          <div className="progress-label">
            <span>Interview Progress</span>
            <span>Step {data.interviewProgress.current} of {data.interviewProgress.total}</span>
          </div>
          <div className="progress-bar">
            <div className="progress-fill" style={{ width: `${progressPercentage}%` }} />
          </div>
        </div>

        {/* Task List */}
        <h4 style={{ margin: 'var(--spacing-md) 0 var(--spacing-sm)', fontSize: 'var(--font-size-base)', color: 'var(--text)' }}>
          Current Tasks
        </h4>
        <div className="task-list">
          {data.tasks.length === 0 ? (
            <div className="empty-state" style={{ padding: 'var(--spacing-md)' }}>
              <div className="empty-state-text">No tasks yet. Start the interview!</div>
            </div>
          ) : (
            data.tasks.slice(0, 5).map((task) => (
              <div
                key={task.id}
                className="task-item"
                onClick={() => toggleTask(task.id)}
                style={{ cursor: 'pointer' }}
              >
                <div className={`task-checkbox ${task.completed ? 'checked' : ''}`} />
                <span className={`task-text ${task.completed ? 'completed' : ''}`}>
                  {task.text}
                </span>
                <div className={getPriorityClass(task.priority)} />
              </div>
            ))
          )}
        </div>

        {isLoading && (
          <div style={{ textAlign: 'center', padding: 'var(--spacing-md)', color: 'var(--overlay)' }}>
            Syncing...
          </div>
        )}
      </div>

      <div className="career-widget-footer">
        <button className="btn btn-primary btn-full" onClick={handleContinueInterview}>
          Continue Interview
        </button>
        <button className="btn btn-secondary btn-small" onClick={() => addTask('Review profile', 'medium')}>
          + Add Task
        </button>
      </div>
    </div>
  );
};

export default CareerConciergeWidget;

// Register as a vanilla WidgetModule via React adapter
createReactWidget({
  id: 'career-concierge',
  title: 'Career Concierge',
  span: 'col-span-1 row-span-2',
  component: CareerConciergeWidget,
});
