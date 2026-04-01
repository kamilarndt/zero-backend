/**
 * GraphicDesignerWidget - Portfolio & Design DNA widget
 * Displays color palette, typography, CV status, and portfolio preview
 */

import React from 'react';
import { useGraphicDesignerData } from '../../hooks/useWidgetData';
import '../../styles/dashboard.css';

export interface GraphicDesignerWidgetProps {
  onChat?: (agent: 'career-concierge' | 'graphic-designer') => void;
  isActive?: boolean;
}

export const GraphicDesignerWidget: React.FC<GraphicDesignerWidgetProps> = ({
  onChat,
  isActive = false,
}) => {
  const { data, isLoading, setCVStatus } = useGraphicDesignerData();

  // Demo data for when no real data is available
  const designDNA = data.designDNA || {
    colors: [
      { hex: '#0d1117', name: 'Rich Black' },
      { hex: '#58a6ff', name: 'Azure' },
      { hex: '#3fb950', name: 'Emerald' },
      { hex: '#d29922', name: 'Gold' },
      { hex: '#f85149', name: 'Coral' },
    ],
    typography: {
      heading: 'Inter, sans-serif',
      body: 'JetBrains Mono, monospace',
    },
    spacing: {
      base: 16,
      scale: 1.5,
    },
  };

  const cvStatuses = data.cvs.length > 0 ? data.cvs : [
    { format: 'pdf' as const, ready: false },
    { format: 'word' as const, ready: false },
    { format: 'latex' as const, ready: false },
  ];

  const handleGenerateCVs = () => {
    onChat?.('graphic-designer');
    // Simulate generation
    setCVStatus('pdf', true);
    setTimeout(() => setCVStatus('word', true), 1000);
    setTimeout(() => setCVStatus('latex', true), 2000);
  };

  const getFormatLabel = (format: string) => {
    switch (format) {
      case 'pdf':
        return 'PDF';
      case 'word':
        return 'Word';
      case 'latex':
        return 'LaTeX';
      default:
        return format.toUpperCase();
    }
  };

  return (
    <div className={`career-widget-card ${isActive ? 'active' : ''}`}>
      <div className="career-widget-header">
        <div className="career-widget-title">
          <div className="career-widget-icon graphic-designer">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="16" height="16">
              <path d="M12 19l7-7 3 3-7 7-3-3z" />
              <path d="M18 13l-1.5-7.5L2 2l3.5 14.5L13 18l5-5z" />
              <path d="M2 2l7.586 7.586" />
              <circle cx="11" cy="11" r="2" />
            </svg>
          </div>
          <span>Graphic Designer</span>
        </div>
        <div className={`connection-status-dot ${!isLoading ? 'connected' : ''}`} />
      </div>

      <div className="career-widget-body">
        {/* Design DNA Preview */}
        <h4 style={{ margin: '0 0 var(--spacing-sm)', fontSize: 'var(--font-size-base)', color: 'var(--text)' }}>
          Design DNA
        </h4>

        <div className="design-dna-preview">
          {/* Color Palette */}
          <div className="color-palette">
            {designDNA.colors.map((color, index) => (
              <div
                key={index}
                className="color-swatch"
                style={{ backgroundColor: color.hex }}
                data-hex={color.hex}
                title={color.name}
              />
            ))}
          </div>

          {/* Typography Sample */}
          <div className="typography-sample">
            <div className="typography-heading" style={{ fontFamily: designDNA.typography.heading }}>
              Heading Text
            </div>
            <div className="typography-body" style={{ fontFamily: designDNA.typography.body }}>
              Body text in mono font
            </div>
          </div>
        </div>

        {/* CV Generation Status */}
        <h4 style={{ margin: 'var(--spacing-md) 0 var(--spacing-sm)', fontSize: 'var(--font-size-base)', color: 'var(--text)' }}>
          CV Templates
        </h4>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--spacing-xs)' }}>
          {cvStatuses.map((cv) => (
            <div
              key={cv.format}
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                padding: 'var(--spacing-sm)',
                background: 'var(--bg-mantle)',
                borderRadius: 'var(--radius-sm)',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--spacing-sm)' }}>
                <div
                  style={{
                    width: '8px',
                    height: '8px',
                    borderRadius: '50%',
                    background: cv.ready ? 'var(--green)' : 'var(--overlay)',
                  }}
                />
                <span style={{ fontSize: 'var(--font-size-sm)', color: 'var(--text)' }}>
                  {getFormatLabel(cv.format)}
                </span>
              </div>
              {cv.ready && (
                <button
                  className="btn btn-ghost btn-small"
                  style={{ padding: '2px 8px', fontSize: '10px' }}
                  onClick={() => {/* Download handler */}}
                >
                  Download
                </button>
              )}
            </div>
          ))}
        </div>

        {/* Portfolio Preview */}
        <h4 style={{ margin: 'var(--spacing-md) 0 var(--spacing-sm)', fontSize: 'var(--font-size-base)', color: 'var(--text)' }}>
          Portfolio
        </h4>

        {data.portfolioPreview ? (
          <div
            style={{
              width: '100%',
              height: '120px',
              background: 'var(--bg-mantle)',
              borderRadius: 'var(--radius-md)',
              backgroundImage: `url(${data.portfolioPreview})`,
              backgroundSize: 'cover',
              backgroundPosition: 'center',
            }}
          />
        ) : (
          <div
            className="empty-state"
            style={{
              width: '100%',
              height: '120px',
              background: 'var(--bg-mantle)',
              borderRadius: 'var(--radius-md)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <span className="empty-state-text" style={{ fontSize: 'var(--font-size-sm)' }}>
              No portfolio yet
            </span>
          </div>
        )}

        {isLoading && (
          <div style={{ textAlign: 'center', padding: 'var(--spacing-md)', color: 'var(--overlay)' }}>
            Generating...
          </div>
        )}
      </div>

      <div className="career-widget-footer">
        <button className="btn btn-primary btn-full" onClick={handleGenerateCVs}>
          Generate CVs
        </button>
        <button className="btn btn-secondary btn-small" onClick={() => onChat?.('graphic-designer')}>
          Update Design DNA
        </button>
      </div>
    </div>
  );
};

export default GraphicDesignerWidget;

// Register as a vanilla WidgetModule via React adapter
import { createReactWidget } from '../ReactWidgetAdapter';
createReactWidget({
  id: 'graphic-designer',
  title: 'Graphic Designer',
  span: 'col-span-1 row-span-2',
  component: GraphicDesignerWidget,
});
