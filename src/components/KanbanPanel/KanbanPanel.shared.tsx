import { useState, useEffect, useCallback, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import type { BlockDto, KanbanBlockDto } from '../../lib/types';
import { COLUMNS, COLUMN_CONFIG, type ColumnId } from './constants';
import * as api from '../../lib/commands';

export interface UseKanbanPanelReturn {
  blocks: KanbanBlockDto[];
  loading: boolean;
  error: string | null;
  activeId: string | null;
  newCardColumn: ColumnId | null;
  newCardText: string;
  doneCollapsed: boolean;
  editingBlock: KanbanBlockDto | null;
  contextMenu: { block: KanbanBlockDto; x: number; y: number } | null;
  grouped: Record<ColumnId, KanbanBlockDto[]>;
  columnItemIds: Record<ColumnId, string[]>;
  setActiveId: (id: string | null) => void;
  setNewCardColumn: (col: ColumnId | null) => void;
  setNewCardText: (text: string) => void;
  setDoneCollapsed: (collapsed: boolean | ((prev: boolean) => boolean)) => void;
  setEditingBlock: (block: KanbanBlockDto | null) => void;
  setContextMenu: (menu: { block: KanbanBlockDto; x: number; y: number } | null) => void;
  setError: (err: string | null) => void;
  loadBlocks: () => Promise<void>;
  handleDragEnd: (event: { active: { id: string }; over: { id: string } | null }) => Promise<void>;
  handleAddCard: (columnId: ColumnId) => Promise<void>;
  handleEditSave: (updated: KanbanBlockDto) => Promise<void>;
  handleDeleteBlock: (block: KanbanBlockDto) => Promise<void>;
  handleEditFromContextMenu: (block: KanbanBlockDto) => void;
  handleContextMenu: (block: KanbanBlockDto, e: React.MouseEvent) => void;
  openTodaysJournal: () => void;
  findColumn: (id: string) => ColumnId | null;
}

export function useKanbanPanel(): UseKanbanPanelReturn {
  const navigate = useNavigate();

  const [blocks, setBlocks] = useState<KanbanBlockDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [newCardColumn, setNewCardColumn] = useState<ColumnId | null>(null);
  const [newCardText, setNewCardText] = useState('');
  const [doneCollapsed, setDoneCollapsed] = useState(false);
  const [editingBlock, setEditingBlock] = useState<KanbanBlockDto | null>(null);
  const [contextMenu, setContextMenu] = useState<{
    block: KanbanBlockDto;
    x: number;
    y: number;
  } | null>(null);

  const loadBlocks = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await api.getKanbanBlocks();
      setBlocks(data.blocks);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadBlocks();
  }, [loadBlocks]);

  const grouped = useMemo(() => {
    const result: Record<ColumnId, KanbanBlockDto[]> = {
      todo: [],
      in_progress: [],
      done: [],
    };
    for (const block of blocks) {
      const marker = block.marker?.toUpperCase() ?? '';
      if (COLUMN_CONFIG.todo.markers.includes(marker)) {
        result.todo.push(block);
      } else if (COLUMN_CONFIG.in_progress.markers.includes(marker)) {
        result.in_progress.push(block);
      } else if (COLUMN_CONFIG.done.markers.includes(marker)) {
        result.done.push(block);
      }
    }
    return result;
  }, [blocks]);

  const columnItemIds = useMemo(() => {
    const result: Record<ColumnId, string[]> = {
      todo: [],
      in_progress: [],
      done: [],
    };
    for (const col of COLUMNS) {
      result[col] = grouped[col].map((b) => b.id);
    }
    return result;
  }, [grouped]);

  function findColumn(id: string): ColumnId | null {
    if (id in COLUMN_CONFIG) return id as ColumnId;
    for (const col of COLUMNS) {
      if (columnItemIds[col].includes(id)) return col;
    }
    return null;
  }

  async function handleDragEnd(event: { active: { id: string }; over: { id: string } | null }) {
    setActiveId(null);
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const fromCol = findColumn(String(active.id));
    const toCol = findColumn(String(over.id));
    if (!fromCol || !toCol) return;
    if (fromCol === toCol) return;

    const block = blocks.find((b) => b.id === active.id);
    if (!block) return;

    // Optimistic update
    const targetMarker = COLUMN_CONFIG[toCol].markers[0];
    setBlocks((prev) =>
      prev.map((b) => (b.id === active.id ? { ...b, marker: targetMarker } : b)),
    );

    // Persist
    try {
      await api.updateBlock(block.page_path, {
        ...block,
        marker: targetMarker,
      } as BlockDto);
      await loadBlocks();
    } catch {
      await loadBlocks();
    }
  }

  async function handleAddCard(columnId: ColumnId) {
    if (!newCardText.trim()) return;
    const marker = COLUMN_CONFIG[columnId].markers[0];
    try {
      await api.createKanbanBlock(newCardText.trim(), marker);
      setNewCardText('');
      setNewCardColumn(null);
      await loadBlocks();
    } catch (e) {
      setError(String(e));
    }
  }

  function handleContextMenu(block: KanbanBlockDto, e: React.MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ block, x: e.clientX - 2, y: e.clientY - 4 });
  }

  async function handleDeleteBlock(block: KanbanBlockDto) {
    try {
      await api.deleteBlock(block.id);
      await loadBlocks();
    } catch (e) {
      setError(String(e));
    }
    setContextMenu(null);
  }

  function handleEditFromContextMenu(block: KanbanBlockDto) {
    setEditingBlock(block);
    setContextMenu(null);
  }

  async function handleEditSave(updated: KanbanBlockDto) {
    try {
      await api.updateBlock(updated.page_path, updated as BlockDto);
      // Sync .md file by re-saving all blocks on that page
      const pageBlocks = await api.getBlocks(updated.page_path);
      await api.saveBlocks(updated.page_path, pageBlocks.blocks);
      setEditingBlock(null);
      await loadBlocks();
    } catch (e) {
      setError(String(e));
      setEditingBlock(null);
    }
  }

  function openTodaysJournal() {
    const today = new Date();
    const yyyy = today.getFullYear();
    const mm = String(today.getMonth() + 1).padStart(2, '0');
    const dd = String(today.getDate()).padStart(2, '0');
    navigate(`/page/journals/${yyyy}-${mm}-${dd}.md`);
  }

  return {
    blocks,
    loading,
    error,
    activeId,
    newCardColumn,
    newCardText,
    doneCollapsed,
    editingBlock,
    contextMenu,
    grouped,
    columnItemIds,
    setActiveId,
    setNewCardColumn,
    setNewCardText,
    setDoneCollapsed,
    setEditingBlock,
    setContextMenu,
    setError,
    loadBlocks,
    handleDragEnd,
    handleAddCard,
    handleEditSave,
    handleDeleteBlock,
    handleEditFromContextMenu,
    handleContextMenu,
    openTodaysJournal,
    findColumn,
  };
}
