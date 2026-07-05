import { useState, useEffect, useCallback, useMemo } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import Card from '@mui/material/Card';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import CircularProgress from '@mui/material/CircularProgress';
import Alert from '@mui/material/Alert';
import RefreshIcon from '@mui/icons-material/Refresh';
import {
  DndContext,
  DragOverlay,
  closestCorners,
  PointerSensor,
  useSensor,
  useSensors,
  type DragStartEvent,
  type DragEndEvent,
} from '@dnd-kit/core';
import { useNavigate } from 'react-router-dom';
import * as api from '../../lib/commands';
import type { BlockDto, KanbanBlockDto } from '../../lib/types';
import KanbanEditDialog from '../KanbanEditDialog';
import KanbanColumn from './KanbanColumn';
import { COLUMNS, COLUMN_CONFIG, type ColumnId } from './constants';

export default function KanbanPanel() {
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
    // Initial data fetch — runs once since loadBlocks is stable ([] deps)
    // eslint-disable-next-line react-hooks/set-state-in-effect
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
    const result: Record<ColumnId, string[]> = { todo: [], in_progress: [], done: [] };
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

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
  );

  function handleDragStart(event: DragStartEvent) {
    setActiveId(String(event.active.id));
  }

  async function handleDragEnd(event: DragEndEvent) {
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
      await api.updateBlock(block.page_path, { ...block, marker: targetMarker } as BlockDto);
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

  // -----------------------------------------------------------------------
  // Render
  // -----------------------------------------------------------------------

  if (loading) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <CircularProgress size={24} />
      </Box>
    );
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
      {/* Header */}
      <Box
        sx={{
          px: 2,
          py: 1.5,
          borderBottom: 1,
          borderColor: 'divider',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <Typography variant="h6" sx={{ fontWeight: 700 }}>
          Kanban
        </Typography>
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button
            size="small"
            variant="text"
            onClick={openTodaysJournal}
            sx={{ textTransform: 'none', fontSize: '0.75rem' }}
          >
            Today's Journal
          </Button>
          <Button
            size="small"
            startIcon={<RefreshIcon />}
            onClick={loadBlocks}
            sx={{ textTransform: 'none', fontSize: '0.75rem' }}
          >
            Refresh
          </Button>
        </Box>
      </Box>

      {/* Error Banner */}
      {error && (
        <Alert severity="error" onClose={() => setError(null)} sx={{ borderRadius: 0 }}>
          {error}
        </Alert>
      )}

      {/* Board */}
      <Box sx={{ flex: 1, overflow: 'auto', p: 2 }}>
        <DndContext
          sensors={sensors}
          collisionDetection={closestCorners}
          onDragStart={handleDragStart}
          onDragEnd={handleDragEnd}
        >
          <Box
            sx={{
              display: 'flex',
              gap: 2,
              alignItems: 'flex-start',
              minHeight: '100%',
            }}
          >
            {COLUMNS.map((col) => (
              <KanbanColumn
                key={col}
                columnId={col}
                blocks={grouped[col]}
                isDone={col === 'done'}
                collapsed={col === 'done' && doneCollapsed}
                onToggleCollapse={() => setDoneCollapsed((c) => !c)}
                newCardActive={newCardColumn === col}
                newCardText={newCardText}
                onNewCardTextChange={setNewCardText}
                onAddCard={() => handleAddCard(col)}
                onStartNewCard={() => setNewCardColumn(col)}
                onCancelNewCard={() => {
                  setNewCardColumn(null);
                  setNewCardText('');
                }}
                onCardContextMenu={handleContextMenu}
                onCardClick={(block) => setEditingBlock(block)}
              />
            ))}
          </Box>

          <DragOverlay>
            {activeId ? (
              <Card sx={{ opacity: 0.85, p: 2, boxShadow: 4 }}>
                <Typography variant="body2">
                  {blocks.find((b) => b.id === activeId)?.content ?? ''}
                </Typography>
              </Card>
            ) : null}
          </DragOverlay>
        </DndContext>
      </Box>

      {/* Edit Dialog */}
      {editingBlock && (
        <KanbanEditDialog
          block={editingBlock}
          onSave={handleEditSave}
          onCancel={() => setEditingBlock(null)}
        />
      )}

      {/* Context Menu */}
      <Menu
        open={contextMenu !== null}
        onClose={() => setContextMenu(null)}
        anchorReference="anchorPosition"
        anchorPosition={contextMenu ? { left: contextMenu.x, top: contextMenu.y } : undefined}
      >
        <MenuItem onClick={() => contextMenu && handleEditFromContextMenu(contextMenu.block)} dense>
          Edit
        </MenuItem>
        <MenuItem
          onClick={() => contextMenu && handleDeleteBlock(contextMenu.block)}
          dense
          sx={{ color: 'error.main' }}
        >
          Delete
        </MenuItem>
      </Menu>
    </Box>
  );
}
