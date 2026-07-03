import { useState, useEffect, useCallback, useRef } from 'react';
import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import CardActionArea from '@mui/material/CardActionArea';
import Grid from '@mui/material/Grid';
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
import DrawIcon from '@mui/icons-material/Draw';
import Chip from '@mui/material/Chip';
import { Excalidraw, MainMenu, restore, restoreLibraryItems } from '@excalidraw/excalidraw';
import '@excalidraw/excalidraw/index.css';
import type { ExcalidrawImperativeAPI, SceneData, LibraryItems } from '@excalidraw/excalidraw/types';
import * as api from '../lib/commands';

interface Board {
  name: string;
  path: string;
  content: string;
}

const emptyScene = { elements: [], appState: { viewBackgroundColor: '#ffffff' } };
const AUTOSAVE_MS = 800;

export default function WhiteboardPanel() {
  const [boards, setBoards] = useState<Board[]>([]);
  const [activeBoard, setActiveBoard] = useState<string | null>(null);
  const [newName, setNewName] = useState('');
  const [sceneData, setSceneData] = useState<SceneData | null>(null);
  const [libraryItems, setLibraryItems] = useState<LibraryItems | null>(null);
  const excalidrawRef = useRef<ExcalidrawImperativeAPI | null>(null);
  const autoSaveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    api.listWhiteboards().then(setBoards);
    api.loadLibrary().then(content => {
      if (content) {
        setLibraryItems(restoreLibraryItems(JSON.parse(content), 'published'));
      } else {
        setLibraryItems([]);
      }
    });
  }, []);

  const doSave = useCallback(async () => {
    if (!activeBoard || !excalidrawRef.current) return;
    try {
      const elements = excalidrawRef.current.getSceneElements();
      const { collaborators, ...cleanAppState } = excalidrawRef.current.getAppState();
      void collaborators;
      await api.saveWhiteboard(activeBoard, JSON.stringify({
        type: 'excalidraw',
        version: 2,
        source: 'stratum',
        elements,
        appState: cleanAppState,
      }));
      setDirty(false);
    } catch (e) {
      console.error('Failed to save whiteboard:', e);
    }
  }, [activeBoard]);

  // Auto-save on dirty changes (debounced)
  useEffect(() => {
    if (!dirty) return;
    if (autoSaveTimer.current) clearTimeout(autoSaveTimer.current);
    autoSaveTimer.current = setTimeout(doSave, AUTOSAVE_MS);
    return () => {
      if (autoSaveTimer.current) clearTimeout(autoSaveTimer.current);
    };
  }, [dirty, doSave]);

  // Also save when navigating away from the board
  const navigateBack = useCallback(() => {
    // Flush pending auto-save immediately
    if (autoSaveTimer.current) {
      clearTimeout(autoSaveTimer.current);
      autoSaveTimer.current = null;
    }
    if (dirty) {
      doSave().then(() => {
        setActiveBoard(null);
        setSceneData(null);
      });
    } else {
      setActiveBoard(null);
      setSceneData(null);
    }
  }, [dirty, doSave]);

  const loadBoard = useCallback(async (name: string) => {
    try {
      const content = await api.loadWhiteboard(name);
      if (content) {
        const parsed = JSON.parse(content);
        if (parsed.appState) {
          delete parsed.appState.collaborators;
        }
        setSceneData(restore(parsed, null, null));
      } else {
        setSceneData(emptyScene);
      }
      setActiveBoard(name);
      setDirty(false);
    } catch (e) {
      console.error('Failed to load whiteboard:', e);
    }
  }, []);

  const createBoard = async () => {
    if (!newName.trim()) return;
    await api.saveWhiteboard(newName, JSON.stringify(emptyScene));
    setNewName('');
    const updated = await api.listWhiteboards();
    setBoards(updated);
    loadBoard(newName);
  };

  const handleChange = useCallback(() => {
    setDirty(true);
  }, []);

  const handleLibraryChange = useCallback((items: LibraryItems) => {
    api.saveLibrary(JSON.stringify(items));
  }, []);

  // --- Listing view ---
  if (!activeBoard) {
    // Parse element count from stored content without loading the full scene
    const boardMeta = boards.map(b => {
      let elementCount = 0;
      try {
        const parsed = JSON.parse(b.content);
        if (Array.isArray(parsed.elements)) {
          elementCount = parsed.elements.length;
        }
      } catch { /* ignore */ }
      return { ...b, elementCount };
    });

    return (
      <Box sx={{ p: 3 }}>
        <Typography variant="h5" sx={{ fontWeight: 600, mb: 2 }}>Whiteboards</Typography>

        <Box sx={{ display: 'flex', gap: 1, mb: 3 }}>
          <TextField
            size="small"
            placeholder="Whiteboard name"
            value={newName}
            onChange={e => setNewName(e.target.value)}
            onKeyDown={e => { if (e.key === 'Enter') createBoard(); }}
            fullWidth
          />
          <Button variant="contained" onClick={createBoard} sx={{ whiteSpace: 'nowrap' }}>
            Create
          </Button>
        </Box>

        <Grid container spacing={2}>
          {boardMeta.map(b => (
            <Grid key={b.name} size={{ xs: 12, sm: 6, md: 4 }}>
              <Card
                variant="outlined"
                sx={{
                  transition: 'box-shadow 0.2s, border-color 0.2s',
                  '&:hover': { borderColor: 'primary.light', boxShadow: 2 },
                }}
              >
                <CardActionArea onClick={() => loadBoard(b.name)} sx={{ height: '100%' }}>
                  <CardContent sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 1, py: 4 }}>
                    <DrawIcon sx={{ fontSize: 48, color: 'primary.main', opacity: 0.7 }} />
                    <Typography variant="subtitle2" align="center" noWrap sx={{ maxWidth: '100%' }}>
                      {b.name}
                    </Typography>
                    <Chip label={`${b.elementCount} elements`} size="small" variant="outlined" />
                  </CardContent>
                </CardActionArea>
              </Card>
            </Grid>
          ))}
          {boardMeta.length === 0 && (
            <Grid size={12}>
              <Typography variant="body2" color="text.secondary" align="center" sx={{ py: 4 }}>
                No whiteboards yet. Create one to get started.
              </Typography>
            </Grid>
          )}
        </Grid>
      </Box>
    );
  }

  // --- Active board view ---
  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, px: 1.5, py: 1, borderBottom: 1, borderColor: 'divider', bgcolor: 'background.default' }}>
        <IconButton size="small" onClick={navigateBack}>
          <ArrowBackIcon fontSize="small" />
        </IconButton>
        <Typography variant="body2" sx={{ fontWeight: 500 }}>{activeBoard}</Typography>
        {dirty && (
          <Chip label="Unsaved" size="small" color="warning" variant="outlined" sx={{ fontSize: '0.65rem', height: 20 }} />
        )}
        <Box sx={{ flex: 1 }} />
      </Box>

      <Box sx={{ flex: 1, position: 'relative', minHeight: 0 }}>
        {sceneData && libraryItems && (
          <Excalidraw
            key={activeBoard}
            theme={document.documentElement.classList.contains('dark') ? 'dark' : 'light'}
            initialData={{ ...sceneData, libraryItems }}
            excalidrawAPI={(api: ExcalidrawImperativeAPI) => { excalidrawRef.current = api; }}
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
        )}
      </Box>
    </Box>
  );
}
