import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import List from '@mui/material/List';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemText from '@mui/material/ListItemText';
import IconButton from '@mui/material/IconButton';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';
import Tooltip from '@mui/material/Tooltip';
import AddIcon from '@mui/icons-material/Add';
import DeleteIcon from '@mui/icons-material/Delete';
import type { PageDto } from '../../lib/types';

interface Props {
  pages: PageDto[];
  collapsed: boolean;
  showNew: boolean;
  newPath: string;
  newTitle: string;
  onShowNewChange: (v: boolean) => void;
  onNewPathChange: (v: string) => void;
  onNewTitleChange: (v: string) => void;
  onCreatePage: () => Promise<void>;
  onDeletePage: (path: string) => void;
  onNavigate: (path: string) => void;
  onNavigateHome?: () => void;
}

export default function PageTree({
  pages,
  collapsed,
  showNew,
  newPath,
  newTitle,
  onShowNewChange,
  onNewPathChange,
  onNewTitleChange,
  onCreatePage,
  onDeletePage,
  onNavigate,
}: Props) {
  if (collapsed) return null;

  return (
    <>
      <Box sx={{ px: 1 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', px: 1, py: 0.5 }}>
          <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary', textTransform: 'uppercase' }}>
            Recent
          </Typography>
          <Tooltip title="New page" arrow>
            <IconButton size="small" onClick={() => onShowNewChange(!showNew)} sx={{ color: 'text.secondary' }}>
              <AddIcon fontSize="small" />
            </IconButton>
          </Tooltip>
        </Box>

        {showNew && (
          <Box sx={{ p: 1, mb: 1, bgcolor: 'action.hover', borderRadius: 1 }}>
            <TextField
              size="small"
              placeholder="Path (e.g., pages/my-note.md)"
              value={newPath}
              onChange={e => onNewPathChange(e.target.value)}
              fullWidth
              sx={{ mb: 0.5, '& .MuiInputBase-input': { fontSize: '0.75rem', py: 0.75 } }}
            />
            <TextField
              size="small"
              placeholder="Title (optional)"
              value={newTitle}
              onChange={e => onNewTitleChange(e.target.value)}
              fullWidth
              sx={{ mb: 0.75, '& .MuiInputBase-input': { fontSize: '0.75rem', py: 0.75 } }}
            />
            <Box sx={{ display: 'flex', gap: 0.5 }}>
              <Button size="small" variant="contained" onClick={onCreatePage}>
                Create
              </Button>
              <Button size="small" onClick={() => onShowNewChange(false)}>
                Cancel
              </Button>
            </Box>
          </Box>
        )}

        <List dense disablePadding>
          {pages.map(page => (
            <ListItemButton
              key={page.path}
              dense
              onClick={() => onNavigate(`/page/${encodeURIComponent(page.path)}`)}
              sx={{ borderRadius: 1, '&:hover .delete-btn': { opacity: 1 } }}
            >
              <ListItemText
                primary={page.title || page.slug}
                slotProps={{
                  primary: { variant: 'body2', noWrap: true },
                  secondary: { variant: 'caption', color: 'text.disabled' as const },
                }}
                secondary={page.block_count ? `${page.block_count}b` : undefined}
              />
              <IconButton
                size="small"
                className="delete-btn"
                sx={{
                  opacity: 0,
                  color: 'error.main',
                  ml: 0.5,
                  p: 0.25,
                  '&:hover': { bgcolor: 'error.light', color: 'error.contrastText' },
                }}
                onClick={e => {
                  e.stopPropagation();
                  if (confirm(`Delete ${page.path}?`)) onDeletePage(page.path);
                }}
              >
                <DeleteIcon fontSize="inherit" />
              </IconButton>
            </ListItemButton>
          ))}
          {pages.length === 0 && (
            <Typography variant="caption" color="text.disabled" sx={{ display: 'block', textAlign: 'center', py: 2 }}>
              No pages yet.
            </Typography>
          )}
        </List>
      </Box>
    </>
  );
}
