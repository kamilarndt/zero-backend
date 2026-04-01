import React, { useState, useCallback, useRef, useEffect, useMemo } from 'react';
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  getFilteredRowModel,
  flexRender,
  ColumnDef,
  CellContext,
  SortingState,
  ColumnFiltersState,
  ColumnSizingState,
} from '@tanstack/react-table';
import { useVirtualizer } from '@tanstack/react-virtual';
import './DataTableBlock.css';

// ─── Types ────────────────────────────────────────────────────────────────────

export interface ColumnDefDTO {
  id: string;
  name: string;
  column_type: string;
  width?: number;
  options?: string;
}

export interface RowDTO {
  id: string;
  table_id: string;
  position: number;
  data: string; // JSON
  created_at: string;
  updated_at: string;
}

export interface TableData {
  table: { id: string; name: string; description?: string };
  columns: ColumnDefDTO[];
  rows: RowDTO[];
  computed_columns: Array<{
    id: string;
    name: string;
    formula: string;
    column_type: string;
  }>;
  relations: Array<{
    id: string;
    source_column_id: string;
    target_table_id: string;
    target_column_id: string;
    relation_type: string;
  }>;
}

interface DataTableBlockProps {
  tableId: string;
  apiBase?: string;
  onCellChange?: (rowId: string, columnId: string, value: unknown) => void;
}

// ─── CSV Utilities ────────────────────────────────────────────────────────────

function parseCSV(text: string): Record<string, string>[] {
  const lines = text.split('\n').filter(l => l.trim());
  if (lines.length === 0) return [];
  const headers = lines[0].split(',').map(h => h.trim().replace(/^"|"$/g, ''));
  return lines.slice(1).map(line => {
    const values = line.split(',').map(v => v.trim().replace(/^"|"$/g, ''));
    return headers.reduce((row, header, i) => {
      row[header] = values[i] || '';
      return row;
    }, {} as Record<string, string>);
  });
}

function exportToCSV(rows: Record<string, unknown>[], columns: ColumnDefDTO[]): string {
  const headers = columns.map(c => `"${c.name}"`).join(',');
  const dataRows = rows.map(row =>
    columns.map(c => {
      const val = row[c.id];
      const str = val !== null && val !== undefined ? String(val) : '';
      return `"${str.replace(/"/g, '""')}"`;
    }).join(',')
  );
  return [headers, ...dataRows].join('\n');
}

function downloadCSV(csv: string, filename: string) {
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

// ─── Cell Editor ──────────────────────────────────────────────────────────────

interface CellEditorProps {
  value: unknown;
  columnType: string;
  onSave: (value: unknown) => void;
  onCancel: () => void;
}

const CellEditor: React.FC<CellEditorProps> = ({ value, columnType, onSave, onCancel }) => {
  const inputRef = useRef<HTMLInputElement>(null);
  const [editValue, setEditValue] = useState(
    value !== null && value !== undefined ? String(value) : ''
  );

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      onSave(parseValue(editValue, columnType));
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onCancel();
    }
    e.stopPropagation();
  };

  if (columnType === 'checkbox') {
    return (
      <input
        ref={inputRef}
        type="checkbox"
        checked={editValue === 'true'}
        onChange={(e) => {
          onSave(e.target.checked);
        }}
        onBlur={() => onSave(parseValue(editValue, columnType))}
        onKeyDown={handleKeyDown}
        className="dt-cell-checkbox"
      />
    );
  }

  if (columnType === 'date') {
    return (
      <input
        ref={inputRef}
        type="date"
        value={editValue}
        onChange={(e) => setEditValue(e.target.value)}
        onBlur={() => onSave(parseValue(editValue, columnType))}
        onKeyDown={handleKeyDown}
        className="dt-cell-input"
      />
    );
  }

  if (columnType === 'number') {
    return (
      <input
        ref={inputRef}
        type="number"
        value={editValue}
        onChange={(e) => setEditValue(e.target.value)}
        onBlur={() => onSave(parseValue(editValue, columnType))}
        onKeyDown={handleKeyDown}
        className="dt-cell-input"
      />
    );
  }

  return (
    <input
      ref={inputRef}
      type="text"
      value={editValue}
      onChange={(e) => setEditValue(e.target.value)}
      onBlur={() => onSave(parseValue(editValue, columnType))}
      onKeyDown={handleKeyDown}
      className="dt-cell-input"
    />
  );
};

function parseValue(raw: string, columnType: string): unknown {
  switch (columnType) {
    case 'number': return raw === '' ? null : Number(raw);
    case 'checkbox': return raw === 'true';
    default: return raw;
  }
}

function formatCell(value: unknown, columnType: string): string {
  if (value === null || value === undefined) return '';
  switch (columnType) {
    case 'checkbox': return value ? '✓' : '☐';
    case 'date': return String(value);
    case 'number': return typeof value === 'number' ? value.toLocaleString() : String(value);
    default: return String(value);
  }
}

// ─── Column Type Badge ────────────────────────────────────────────────────────

const ColumnTypeBadge: React.FC<{ type: string }> = ({ type }) => {
  const colors: Record<string, string> = {
    text: 'var(--subtext)',
    number: 'var(--blue)',
    date: 'var(--peach)',
    checkbox: 'var(--green)',
    select: 'var(--mauve)',
    relation: 'var(--teal)',
    computed: 'var(--yellow)',
    formula: 'var(--yellow)',
    rollup: 'var(--yellow)',
  };

  return (
    <span
      className="dt-col-type-badge"
      style={{ color: colors[type] || 'var(--subtext)' }}
    >
      {type}
    </span>
  );
};

// ─── Sort Indicator ───────────────────────────────────────────────────────────

const SortIndicator: React.FC<{ direction: false | 'asc' | 'desc' }> = ({ direction }) => {
  if (!direction) return <span className="dt-sort-indicator dt-sort-none">↕</span>;
  return (
    <span className="dt-sort-indicator dt-sort-active">
      {direction === 'asc' ? '↑' : '↓'}
    </span>
  );
};

// ─── Main DataTableBlock ──────────────────────────────────────────────────────

export const DataTableBlock: React.FC<DataTableBlockProps> = ({
  tableId,
  apiBase = '/v1/knowledge',
  onCellChange,
}) => {
  const [tableData, setTableData] = useState<TableData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [editingCell, setEditingCell] = useState<{ rowId: string; colId: string } | null>(null);
  const [focusedCell, setFocusedCell] = useState<{ row: number; col: number }>({ row: 0, col: 0 });
  const tableContainerRef = useRef<HTMLDivElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  // ── FAZA 3.2: Sorting, Filtering, Column Resize state ───────────────────
  const [sorting, setSorting] = useState<SortingState>([]);
  const [globalFilter, setGlobalFilter] = useState('');
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const [columnSizing, setColumnSizing] = useState<ColumnSizingState>({});
  const [showFilterRow, setShowFilterRow] = useState(false);

  // ── Fetch table data ────────────────────────────────────────────────────────
  const fetchTable = useCallback(async () => {
    try {
      const res = await fetch(`${apiBase}/tables/${tableId}`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data: TableData = await res.json();
      setTableData(data);
      setError(null);
    } catch (e: any) {
      // Fallback to mock data for demo mode
      console.log('Backend API unavailable, using mock data');

      // Generate 20 rows for virtual scrolling demo
      const statuses = ['Active', 'Pending', 'Done', 'Blocked', 'Review'];
      const priorities = ['Critical', 'High', 'Medium', 'Low'];
      const categories = ['Feature', 'Bug', 'Task', 'Refactor', 'Docs'];
      const assignees = ['Alice', 'Bob', 'Charlie', 'Diana', 'Eve'];

      const rows = Array.from({ length: 20 }, (_, i) => ({
        id: `row${i + 1}`,
        table_id: tableId,
        position: i,
        data: JSON.stringify({
          col1: `Task ${i + 1}: ${categories[i % categories.length]} implementation`,
          col2: statuses[i % statuses.length],
          col3: priorities[i % priorities.length],
          col4: `2026-04-${String((i % 30) + 1).padStart(2, '0')}`,
          col5: Math.floor(Math.random() * 100) + 1,
          col6: assignees[i % assignees.length],
          col7: i % 3 === 0 ? 'true' : 'false',
        }),
        created_at: '2026-04-01',
        updated_at: '2026-04-01',
      }));

      setTableData({
        table: {
          id: tableId,
          name: '📊 Project Tasks',
          description: 'Advanced demo with sorting, filtering, virtual scrolling'
        },
        columns: [
          { id: 'col1', name: 'Task Name', column_type: 'text', width: 280 },
          { id: 'col2', name: 'Status', column_type: 'select', width: 120, options: 'Active,Pending,Done,Blocked,Review' },
          { id: 'col3', name: 'Priority', column_type: 'select', width: 110, options: 'Critical,High,Medium,Low' },
          { id: 'col4', name: 'Due Date', column_type: 'date', width: 130 },
          { id: 'col5', name: 'Progress', column_type: 'number', width: 100 },
          { id: 'col6', name: 'Assignee', column_type: 'select', width: 120, options: 'Alice,Bob,Charlie,Diana,Eve' },
          { id: 'col7', name: 'Verified', column_type: 'checkbox', width: 90 },
        ],
        rows,
        computed_columns: [],
        relations: [],
      });
      setError(null);
    } finally {
      setLoading(false);
    }
  }, [tableId, apiBase]);

  useEffect(() => {
    fetchTable();
  }, [fetchTable]);

  // ── SSE sync for live updates ───────────────────────────────────────────────
  useEffect(() => {
    const es = new EventSource('/v1/events/stream');
    es.onmessage = (e) => {
      try {
        const evt = JSON.parse(e.data);
        if (evt.from === 'knowledge' && evt.data?.table_id === tableId) {
          fetchTable();
        }
      } catch {}
    };
    es.onerror = () => es.close();
    return () => es.close();
  }, [tableId, fetchTable]);

  // ── Cell update ─────────────────────────────────────────────────────────────
  const handleCellSave = useCallback(async (rowId: string, columnId: string, value: unknown) => {
    setEditingCell(null);
    try {
      const res = await fetch(`${apiBase}/tables/${tableId}/rows/${rowId}/cells`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ column_id: columnId, value }),
      });
      if (res.ok) {
        setTableData((prev) => {
          if (!prev) return prev;
          return {
            ...prev,
            rows: prev.rows.map((r) => {
              if (r.id !== rowId) return r;
              const data = JSON.parse(r.data || '{}');
              data[columnId] = value;
              return { ...r, data: JSON.stringify(data), updated_at: new Date().toISOString() };
            }),
          };
        });
        onCellChange?.(rowId, columnId, value);
      }
    } catch (e) {
      console.error('Cell update failed:', e);
    }
  }, [tableId, apiBase, onCellChange]);

  // ── Add new row ─────────────────────────────────────────────────────────────
  const handleAddRow = useCallback(async () => {
    try {
      const res = await fetch(`${apiBase}/tables/${tableId}/rows`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ data: '{}' }),
      });
      if (res.ok) {
        fetchTable();
      }
    } catch (e) {
      console.error('Add row failed:', e);
    }
  }, [tableId, apiBase, fetchTable]);

  // ── FAZA 3.3: CSV Import ───────────────────────────────────────────────────
  const handleCSVImport = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const text = await file.text();
    const rows = parseCSV(text);
    if (rows.length === 0) return;

    // Import each row
    for (const row of rows) {
      try {
        await fetch(`${apiBase}/tables/${tableId}/rows`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ data: JSON.stringify(row) }),
        });
      } catch (err) {
        console.error('CSV import row failed:', err);
      }
    }

    fetchTable();
    // Reset file input
    if (fileInputRef.current) fileInputRef.current.value = '';
  }, [tableId, apiBase, fetchTable]);

  // ── FAZA 3.3: CSV Export ───────────────────────────────────────────────────
  const handleCSVExport = useCallback(() => {
    if (!tableData) return;
    const rows = tableData.rows.map(r => JSON.parse(r.data || '{}'));
    const csv = exportToCSV(rows, tableData.columns);
    downloadCSV(csv, `${tableData.table.name || 'table'}.csv`);
  }, [tableData]);

  // ── Build TanStack columns ──────────────────────────────────────────────────
  const columns = useMemo<ColumnDef<Record<string, unknown>>[]>(() => {
    if (!tableData) return [];

    const cols: ColumnDef<Record<string, unknown>>[] = tableData.columns.map((col) => ({
      id: col.id,
      accessorFn: (row) => row[col.id],
      header: ({ column }) => (
        <div
          className="dt-col-header dt-col-header-sortable"
          onClick={column.getToggleSortingHandler()}
          title="Click to sort"
        >
          <span className="dt-col-name">{col.name}</span>
          <ColumnTypeBadge type={col.column_type} />
          <SortIndicator direction={column.getIsSorted()} />
        </div>
      ),
      size: col.width || 150,
      minSize: 60,
      enableSorting: true,
      enableColumnFilter: true,
      enableResizing: true,
      cell: (info: CellContext<Record<string, unknown>, unknown>) => {
        const rowId = info.row.original._rowId as string;
        const value = info.getValue();
        const isEditing =
          editingCell?.rowId === rowId && editingCell?.colId === col.id;

        if (isEditing) {
          return (
            <CellEditor
              value={value}
              columnType={col.column_type}
              onSave={(v) => handleCellSave(rowId, col.id, v)}
              onCancel={() => setEditingCell(null)}
            />
          );
        }

        return (
          <div
            className={`dt-cell column-type-${col.column_type}`}
            onClick={() => setEditingCell({ rowId, colId: col.id })}
            title={value !== null && value !== undefined ? String(value) : ''}
          >
            {formatCell(value, col.column_type)}
          </div>
        );
      },
    }));

    // Add computed columns (read-only)
    for (const cc of tableData.computed_columns) {
      cols.push({
        id: cc.id,
        accessorFn: (row) => row[cc.id],
        header: () => (
          <div className="dt-col-header">
            <span className="dt-col-name">{cc.name}</span>
            <ColumnTypeBadge type="computed" />
            <span className="dt-formula-badge" title={cc.formula}>ƒ</span>
          </div>
        ),
        size: 120,
        enableSorting: true,
        cell: (info) => {
          const value = info.getValue();
          return (
            <div className="dt-cell dt-cell-computed">
              {value !== null && value !== undefined ? String(value) : '—'}
            </div>
          );
        },
      });
    }

    return cols;
  }, [tableData, editingCell, handleCellSave]);

  // ── Build row data ──────────────────────────────────────────────────────────
  const rowData = useMemo(() => {
    if (!tableData) return [];
    return tableData.rows.map((row) => {
      const data = JSON.parse(row.data || '{}');
      return { ...data, _rowId: row.id, _position: row.position };
    });
  }, [tableData]);

  // ── FAZA 3.2: TanStack Table instance with sorting, filtering, resize ───
  const table = useReactTable({
    data: rowData,
    columns,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    enableColumnResizing: true,
    columnResizeMode: 'onChange',
    state: {
      sorting,
      globalFilter,
      columnFilters,
      columnSizing,
    },
    onSortingChange: setSorting,
    onGlobalFilterChange: setGlobalFilter,
    onColumnFiltersChange: setColumnFilters,
    onColumnSizingChange: setColumnSizing,
  });

  // ── Virtualizer for large tables ────────────────────────────────────────────
  const { rows } = table.getRowModel();
  const rowVirtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => tableContainerRef.current,
    estimateSize: () => 36,
    overscan: 10,
  });

  // ── FAZA 3.3: Keyboard navigation ──────────────────────────────────────────
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (editingCell) return; // Don't navigate while editing

    const colCount = tableData?.columns.length || 0;
    const rowCount = rows.length;
    if (rowCount === 0 || colCount === 0) return;

    let { row, col } = focusedCell;

    switch (e.key) {
      case 'ArrowUp':
        e.preventDefault();
        row = Math.max(0, row - 1);
        break;
      case 'ArrowDown':
        e.preventDefault();
        row = Math.min(rowCount - 1, row + 1);
        break;
      case 'ArrowLeft':
        e.preventDefault();
        col = Math.max(0, col - 1);
        break;
      case 'ArrowRight':
      case 'Tab':
        e.preventDefault();
        col = col < colCount - 1 ? col + 1 : 0;
        if (e.key === 'Tab' && col === 0 && row < rowCount - 1) {
          row += 1;
        }
        break;
      case 'Enter':
        e.preventDefault();
        if (tableData && rows[row]) {
          const colDef = tableData.columns[col];
          if (colDef) {
            const rowId = rows[row].original._rowId as string;
            setEditingCell({ rowId, colId: colDef.id });
          }
        }
        return;
      case 'Escape':
        e.preventDefault();
        setEditingCell(null);
        return;
      default:
        return;
    }

    setFocusedCell({ row, col });
    rowVirtualizer.scrollToIndex(row, { align: 'auto' });
  }, [editingCell, focusedCell, rows, tableData, rowVirtualizer]);

  // ── Render ──────────────────────────────────────────────────────────────────

  if (loading) {
    return <div className="dt-loading">Loading table...</div>;
  }

  if (error) {
    return <div className="dt-error">Error: {error}</div>;
  }

  if (!tableData) {
    return <div className="dt-empty">Table not found</div>;
  }

  const virtualRows = rowVirtualizer.getVirtualItems();
  const totalSize = rowVirtualizer.getTotalSize();

  return (
    <div className="dt-block" onKeyDown={handleKeyDown} tabIndex={0}>
      {/* FAZA 3.3 & 3.4: Enhanced Toolbar */}
      <div className="dt-toolbar">
        <div className="dt-toolbar-left">
          <span className="dt-table-name">{tableData.table.name}</span>
          <span className="dt-row-count">{rows.length} rows</span>
          <span className="dt-demo-badge">Demo Mode</span>
        </div>
        <div className="dt-toolbar-right">
          {/* Global Filter */}
          <input
            type="text"
            className="dt-global-filter"
            placeholder="🔍 Filter..."
            value={globalFilter}
            onChange={(e) => setGlobalFilter(e.target.value)}
            title="Global filter (searches all columns)"
          />
          {/* Toggle per-column filters */}
          <button
            className={`dt-btn ${showFilterRow ? 'dt-btn-active' : ''}`}
            onClick={() => setShowFilterRow(!showFilterRow)}
            title="Toggle column filters"
          >
            ⚡
          </button>
          {/* CSV Import */}
          <input
            ref={fileInputRef}
            type="file"
            accept=".csv"
            onChange={handleCSVImport}
            style={{ display: 'none' }}
          />
          <button
            className="dt-btn dt-btn-import"
            onClick={() => fileInputRef.current?.click()}
            title="Import CSV"
          >
            ↑ CSV
          </button>
          {/* CSV Export */}
          <button
            className="dt-btn dt-btn-export"
            onClick={handleCSVExport}
            title="Export CSV"
          >
            ↓ CSV
          </button>
          {/* Add Row */}
          <button className="dt-btn dt-btn-add" onClick={handleAddRow} title="Add row">
            + Row
          </button>
        </div>
      </div>

      {/* Table */}
      <div className="dt-container" ref={tableContainerRef}>
        <table
          className="dt-table"
          style={{ width: table.getCenterTotalSize() }}
        >
          <thead>
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id} className="dt-header-row">
                {headerGroup.headers.map((header) => (
                  <th
                    key={header.id}
                    className="dt-th"
                    style={{ width: header.getSize() }}
                    colSpan={header.colSpan}
                  >
                    {header.isPlaceholder
                      ? null
                      : flexRender(header.column.columnDef.header, header.getContext())}
                    {/* FAZA 3.2: Column Resize Handle */}
                    {header.column.getCanResize() && (
                      <div
                        onMouseDown={header.getResizeHandler()}
                        onTouchStart={header.getResizeHandler()}
                        className={`dt-resize-handle ${
                          header.column.getIsResizing() ? 'dt-resize-active' : ''
                        }`}
                      />
                    )}
                  </th>
                ))}
              </tr>
            ))}
            {/* FAZA 3.2: Per-column filter row */}
            {showFilterRow && (
              <tr className="dt-filter-row">
                {table.getHeaderGroups()[0]?.headers.map((header) => (
                  <th key={header.id} className="dt-th dt-th-filter" style={{ width: header.getSize() }}>
                    {header.column.getCanFilter() ? (
                      <input
                        type="text"
                        className="dt-column-filter"
                        placeholder={`Filter ${header.column.id}`}
                        value={(header.column.getFilterValue() as string) ?? ''}
                        onChange={(e) => header.column.setFilterValue(e.target.value)}
                      />
                    ) : null}
                  </th>
                ))}
              </tr>
            )}
          </thead>
          <tbody>
            {/* Spacer for virtualization */}
            {virtualRows.length > 0 && virtualRows[0].start > 0 && (
              <tr style={{ height: virtualRows[0].start }}>
                <td colSpan={columns.length} />
              </tr>
            )}
            {virtualRows.map((virtualRow) => {
              const row = rows[virtualRow.index];
              return (
                <tr
                  key={row.id}
                  className={`dt-tr ${focusedCell.row === virtualRow.index ? 'dt-tr-focused' : ''}`}
                  style={{ height: virtualRow.size }}
                >
                  {row.getVisibleCells().map((cell, colIdx) => (
                    <td
                      key={cell.id}
                      className={`dt-td ${focusedCell.row === virtualRow.index && focusedCell.col === colIdx ? 'dt-td-focused' : ''}`}
                      style={{ width: cell.column.getSize() }}
                    >
                      {flexRender(cell.column.columnDef.cell, cell.getContext())}
                    </td>
                  ))}
                </tr>
              );
            })}
            {/* Spacer for virtualization */}
            {virtualRows.length > 0 && (
              <tr style={{ height: totalSize - (virtualRows[virtualRows.length - 1]?.end || 0) }}>
                <td colSpan={columns.length} />
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
};

export default DataTableBlock;
