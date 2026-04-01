/**
 * KnowledgeWidget - Notion-style KB with BlockNote editor
 *
 * Features:
 * - BlockNote rich text editor
 * - DataTable blocks embedded in documents
 * - Auto-save to backend
 *
 * Registered via ReactWidgetAdapter to bridge React hooks → vanilla registry.
 */

import React, { useCallback, useState, useEffect } from 'react';
import { BlockNoteView } from '@blocknote/mantine';
import {
  BlockNoteSchema,
  defaultBlockSpecs,
} from '@blocknote/core';
import '@blocknote/core/fonts/inter.css';
import '@blocknote/mantine/style.css';

import { createReactWidget } from '../ReactWidgetAdapter';

// ─── Knowledge Schema (default blocks only) ───────────────────────────

export const knowledgeSchema = BlockNoteSchema.create({
  blockSpecs: {
    ...defaultBlockSpecs,
  },
});

// ─── Knowledge Widget Component ──────────────────────────────────────────

export interface KnowledgeWidgetProps {
  documentId?: string;
  apiBase?: string;
}

export const KnowledgeWidget: React.FC<KnowledgeWidgetProps> = ({
  documentId = 'default',
  apiBase = '/v1/knowledge',
}) => {
  const [editor, setEditor] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Initialize BlockNote editor
  useEffect(() => {
    async function initEditor() {
      try {
        // Dynamically import BlockNoteEditor to avoid SSR issues
        const { BlockNoteEditor } = await import('@blocknote/core');

        // Create editor instance
        const instance = BlockNoteEditor.create({
          schema: knowledgeSchema,
          initialContent: [
            {
              type: 'heading',
              content: '📚 Knowledge Base',
            },
            {
              type: 'paragraph',
              content: 'Welcome to your Notion-style knowledge base. Type "/" to see available blocks.',
            },
            {
              type: 'paragraph',
              content: 'Try adding a DataTable by typing "/" and selecting "Data Table".',
            },
          ],
        });

        setEditor(instance);
        setLoading(false);
      } catch (e: any) {
        setError(e.message);
        setLoading(false);
      }
    }

    initEditor();
  }, [documentId]);

  // Auto-save handler
  const handleChange = useCallback(() => {
    if (!editor) return;

    // Debounced save to backend
    const content = editor.document;
    console.log('Saving document:', documentId, content);

    // TODO: Implement backend save
    // fetch(`${apiBase}/documents/${documentId}`, {
    //   method: 'POST',
    //   headers: { 'Content-Type': 'application/json' },
    //   body: JSON.stringify({ content }),
    // });
  }, [editor, documentId, apiBase]);

  if (loading) {
    return (
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        height: '100%',
        color: 'var(--subtext)',
        fontFamily: 'var(--font-mono)',
      }}>
        Loading KB...
      </div>
    );
  }

  if (error) {
    return (
      <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        height: '100%',
        color: 'var(--red)',
        fontFamily: 'var(--font-mono)',
      }}>
        Error: {error}
      </div>
    );
  }

  return (
    <div className="knowledge-widget-container">
      <BlockNoteView
        editor={editor}
        onChange={handleChange}
        theme="dark"
      />
    </div>
  );
};

export default KnowledgeWidget;

// Register as a vanilla WidgetModule via React adapter
createReactWidget({
  id: 'knowledge',
  title: '📚 Knowledge Base',
  span: 'col-span-2 row-span-2',
  component: KnowledgeWidget,
  props: {
    documentId: 'default',
    apiBase: '/v1/knowledge',
  },
});
