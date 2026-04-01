import { Widget } from './Widget'

interface LogEntry {
  timestamp: string
  agent: string
  message: string
  level?: 'info' | 'warn' | 'error'
}

interface Props {
  entries?: LogEntry[]
}

export function AgentLogWidget({ entries }: Props) {
  const logs = entries || [
    { timestamp: '14:32:01', agent: 'coder', message: 'Started refactoring auth module' },
    { timestamp: '14:32:15', agent: 'researcher', message: 'Found 3 relevant documents' },
    { timestamp: '14:32:42', agent: 'coder', message: 'Applied patch to src/auth.rs' },
    { timestamp: '14:33:08', agent: 'system', message: 'Build completed in 2.3s' },
  ]

  return (
    <Widget title="Agent Activity Log">
      <div style={logContainerStyle}>
        {logs.map((log, i) => (
          <div key={i} style={logEntryStyle}>
            <span style={timestampStyle}>{log.timestamp}</span>
            <span style={{ ...agentStyle, ...getAgentColor(log.agent) }}>{log.agent}</span>
            <span style={messageStyle}>{log.message}</span>
          </div>
        ))}
      </div>
    </Widget>
  )
}

function getAgentColor(agent: string): React.CSSProperties {
  const colors: Record<string, string> = {
    coder: '#4a9eff',
    researcher: '#ff6b9d',
    system: '#00ffaa',
  }
  return { color: colors[agent] || '#a0a0b0' }
}

const logContainerStyle: React.CSSProperties = {
  fontFamily: 'monospace',
  fontSize: '13px',
}

const logEntryStyle: React.CSSProperties = {
  padding: '6px 0',
  borderBottom: '1px solid #1a1a24',
  display: 'flex',
  gap: '12px',
}

const timestampStyle: React.CSSProperties = {
  color: '#505060',
  minWidth: '70px',
}

const agentStyle: React.CSSProperties = {
  fontWeight: 600,
  minWidth: '80px',
}

const messageStyle: React.CSSProperties = {
  color: '#c0c0d0',
  flex: 1,
}
