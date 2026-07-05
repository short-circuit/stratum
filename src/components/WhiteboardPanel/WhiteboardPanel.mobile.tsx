import { useState, useMemo } from 'react';
import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import CardActionArea from '@mui/material/CardActionArea';
import SwipeableDrawer from '@mui/material/SwipeableDrawer';
import Chip from '@mui/material/Chip';
import DrawIcon from '@mui/icons-material/Draw';
import AddIcon from '@mui/icons-material/Add';
import DeleteIcon from '@mui/icons-material/Delete';
import EditIcon from '@mui/icons-material/Edit';
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
import { Excalidraw, MainMenu } from '@excalidraw/excalidraw';
import '@excalidraw/excalidraw/index.css';
import { useWhiteboardPanel } from './WhiteboardPanel.shared';

function parseBoardPreview(content: string) {
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

export default function WhiteboardPanelMobile() {
  const {
    boards,
    activeBoard,
    sceneData,
    libraryItems,
    excalidrawRef,
    dirty,
    loadBoard,
    createBoard,
    deleteBoards,
    handleRename,
    navigateBack,
    handleChange,
    handleLibraryChange,
  } = useWhiteboardPanel();

  const [createOpen, setCreateOpen] = useState(false);
  const [newName, setNewName] = useState('');
  const [renameTarget, setRenameTarget] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);

  const boardsWithMeta = useMemo(() =>
    boards.map(b => {
      const { elementCount, preview, previewTheme } = parseBoardPreview(b.content);
      return { ...b, elementCount, preview, previewTheme };
    }),
    [boards],
  );

  function handleCreateSubmit() {
    if (!newName.trim()) return;
    createBoard(newName);
    setNewName('');
    setCreateOpen(false);
  }

  function handleRenameSubmit() {
    if (!renameTarget || !renameValue.trim()) return;
    handleRename(renameTarget, renameValue);
    setRenameTarget(null);
    setRenameValue('');
  }

  function handleDeleteConfirm() {
    if (!deleteTarget) return;
    deleteBoards(deleteTarget);
    setDeleteTarget(null);
  }

  if (activeBoard && sceneData && libraryItems) {
    return (
      <Box sx={{ height: '100dvh', display: 'flex', flexDirection: 'column', bgcolor: 'background.default' }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5, px: 1, py: 0.5, borderBottom: 1, borderColor: 'divider', bgcolor: 'background.default' }}>
          <IconButton size="small" onClick={navigateBack}>
            <ArrowBackIcon fontSize="small" />
          </IconButton>
          <Typography variant="body2" noWrap sx={{ fontWeight: 500, flexGrow: 1 }}>{activeBoard}</Typography>
          {dirty && (
            <Chip label="Unsaved" size="small" color="warning" variant="outlined" sx={{ fontSize: '0.6rem', height: 18 }} />
          )}
        </Box>
        <Box sx={{ flex: 1, position: 'relative', minHeight: 0 }}>
          <Excalidraw
            key={activeBoard}
            theme={document.documentElement.classList.contains('dark') ? 'dark' : 'light'}
            initialData={{ ...sceneData, libraryItems }}
            excalidrawAPI={(api) => { excalidrawRef.current = api; }}
            onChange={handleChange}
            onLibraryChange={handleLibraryChange}
          >
            <MainMenu>
              <MainMenu.DefaultItems.Help />
              <MainMenu.DefaultItems.ClearCanvas />
              <MainMenu.DefaultItems.ToggleTheme />
              <MainMenu.DefaultItems.ChangeCanvasBackground />
              <MainMenu.DefaultItems.Export />
              <MainMenu.DefaultItems.SaveAsImage />
              <MainMenu.DefaultItems.SearchMenu />
            </MainMenu>
          </Excalidraw>
        </Box>
      </Box>
    );
  }

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column', bgcolor: 'background.default' }}>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, px: 2, py: 1.5, borderBottom: 1, borderColor: 'divider' }}>
        <Typography variant="h6" sx={{ fontWeight: 600, flexGrow: 1 }}>Whiteboards</Typography>
        <Button
          variant="contained"
          size="small"
          startIcon={<AddIcon />}
          onClick={() => setCreateOpen(true)}
        >
          New
        </Button>
      </Box>

      <Box sx={{ flex: 1, overflow: 'auto', px: 1.5, py: 1.5 }}>
        <Box sx={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 1.5 }}>
          {boardsWithMeta.map(b => (
            <Card
              key={b.name}
              variant="outlined"
              sx={{
                transition: 'box-shadow 0.2s, border-color 0.2s',
                '&:hover': { borderColor: 'primary.light' },
              }}
            >
              <CardActionArea onClick={() => loadBoard(b.name)} sx={{ height: '100%' }}>
                <Box sx={{
                  width: '100%',
                  height: 90,
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
                      sx={{ width: '100%', height: '100%', objectFit: 'contain' }}
                    />
                  ) : (
                    <DrawIcon sx={{ fontSize: 36, color: 'text.disabled' }} />
                  )}
                </Box>
                <CardContent sx={{ py: 1, px: 1.25, '&:last-child': { pb: 1 } }}>
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.25 }}>
                    <Typography variant="body2" noWrap sx={{ flexGrow: 1, fontSize: '0.8rem' }}>{b.name}</Typography>
                    <IconButton
                      size="small"
                      onClick={(e) => { e.stopPropagation(); setRenameTarget(b.name); setRenameValue(b.name); }}
                      sx={{ color: 'text.secondary', p: 0.25 }}
                    >
                      <EditIcon sx={{ fontSize: 14 }} />
                    </IconButton>
                    <IconButton
                      size="small"
                      onClick={(e) => { e.stopPropagation(); setDeleteTarget(b.name); }}
                      sx={{ color: 'error.main', p: 0.25 }}
                    >
                      <DeleteIcon sx={{ fontSize: 14 }} />
                    </IconButton>
                  </Box>
                  {b.elementCount > 0 && (
                    <Chip label={`${b.elementCount} el`} size="small" variant="outlined" sx={{ mt: 0.25, height: 18, fontSize: '0.6rem' }} />
                  )}
                </CardContent>
              </CardActionArea>
            </Card>
          ))}
        </Box>
        {boardsWithMeta.length === 0 && (
          <Typography variant="body2" color="text.secondary" align="center" sx={{ py: 6 }}>
            No whiteboards yet. Create one to get started.
          </Typography>
        )}
      </Box>

      <SwipeableDrawer
        anchor="bottom"
        open={createOpen}
        onClose={() => { setCreateOpen(false); setNewName(''); }}
        onOpen={() => setCreateOpen(true)}
        disableSwipeToOpen
        slotProps={{ paper: { sx: { borderTopLeftRadius: 16, borderTopRightRadius: 16, px: 2.5, pb: 3, pt: 2 } } }}
      >
        <Box sx={{ width: 40, height: 4, bgcolor: 'grey.300', borderRadius: 2, mx: 'auto', mb: 2 }} />
        <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1.5 }}>New Whiteboard</Typography>
        <TextField
          autoFocus
          fullWidth
          size="small"
          placeholder="Whiteboard name"
          value={newName}
          onChange={e => setNewName(e.target.value)}
          onKeyDown={e => { if (e.key === 'Enter') handleCreateSubmit(); }}
          sx={{ mb: 1.5 }}
        />
        <Button variant="contained" fullWidth onClick={handleCreateSubmit}>
          Create
        </Button>
      </SwipeableDrawer>

      <SwipeableDrawer
        anchor="bottom"
        open={renameTarget !== null}
        onClose={() => { setRenameTarget(null); setRenameValue(''); }}
        onOpen={() => {}}
        disableSwipeToOpen
        slotProps={{ paper: { sx: { borderTopLeftRadius: 16, borderTopRightRadius: 16, px: 2.5, pb: 3, pt: 2 } } }}
      >
        <Box sx={{ width: 40, height: 4, bgcolor: 'grey.300', borderRadius: 2, mx: 'auto', mb: 2 }} />
        <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1.5 }}>Rename Whiteboard</Typography>
        <TextField
          autoFocus
          fullWidth
          size="small"
          placeholder="New name"
          value={renameValue}
          onChange={e => setRenameValue(e.target.value)}
          onKeyDown={e => { if (e.key === 'Enter') handleRenameSubmit(); }}
          sx={{ mb: 1.5 }}
        />
        <Button variant="contained" fullWidth onClick={handleRenameSubmit}>
          Rename
        </Button>
      </SwipeableDrawer>

      <SwipeableDrawer
        anchor="bottom"
        open={deleteTarget !== null}
        onClose={() => setDeleteTarget(null)}
        onOpen={() => {}}
        disableSwipeToOpen
        slotProps={{ paper: { sx: { borderTopLeftRadius: 16, borderTopRightRadius: 16, px: 2.5, pb: 3, pt: 2 } } }}
      >
        <Box sx={{ width: 40, height: 4, bgcolor: 'grey.300', borderRadius: 2, mx: 'auto', mb: 2 }} />
        <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
          Delete &quot;{deleteTarget}&quot;?
        </Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          This cannot be undone.
        </Typography>
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button variant="outlined" fullWidth onClick={() => setDeleteTarget(null)}>
            Cancel
          </Button>
          <Button variant="contained" color="error" fullWidth onClick={handleDeleteConfirm}>
            Delete
          </Button>
        </Box>
      </SwipeableDrawer>
    </Box>
  );
}
