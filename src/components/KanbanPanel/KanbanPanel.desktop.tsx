import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import Card from '@mui/material/Card';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
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
import KanbanEditDialog from '../KanbanEditDialog';
import KanbanColumn from './KanbanColumn';
import { COLUMNS } from './constants';
import { useKanbanPanel } from './KanbanPanel.shared';

export default function KanbanPanelDesktop() {
  const {
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
    setActiveId,
    setNewCardColumn,
    setNewCardText,
    setDoneCollapsed,
    setEditingBlock,
    setContextMenu,
    setError,
    loadBlocks,
    handleDragEnd: sharedHandleDragEnd,
    handleAddCard,
    handleEditSave,
    handleDeleteBlock,
    handleEditFromContextMenu,
    handleContextMenu,
    openTodaysJournal,
  } = useKanbanPanel();

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
  );

  function handleDragStart(event: DragStartEvent) {
    setActiveId(String(event.active.id));
  }

  async function onDragEnd(event: DragEndEvent) {
    setActiveId(null);
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    // Reuse the shared handler with plain objects
    await sharedHandleDragEnd({
      active: { id: String(active.id) },
      over: over ? { id: String(over.id) } : null,
    });
  }

  // -----------------------------------------------------------------------
  // Render
  // -----------------------------------------------------------------------

  if (loading) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <Box
          component="span"
          sx={{ width: 24, height: 24, borderRadius: '50%', border: 2, borderColor: 'primary.main', borderTopColor: 'transparent', animation: 'spin 0.6s linear infinite' }}
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

      {/* Board */}
      <Box sx={{ flex: 1, overflow: 'auto', p: 2 }}>
        <DndContext
          sensors={sensors}
          collisionDetection={closestCorners}
          onDragStart={handleDragStart}
          onDragEnd={onDragEnd}
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
