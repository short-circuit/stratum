import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { useDroppable } from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';
import KanbanCard from './KanbanCard';
import { COLUMN_CONFIG, type ColumnId } from './constants';

// ---------------------------------------------------------------------------
// Section — a single marker group rendered as a vertical list (droppable).
// Extracted from KanbanPanel.mobile.tsx during the E6 sizing-gate refactor.
// ---------------------------------------------------------------------------

export default function Section({
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
