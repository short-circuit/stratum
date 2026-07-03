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
import { Excalidraw, MainMenu, restore, restoreLibraryItems, exportToCanvas } from '@excalidraw/excalidraw';
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
const THUMBNAIL_MAX = 300;

function parseBoardMeta(content: string) {
  let elementCount = 0;
  let preview: string | null = null;
  try {
    const parsed = JSON.parse(content);
    if (Array.isArray(parsed.elements)) {
      elementCount = parsed.elements.length;
    }
    if (typeof parsed.preview === 'string') {
      preview = parsed.preview;
    }
  } catch { /* ignore */ }
  return { elementCount, preview };
}

async function generateThumbnail(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  elements: readonly any[],
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  appState: any,
): Promise<string | null> {
  try {
    const canvas = await exportToCanvas({
      elements,
      appState: { ...appState, viewBackgroundColor: appState.viewBackgroundColor || '#ffffff' },
      files: null,
      maxWidthOrHeight: THUMBNAIL_MAX,
      exportPadding: 8,
    });
    return canvas.toDataURL('image/webp', 0.6);
  } catch (e) {
    console.warn('Failed to generate thumbnail:', e);
    return null;
  }
}

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

  const doSave = useCallback(async (generatePreview = false) => {
    if (!activeBoard || !excalidrawRef.current) return;
    try {
      const elements = excalidrawRef.current.getSceneElements();
      const { collaborators, ...cleanAppState } = excalidrawRef.current.getAppState();
      void collaborators;
      const data: Record<string, unknown> = {
        type: 'excalidraw',
        version: 2,
        source: 'stratum',
        elements,
        appState: cleanAppState,
      };
      if (generatePreview && elements.length > 0) {
        const thumb = await generateThumbnail(elements, cleanAppState);
        if (thumb) data.preview = thumb;
      }
      await api.saveWhiteboard(activeBoard, JSON.stringify(data));
      setDirty(false);
    } catch (e) {
      console.error('Failed to save whiteboard:', e);
    }
  }, [activeBoard]);

  useEffect(() => {
    if (!dirty) return;
    if (autoSaveTimer.current) clearTimeout(autoSaveTimer.current);
    autoSaveTimer.current = setTimeout(() => doSave(true), AUTOSAVE_MS);
    return () => {
      if (autoSaveTimer.current) clearTimeout(autoSaveTimer.current);
    };
  }, [dirty, doSave]);

  const navigateBack = useCallback(() => {
    if (autoSaveTimer.current) {
      clearTimeout(autoSaveTimer.current);
      autoSaveTimer.current = null;
    }
    const flush = dirty ? doSave(true) : Promise.resolve();
    flush.then(() => {
      setActiveBoard(null);
      setSceneData(null);
      api.listWhiteboards().then(setBoards);
    });
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

  const boardMeta = boards.map(b => {
    const { elementCount, preview } = parseBoardMeta(b.content);
    return { ...b, elementCount, preview };
  });

  if (!activeBoard) {
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
                  <Box sx={{
                    width: '100%',
                    height: 140,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    bgcolor: 'action.hover',
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
                  </Box>
                  <CardContent sx={{ py: 1.5, px: 2 }}>
                    <Typography variant="subtitle2" noWrap>{b.name}</Typography>
                    {b.elementCount > 0 && (
                      <Chip label={`${b.elementCount} elements`} size="small" variant="outlined" sx={{ mt: 0.5 }} />
                    )}
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
