import Box from '@mui/material/Box';
import Button from '@mui/material/Button';

// ---------------------------------------------------------------------------
// CardContextMenu — mobile bottom-sheet context menu for a kanban card.
// Extracted from KanbanPanel.mobile.tsx during the E6 sizing-gate refactor.
// ---------------------------------------------------------------------------

export default function CardContextMenu({
  onEdit,
  onDelete,
  onClose,
}: {
  onEdit: () => void;
  onDelete: () => void;
  onClose: () => void;
}) {
  return (
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
        onClick={onEdit}
      >
        Edit
      </Button>
      <Button
        fullWidth
        sx={{ textTransform: 'none', py: 1.5, color: 'error.main' }}
        onClick={onDelete}
      >
        Delete
      </Button>
      <Button
        fullWidth
        sx={{ textTransform: 'none', py: 1 }}
        onClick={onClose}
      >
        Cancel
      </Button>
    </Box>
  );
}
