import { useState, memo } from 'react';
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
import ArticleIcon from '@mui/icons-material/Article';
import DeleteIcon from '@mui/icons-material/Delete';
import DriveFileMoveIcon from '@mui/icons-material/DriveFileMove';
import type { PageDto } from '../../lib/types';
import ConfirmDialog from '../ui/ConfirmDialog';
import MovePageDialog from './MovePageDialog';
import { useTranslation } from 'react-i18next';

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
  onMovePage: (oldPath: string, newPath: string) => Promise<boolean>;
  onNavigate: (path: string) => void;
  onNavigateHome?: () => void;
}

const PageTree = memo(function PageTree({
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
  onMovePage,
  onNavigate,
  onNavigateHome,
}: Props) {
  const { t } = useTranslation();
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [moveTarget, setMoveTarget] = useState<PageDto | null>(null);

  if (collapsed) return null;

  const recentPages = pages.filter(p => !p.path.startsWith('journals/'));

  return (
    <>
      <Box sx={{ px: 1, pt: 1 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', px: 1, py: 0.5 }}>
          <Box
            component="span"
            onClick={onNavigateHome}
            sx={{ display: 'flex', alignItems: 'center', gap: 0.5, cursor: onNavigateHome ? 'pointer' : undefined, '&:hover': onNavigateHome ? { color: 'primary.main' } : undefined }}
          >
            <ArticleIcon fontSize="inherit" sx={{ fontSize: '0.8rem', color: 'text.secondary' }} />
            <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary', textTransform: 'uppercase' }}>
              {t('sidebar.recent')}
            </Typography>
          </Box>
          <Tooltip title={t('sidebar.new_page')} arrow>
            <IconButton size="small" onClick={() => onShowNewChange(!showNew)} sx={{ color: 'text.secondary' }}>
              <AddIcon fontSize="small" />
            </IconButton>
          </Tooltip>
        </Box>

        {showNew && (
          <Box sx={{ p: 1, mb: 1, bgcolor: 'action.hover', borderRadius: 1 }}>
            <TextField
              size="small"
              placeholder={t('sidebar.path_e_g_pages_my_note_md')}
              value={newPath}
              onChange={e => onNewPathChange(e.target.value)}
              fullWidth
              sx={{ mb: 0.5, '& .MuiInputBase-input': { fontSize: '0.75rem', py: 0.75 } }}
            />
            <TextField
              size="small"
              placeholder={t('sidebar.title_optional')}
              value={newTitle}
              onChange={e => onNewTitleChange(e.target.value)}
              fullWidth
              sx={{ mb: 0.75, '& .MuiInputBase-input': { fontSize: '0.75rem', py: 0.75 } }}
            />
            <Box sx={{ display: 'flex', gap: 0.5 }}>
              <Button size="small" variant="contained" onClick={onCreatePage}>
                {t('sidebar.create')}
              </Button>
              <Button size="small" onClick={() => onShowNewChange(false)}>
                {t('sidebar.cancel')}
              </Button>
            </Box>
          </Box>
        )}

        <List dense disablePadding>
          {recentPages.map(page => (
            <ListItemButton
              key={page.path}
              dense
              onClick={() => onNavigate(`/page/${encodeURIComponent(page.path)}`)}
              sx={{ borderRadius: 1, '&:hover .delete-btn': { opacity: 1 }, '&:hover .move-btn': { opacity: 1 } }}
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
                className="move-btn"
                sx={{
                  opacity: 0,
                  color: 'text.secondary',
                  ml: 0.5,
                  p: 0.25,
                  '&:hover': { bgcolor: 'action.hover', color: 'primary.main' },
                }}
                onClick={e => {
                  e.stopPropagation();
                  setMoveTarget(page);
                }}
                aria-label={t('sidebar.move_rename')}
                title={t('sidebar.move_rename')}
              >
                <DriveFileMoveIcon fontSize="inherit" />
              </IconButton>
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
                  setDeleteTarget(page.path);
                }}
              >
                <DeleteIcon fontSize="inherit" />
              </IconButton>
            </ListItemButton>
          ))}
          {recentPages.length === 0 && (
            <Typography variant="caption" color="text.disabled" sx={{ display: 'block', textAlign: 'center', py: 2 }}>
              {t('pages_home.no_pages_yet')}
            </Typography>
          )}
        </List>
      </Box>

      <ConfirmDialog
        open={deleteTarget !== null}
        title={t('sidebar.delete_page')}
        message={`${t('editor.delete')} ${deleteTarget ?? ''}? ${t('common.this_cannot_be_undone')}`}
        confirmLabel={t('editor.delete')}
        confirmColor="error"
        onConfirm={() => {
          if (deleteTarget) onDeletePage(deleteTarget);
          setDeleteTarget(null);
        }}
        onCancel={() => setDeleteTarget(null)}
      />

      <MovePageDialog
        open={moveTarget !== null}
        page={moveTarget}
        pages={pages}
        onClose={() => setMoveTarget(null)}
        onSubmit={onMovePage}
      />
    </>
  );
});

export default PageTree;
