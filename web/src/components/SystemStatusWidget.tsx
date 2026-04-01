import { Widget } from './Widget'

interface SystemStatus {
  cpu: number
  memory: number
  agents: number
  uptime: string
}

interface Props {
  data?: SystemStatus
}

export function SystemStatusWidget({ data }: Props) {
  const status = data || { cpu: 45, memory: 62, agents: 3, uptime: '2h 14m' }

  return (
    <Widget title="System Status">
      <div style={gridStyle}>
        <div style={itemStyle}>
          <div style={labelStyle}>CPU</div>
          <div style={valueStyle}>{status.cpu}%</div>
        </div>
        <div style={itemStyle}>
          <div style={labelStyle}>Memory</div>
          <div style={valueStyle}>{status.memory}%</div>
        </div>
        <div style={itemStyle}>
          <div style={labelStyle}>Agents</div>
          <div style={valueStyle}>{status.agents}</div>
        </div>
        <div style={itemStyle}>
          <div style={labelStyle}>Uptime</div>
          <div style={valueStyle}>{status.uptime}</div>
        </div>
      </div>
    </Widget>
  )
}

const gridStyle: React.CSSProperties = {
  display: 'grid',
  gridTemplateColumns: '1fr 1fr',
  gap: '16px',
}

const itemStyle: React.CSSProperties = {
  background: '#1a1a24',
  padding: '12px',
  borderRadius: '6px',
  textAlign: 'center',
}

const labelStyle: React.CSSProperties = {
  fontSize: '11px',
  color: '#606070',
  marginBottom: '4px',
  textTransform: 'uppercase',
}

const valueStyle: React.CSSProperties = {
  fontSize: '20px',
  fontWeight: 700,
  color: '#00ffaa',
}
