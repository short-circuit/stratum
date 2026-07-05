import Box from '@mui/material/Box';
import Paper from '@mui/material/Paper';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import TextField from '@mui/material/TextField';
import Chip from '@mui/material/Chip';
import Collapse from '@mui/material/Collapse';
import AddIcon from '@mui/icons-material/Add';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import ExpandLessIcon from '@mui/icons-material/ExpandLess';
import { useDroppable } from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';
import KanbanCard from './KanbanCard';
import { COLUMN_CONFIG, type ColumnId } from './constants';
import type { KanbanBlockDto } from '../../lib/types';

interface KanbanColumnProps {
  columnId: ColumnId;
  blocks: KanbanBlockDto[];
  isDone: boolean;
  collapsed: boolean;
  onToggleCollapse: () => void;
  newCardActive: boolean;
  newCardText: string;
  onNewCardTextChange: (text: string) => void;
  onAddCard: () => void;
  onStartNewCard: () => void;
  onCancelNewCard: () => void;
  onCardContextMenu?: (block: KanbanBlockDto, e: React.MouseEvent) => void;
  onCardClick?: (block: KanbanBlockDto) => void;
}

export default function KanbanColumn({
  columnId,
  blocks,
  isDone,
  collapsed,
  onToggleCollapse,
  newCardActive,
  newCardText,
  onNewCardTextChange,
  onAddCard,
  onStartNewCard,
  onCancelNewCard,
  onCardContextMenu,
  onCardClick,
}: KanbanColumnProps) {
  const config = COLUMN_CONFIG[columnId];
  const { setNodeRef } = useDroppable({ id: columnId });

  return (
    <Paper
      sx={{
        minWidth: 280,
        maxWidth: 340,
        flex: '1 1 0',
        display: 'flex',
        flexDirection: 'column',
        maxHeight: '100%',
        bgcolor: 'action.hover',
        overflow: 'hidden',
      }}
    >
      {/* Column Header */}
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
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          {isDone && (
            <IconButton size="small" onClick={onToggleCollapse} sx={{ p: 0 }}>
              {collapsed ? <ExpandLessIcon fontSize="small" /> : <ExpandMoreIcon fontSize="small" />}
            </IconButton>
          )}
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <Box
              sx={{
                width: 8,
                height: 8,
                borderRadius: '50%',
                bgcolor: config.color,
              }}
            />
            <Typography variant="subtitle2" sx={{ fontWeight: 700 }}>
              {config.label}
            </Typography>
          </Box>
          <Chip
            label={blocks.length}
            size="small"
            sx={{ height: 20, minWidth: 20, fontSize: '0.7rem' }}
          />
        </Box>
      </Box>

      {/* Column Body */}
      <Collapse in={!collapsed} timeout="auto">
        <Box
          ref={setNodeRef}
          sx={{
            flex: 1,
            overflowY: 'auto',
            p: 1,
            display: 'flex',
            flexDirection: 'column',
            gap: 0.5,
            minHeight: 80,
          }}
        >
          <SortableContext items={blocks.map((b) => b.id)} strategy={verticalListSortingStrategy}>
            {blocks.map((block) => (
              <KanbanCard key={block.id} block={block} onContextMenu={onCardContextMenu} onClick={onCardClick} />
            ))}
          </SortableContext>

          {blocks.length === 0 && !newCardActive && (
            <Typography variant="caption" color="text.disabled" sx={{ textAlign: 'center', py: 2 }}>
              No tasks yet
            </Typography>
          )}

          {/* Inline Add Card Form */}
          {newCardActive ? (
            <Box sx={{ mt: 1 }}>
              <TextField
                size="small"
                placeholder="Task description..."
                value={newCardText}
                onChange={(e) => onNewCardTextChange(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !e.shiftKey) {
                    e.preventDefault();
                    onAddCard();
                  }
                  if (e.key === 'Escape') onCancelNewCard();
                }}
                fullWidth
                autoFocus
                multiline
                maxRows={3}
                sx={{ '& .MuiInputBase-input': { fontSize: '0.8rem', py: 0.75 } }}
              />
              <Box sx={{ display: 'flex', gap: 0.5, mt: 0.5 }}>
                <Button
                  size="small"
                  variant="contained"
                  onClick={onAddCard}
                  disabled={!newCardText.trim()}
                >
                  Add
                </Button>
                <Button size="small" onClick={onCancelNewCard}>
                  Cancel
                </Button>
              </Box>
            </Box>
          ) : (
            <Button
              size="small"
              startIcon={<AddIcon />}
              onClick={onStartNewCard}
              sx={{ mt: 0.5, textTransform: 'none', fontSize: '0.75rem', alignSelf: 'flex-start' }}
            >
              Add Card
            </Button>
          )}
        </Box>
      </Collapse>
    </Paper>
  );
}
