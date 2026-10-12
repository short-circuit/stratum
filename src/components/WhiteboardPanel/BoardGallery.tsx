import { useState } from 'react';
import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import CardActionArea from '@mui/material/CardActionArea';
import Checkbox from '@mui/material/Checkbox';
import Grid from '@mui/material/Grid';
import DrawIcon from '@mui/icons-material/Draw';
import DeleteIcon from '@mui/icons-material/Delete';
import Chip from '@mui/material/Chip';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogContentText from '@mui/material/DialogContentText';
import DialogActions from '@mui/material/DialogActions';
import { useTranslation } from 'react-i18next';

export interface Board {
  name: string;
  path: string;
  content: string;
}

interface BoardWithMeta extends Board {
  elementCount: number;
  preview: string | null;
  previewTheme: 'dark' | 'light' | null;
}

interface BoardGalleryProps {
  boards: Board[];
  onLoadBoard: (name: string) => void;
  onCreateBoard: (name: string) => void;
  onDeleteBoards: (names: string | string[]) => Promise<void>;
  onRename: (oldName: string, newName: string) => Promise<void>;
}

function parseBoardMeta(content: string) {
  let elementCount = 0;
  let preview: string | null = null;
  let previewTheme: 'dark' | 'light' | null = null;
  try {
    const parsed = JSON.parse(content);
    if (Array.isArray(parsed.elements)) {
      elementCount = parsed.elements.length;
    }
    if (typeof parsed.preview === 'string') {
      preview = parsed.preview;
    }
    if (parsed.previewTheme === 'dark' || parsed.previewTheme === 'light') {
      previewTheme = parsed.previewTheme;
    }
  } catch { /* ignore */ }
  return { elementCount, preview, previewTheme };
}

export default function BoardGallery({
  boards,
  onLoadBoard,
  onCreateBoard,
  onDeleteBoards,
  onRename,
}: BoardGalleryProps) {
  const { t } = useTranslation();
  const [newName, setNewName] = useState('');
  const [selectedBoards, setSelectedBoards] = useState<Set<string>>(new Set());
  const [confirmDelete, setConfirmDelete] = useState<string | string[] | null>(null);
  const [renameTarget, setRenameTarget] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');

  const boardsWithMeta: BoardWithMeta[] = boards.map(b => {
    const { elementCount, preview, previewTheme } = parseBoardMeta(b.content);
    return { ...b, elementCount, preview, previewTheme };
  });

  function toggleSelectBoard(name: string, e: React.MouseEvent) {
    e.stopPropagation();
    setSelectedBoards(prev => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  }

  async function handleDeleteBoards(names: string | string[]) {
    await onDeleteBoards(names);
    setConfirmDelete(null);
    setSelectedBoards(new Set());
  }

  async function handleRename(oldName: string, newName: string) {
    await onRename(oldName, newName);
    setRenameTarget(null);
  }

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      <Box sx={{ flex: 1, overflow: 'auto', p: 3 }}>
      <Typography variant="h5" sx={{ fontWeight: 600, mb: 2 }}>{t('whiteboard.whiteboards')}</Typography>

      <Box sx={{ display: 'flex', gap: 1, mb: 3, alignItems: 'center' }}>
        <TextField
          size="small"
          placeholder={t('whiteboard.whiteboard_name')}
          value={newName}
          onChange={e => setNewName(e.target.value)}
          onKeyDown={e => { if (e.key === 'Enter') onCreateBoard(newName); }}
          sx={{ flexGrow: 1 }}
        />
        <Button variant="contained" onClick={() => onCreateBoard(newName)} sx={{ whiteSpace: 'nowrap' }}>
          {t('whiteboard.create')}
        </Button>
        {selectedBoards.size > 0 && (
          <>
            <Typography variant="body2" color="text.secondary" sx={{ mx: 0.5 }}>
              {t('whiteboard.selected_count', { count: selectedBoards.size })}
            </Typography>
            <Button
              variant="outlined"
              color="error"
              size="small"
              startIcon={<DeleteIcon />}
              onClick={() => setConfirmDelete(Array.from(selectedBoards))}
            >
              {t('whiteboard.delete')}
            </Button>
          </>
        )}
      </Box>

      <Grid container spacing={2}>
        {boardsWithMeta.map(b => {
          const isSelected = selectedBoards.has(b.name);
          return (
          <Grid key={b.name} size={{ xs: 12, sm: 6, md: 4 }}>
            <Card
              variant="outlined"
              sx={{
                transition: 'box-shadow 0.2s, border-color 0.2s',
                '&:hover': { borderColor: 'primary.light', boxShadow: 2 },
                outline: isSelected ? '2px solid' : 'none',
                outlineColor: isSelected ? 'primary.main' : 'transparent',
              }}
            >
              <CardActionArea onClick={() => onLoadBoard(b.name)} sx={{ height: '100%' }}>
                <Box sx={{
                  width: '100%',
                  height: 140,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  bgcolor: b.previewTheme === 'dark' ? '#232329' : '#ffffff',
                  overflow: 'hidden',
                  position: 'relative',
                }}>
                  {b.preview ? (
                    <Box
                      component="img"
                      src={b.preview}
                      alt={b.name}
                      sx={{
                        width: '100%',
                        height: '100%',
                        objectFit: 'contain',
                      }}
                    />
                  ) : (
                    <DrawIcon sx={{ fontSize: 48, color: 'text.disabled' }} />
                  )}
                  <Checkbox
                    size="small"
                    checked={isSelected}
                    onClick={(e) => toggleSelectBoard(b.name, e)}
                    sx={{ position: 'absolute', top: 4, left: 4, bgcolor: 'rgba(255,255,255,0.85)', borderRadius: 0.5 }}
                  />
                </Box>
                <CardContent sx={{ py: 1.5, px: 2 }}>
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
                    <Typography variant="subtitle2" noWrap sx={{ flexGrow: 1 }}>{b.name}</Typography>
                    <IconButton
                      size="small"
                      onClick={(e) => { e.stopPropagation(); setRenameTarget(b.name); setRenameValue(b.name); }}
                      sx={{ color: 'text.secondary', p: 0.25 }}
                    >
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/>
                      </svg>
                    </IconButton>
                    <IconButton
                      size="small"
                      onClick={(e) => { e.stopPropagation(); setConfirmDelete(b.name); }}
                      sx={{ color: 'error.main', p: 0.25 }}
                    >
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/>
                      </svg>
                    </IconButton>
                  </Box>
                  {b.elementCount > 0 && (
                    <Chip label={t('whiteboard.element_count', { count: b.elementCount })} size="small" variant="outlined" sx={{ mt: 0.5 }} />
                  )}
                </CardContent>
              </CardActionArea>
            </Card>
          </Grid>
          );
        })}
        {boardsWithMeta.length === 0 && (
          <Grid size={12}>
            <Typography variant="body2" color="text.secondary" align="center" sx={{ py: 4 }}>
              {t('whiteboard.no_whiteboards_yet_create_one_to_get_started')}
            </Typography>
          </Grid>
        )}
      </Grid>

      <Dialog open={confirmDelete !== null} onClose={() => setConfirmDelete(null)}>
        <DialogTitle>{t('whiteboard.delete_whiteboard')}{Array.isArray(confirmDelete) && confirmDelete.length > 1 ? 's' : ''}?</DialogTitle>
        <DialogContent>
          <DialogContentText>
            {Array.isArray(confirmDelete)
              ? `${t('whiteboard.delete')} ${confirmDelete.length} ${t('whiteboard.whiteboards')}? ${t('whiteboard.this_cannot_be_undone')}`
              : `${t('page.delete')}"${confirmDelete}"? ${t('whiteboard.this_cannot_be_undone')}`}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmDelete(null)}>{t('whiteboard.cancel')}</Button>
          <Button
            variant="contained"
            color="error"
            onClick={() => confirmDelete && handleDeleteBoards(confirmDelete)}
            autoFocus
          >
            {t('whiteboard.delete')}
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={renameTarget !== null} onClose={() => setRenameTarget(null)} maxWidth="xs" fullWidth>
        <DialogTitle>{t('whiteboard.rename_whiteboard')}</DialogTitle>
        <DialogContent>
          <TextField
            autoFocus
            size="small"
            fullWidth
            value={renameValue}
            onChange={e => setRenameValue(e.target.value)}
            onKeyDown={e => { if (e.key === 'Enter' && renameTarget) handleRename(renameTarget, renameValue); }}
            sx={{ mt: 1 }}
          />
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setRenameTarget(null)}>{t('whiteboard.cancel')}</Button>
          <Button
            variant="contained"
            onClick={() => renameTarget && handleRename(renameTarget, renameValue)}
          >
            {t('whiteboard.rename')}
          </Button>
        </DialogActions>
      </Dialog>
      </Box>
    </Box>
  );
}
