import React from 'react'

interface WidgetProps {
  title: string
  children: React.ReactNode
}

export function Widget({ title, children }: WidgetProps) {
  return (
    <div style={widgetStyle}>
      <div style={headerStyle}>{title}</div>
      <div style={contentStyle}>{children}</div>
    </div>
  )
}

const widgetStyle: React.CSSProperties = {
  background: '#12121a',
  border: '1px solid #2a2a3a',
  borderRadius: '8px',
  overflow: 'hidden',
  display: 'flex',
  flexDirection: 'column',
}

const headerStyle: React.CSSProperties = {
  padding: '12px 16px',
  background: '#1a1a24',
  borderBottom: '1px solid #2a2a3a',
  fontWeight: 600,
  fontSize: '14px',
  color: '#a0a0b0',
  textTransform: 'uppercase',
  letterSpacing: '0.5px',
}

const contentStyle: React.CSSProperties = {
  padding: '16px',
  flex: 1,
  overflow: 'auto',
}
