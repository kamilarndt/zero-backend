import React from 'react'
import { Widget } from './Widget'

interface Task {
  id: string
  name: string
  status: 'running' | 'completed' | 'failed'
  progress: number
}

interface Props {
  tasks?: Task[]
}

export function TaskExecutionWidget({ tasks }: Props) {
  const taskList = tasks || [
    { id: '1', name: 'Refactor auth module', status: 'running', progress: 65 },
    { id: '2', name: 'Run test suite', status: 'completed', progress: 100 },
    { id: '3', name: 'Update documentation', status: 'failed', progress: 30 },
  ]

  return (
    <Widget title="Task Queue">
      <div style={containerStyle}>
        {taskList.map((task) => (
          <div key={task.id} style={taskStyle}>
            <div style={taskHeaderStyle}>
              <span style={taskNameStyle}>{task.name}</span>
              <span style={{ ...statusStyle, ...getStatusColor(task.status) }}>{task.status}</span>
            </div>
            <div style={progressBarStyle}>
              <div
                style={{
                  ...progressFillStyle,
                  width: `${task.progress}%`,
                  background: getProgressColor(task.status),
                }}
              />
            </div>
          </div>
        ))}
      </div>
    </Widget>
  )
}

function getStatusColor(status: string): React.CSSProperties {
  const colors: Record<string, string> = {
    running: '#4a9eff',
    completed: '#00ffaa',
    failed: '#ff4466',
  }
  return { color: colors[status] || '#a0a0b0' }
}

function getProgressColor(status: string): string {
  if (status === 'failed') return '#ff4466'
  if (status === 'completed') return '#00ffaa'
  return '#4a9eff'
}

const containerStyle: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '12px',
}

const taskStyle: React.CSSProperties = {
  background: '#1a1a24',
  padding: '12px',
  borderRadius: '6px',
}

const taskHeaderStyle: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  marginBottom: '8px',
  fontSize: '13px',
}

const taskNameStyle: React.CSSProperties = {
  fontWeight: 500,
  color: '#c0c0d0',
}

const statusStyle: React.CSSProperties = {
  fontSize: '11px',
  textTransform: 'uppercase',
  fontWeight: 600,
}

const progressBarStyle: React.CSSProperties = {
  height: '4px',
  background: '#2a2a3a',
  borderRadius: '2px',
  overflow: 'hidden',
}

const progressFillStyle: React.CSSProperties = {
  height: '100%',
  transition: 'width 0.3s ease',
}
