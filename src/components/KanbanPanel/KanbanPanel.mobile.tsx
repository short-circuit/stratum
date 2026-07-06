import { useState, useRef } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';
import Card from '@mui/material/Card';

import Alert from '@mui/material/Alert';
import RefreshIcon from '@mui/icons-material/Refresh';
import AddIcon from '@mui/icons-material/Add';
import {
  DndContext,
  DragOverlay,
  closestCorners,
  PointerSensor,
  TouchSensor,
  useSensor,
  useSensors,
  useDroppable,
  type DragStartEvent,
  type DragEndEvent,
} from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';
import KanbanEditDialog from '../KanbanEditDialog';
import KanbanCard from './KanbanCard';
import { COLUMNS, COLUMN_CONFIG, type ColumnId } from './constants';
import { useKanbanPanel } from './KanbanPanel.shared';

// ---------------------------------------------------------------------------
// Section — a single marker group rendered as a vertical list (droppable)
// ---------------------------------------------------------------------------

function Section({
  columnId,
  blocks,
  onCardClick,
  onCardContextMenu,
}: {
  columnId: ColumnId;
  blocks: { id: string }[];
  onCardClick?: (block: any) => void;
  onCardContextMenu?: (block: any, e: React.MouseEvent) => void;
}) {
  const config = COLUMN_CONFIG[columnId];
  const { setNodeRef, isOver } = useDroppable({ id: columnId });

  return (
    <Box sx={{ mb: 1.5 }}>
      {/* Section header */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 1,
          px: 2,
          py: 1,
          bgcolor: isOver ? 'action.selected' : 'transparent',
          borderRadius: 1,
        }}
      >
        <Box
          sx={{
            width: 10,
            height: 10,
            borderRadius: '50%',
            bgcolor: config.color,
            flexShrink: 0,
          }}
        />
        <Typography variant="subtitle2" sx={{ fontWeight: 700 }}>
          {config.label}
        </Typography>
        <Typography variant="caption" color="text.secondary">
          {blocks.length}
        </Typography>
      </Box>

      {/* Cards */}
      <Box
        ref={setNodeRef}
        sx={{
          px: 2,
          display: 'flex',
          flexDirection: 'column',
          gap: 0.5,
          minHeight: 48,
          transition: 'background-color 0.15s ease',
          bgcolor: isOver ? 'action.hover' : 'transparent',
          borderRadius: 1,
          py: 0.5,
        }}
      >
        <SortableContext items={blocks.map((b) => b.id)} strategy={verticalListSortingStrategy}>
          {blocks.map((block: any) => (
            <KanbanCard
              key={block.id}
              block={block}
              onClick={onCardClick}
              onContextMenu={onCardContextMenu}
            />
          ))}
        </SortableContext>

        {blocks.length === 0 && (
          <Typography
            variant="caption"
            color="text.disabled"
            sx={{ textAlign: 'center', py: 2 }}
          >
            No tasks
          </Typography>
        )}
      </Box>
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Mobile KanbanPanel
// ---------------------------------------------------------------------------

export default function KanbanPanelMobile() {
  const {
    blocks,
    loading,
    error,
    activeId,
    editingBlock,
    grouped,
    setActiveId,
    setEditingBlock,
    setError,
    loadBlocks,
    handleDragEnd: sharedHandleDragEnd,
    handleEditSave,
    handleDeleteBlock,
    handleEditFromContextMenu,
    handleContextMenu,
    openTodaysJournal,
  } = useKanbanPanel();

  // Local state for the add-card form
  const [addTarget, setAddTarget] = useState<ColumnId>('todo');
  const [addText, setAddText] = useState('');

  // Local context menu state (mirrored from shared, but positioned differently on mobile)
  const [contextMenuBlock, setContextMenuBlock] = useState<any | null>(null);
  const contextMenuAnchorRef = useRef<HTMLElement | null>(null);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(TouchSensor, { activationConstraint: { delay: 250, tolerance: 5 } }),
  );

  function handleDragStart(event: DragStartEvent) {
    setActiveId(String(event.active.id));
  }

  async function onDragEnd(event: DragEndEvent) {
    setActiveId(null);
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    await sharedHandleDragEnd({
      active: { id: String(active.id) },
      over: over ? { id: String(over.id) } : null,
    });
  }

  async function handleLocalAddCard() {
    if (!addText.trim()) return;
    const marker = COLUMN_CONFIG[addTarget].markers[0];
    try {
      const { createKanbanBlock } = await import('../../lib/commands');
      await createKanbanBlock(addText.trim(), marker);
      setAddText('');
      await loadBlocks();
    } catch (e) {
      setError(String(e));
    }
  }

  function handleMobileContextMenu(block: any, e: React.MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    setContextMenuBlock(block);
    contextMenuAnchorRef.current = e.currentTarget as HTMLElement;
    // Also fire shared handler for state consistency
    handleContextMenu(block, e);
  }

  // -----------------------------------------------------------------------
  // Render
  // -----------------------------------------------------------------------

  if (loading) {
    return (
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: '100%',
        }}
      >
        <Box
          component="span"
          sx={{
            width: 24,
            height: 24,
            borderRadius: '50%',
            border: 2,
            borderColor: 'primary.main',
            borderTopColor: 'transparent',
            animation: 'spin 0.6s linear infinite',
          }}
        />
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
            Today&apos;s Journal
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

      {/* Add Card Form */}
      <Box
        sx={{
          px: 2,
          py: 1.5,
          borderBottom: 1,
          borderColor: 'divider',
          display: 'flex',
          gap: 1,
          alignItems: 'flex-end',
        }}
      >
        <Box sx={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 0.5 }}>
          <TextField
            size="small"
            placeholder="New task..."
            value={addText}
            onChange={(e) => setAddText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                handleLocalAddCard();
              }
            }}
            fullWidth
            multiline
            maxRows={2}
            sx={{ '& .MuiInputBase-input': { fontSize: '0.85rem', py: 0.75 } }}
          />
          <Box sx={{ display: 'flex', gap: 0.5 }}>
            {COLUMNS.map((col) => (
              <Button
                key={col}
                size="small"
                variant={addTarget === col ? 'contained' : 'outlined'}
                onClick={() => setAddTarget(col)}
                sx={{
                  textTransform: 'none',
                  fontSize: '0.65rem',
                  py: 0.25,
                  px: 1,
                  minWidth: 0,
                  borderColor: addTarget === col ? COLUMN_CONFIG[col].color : undefined,
                  bgcolor: addTarget === col ? COLUMN_CONFIG[col].color : undefined,
                  '&:hover': {
                    bgcolor: addTarget === col ? COLUMN_CONFIG[col].color : undefined,
                  },
                }}
              >
                {COLUMN_CONFIG[col].label}
              </Button>
            ))}
          </Box>
        </Box>
        <Button
          size="small"
          variant="contained"
          onClick={handleLocalAddCard}
          disabled={!addText.trim()}
          sx={{ minWidth: 48, height: 36 }}
        >
          <AddIcon fontSize="small" />
        </Button>
      </Box>

      {/* Board — list view */}
      <Box sx={{ flex: 1, overflow: 'auto', py: 1.5 }}>
        <DndContext
          sensors={sensors}
          collisionDetection={closestCorners}
          onDragStart={handleDragStart}
          onDragEnd={onDragEnd}
        >
          {COLUMNS.map((col) => (
            <Section
              key={col}
              columnId={col}
              blocks={grouped[col]}
              onCardClick={(block) => setEditingBlock(block)}
              onCardContextMenu={handleMobileContextMenu}
            />
          ))}

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
      {contextMenuBlock && (
        <Box
          sx={{
            position: 'fixed',
            bottom: 0,
            left: 0,
            right: 0,
            bgcolor: 'background.paper',
            borderTop: 1,
            borderColor: 'divider',
            zIndex: 1300,
            display: 'flex',
            flexDirection: 'column',
          }}
        >
          <Button
            fullWidth
            sx={{ textTransform: 'none', py: 1.5, borderBottom: 1, borderColor: 'divider' }}
            onClick={() => {
              handleEditFromContextMenu(contextMenuBlock);
              setContextMenuBlock(null);
            }}
          >
            Edit
          </Button>
          <Button
            fullWidth
            sx={{ textTransform: 'none', py: 1.5, color: 'error.main' }}
            onClick={() => {
              handleDeleteBlock(contextMenuBlock);
              setContextMenuBlock(null);
            }}
          >
            Delete
          </Button>
          <Button
            fullWidth
            sx={{ textTransform: 'none', py: 1 }}
            onClick={() => setContextMenuBlock(null)}
          >
            Cancel
          </Button>
        </Box>
      )}
    </Box>
  );
}
