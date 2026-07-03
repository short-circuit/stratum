import { useState, useEffect, useCallback, useRef } from 'react';
import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import List from '@mui/material/List';
import ListItemButton from '@mui/material/ListItemButton';
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
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

export default function WhiteboardPanel() {
  const [boards, setBoards] = useState<Board[]>([]);
  const [activeBoard, setActiveBoard] = useState<string | null>(null);
  const [newName, setNewName] = useState('');
  const [sceneData, setSceneData] = useState<SceneData | null>(null);
  const [libraryItems, setLibraryItems] = useState<LibraryItems | null>(null);
  const excalidrawRef = useRef<ExcalidrawImperativeAPI | null>(null);

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

  const saveBoard = useCallback(async () => {
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
    } catch (e) {
      console.error('Failed to save whiteboard:', e);
    }
  }, [activeBoard]);

  const handleLibraryChange = useCallback((items: LibraryItems) => {
    api.saveLibrary(JSON.stringify(items));
  }, []);

  if (!activeBoard) {
    return (
      <Box sx={{ p: 3, maxWidth: 600, mx: 'auto' }}>
        <Typography variant="h5" sx={{ fontWeight: 600, mb: 2 }}>Whiteboards</Typography>

        <Box sx={{ display: 'flex', gap: 1, mb: 2 }}>
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

        <List disablePadding>
          {boards.map(b => (
            <ListItemButton key={b.name} onClick={() => loadBoard(b.name)} sx={{ borderRadius: 1 }}>
              {b.name}
            </ListItemButton>
          ))}
          {boards.length === 0 && (
            <Typography variant="body2" color="text.secondary">No whiteboards yet.</Typography>
          )}
        </List>
      </Box>
    );
  }

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, px: 1.5, py: 1, borderBottom: 1, borderColor: 'divider', bgcolor: 'background.default' }}>
        <IconButton size="small" onClick={() => { setActiveBoard(null); setSceneData(null); }}>
          <ArrowBackIcon fontSize="small" />
        </IconButton>
        <Typography variant="body2" sx={{ fontWeight: 500 }}>{activeBoard}</Typography>
        <Box sx={{ flex: 1 }} />
        <Button size="small" variant="contained" onClick={saveBoard}>
          Save
        </Button>
      </Box>

      <Box sx={{ flex: 1, position: 'relative', minHeight: 0 }}>
        {sceneData && libraryItems && (
          <Excalidraw
            key={activeBoard}
            theme={document.documentElement.classList.contains('dark') ? 'dark' : 'light'}
            initialData={{ ...sceneData, libraryItems }}
            excalidrawAPI={(api: ExcalidrawImperativeAPI) => { excalidrawRef.current = api; }}
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
