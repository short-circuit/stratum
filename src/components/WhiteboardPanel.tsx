import { useState, useEffect, useCallback, useRef } from 'react';
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
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
import DrawIcon from '@mui/icons-material/Draw';
import DeleteIcon from '@mui/icons-material/Delete';
import Chip from '@mui/material/Chip';
import Dialog from '@mui/material/Dialog';
import DialogTitle from '@mui/material/DialogTitle';
import DialogContent from '@mui/material/DialogContent';
import DialogContentText from '@mui/material/DialogContentText';
import DialogActions from '@mui/material/DialogActions';
import { useTheme } from '@mui/material/styles';
import { Excalidraw, MainMenu, restore, restoreLibraryItems, exportToCanvas } from '@excalidraw/excalidraw';
import '@excalidraw/excalidraw/index.css';
import type { ExcalidrawImperativeAPI, SceneData, LibraryItems } from '@excalidraw/excalidraw/types';
import * as api from '../lib/commands';
import { setLatestLibraryJson } from '../lib/libraryStore';

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

function invertHexColor(hex: string): string {
  const clean = hex.replace('#', '');
  if (clean.length !== 6) return hex;
  const r = (255 - parseInt(clean.substring(0, 2), 16)).toString(16).padStart(2, '0');
  const g = (255 - parseInt(clean.substring(2, 4), 16)).toString(16).padStart(2, '0');
  const b = (255 - parseInt(clean.substring(4, 6), 16)).toString(16).padStart(2, '0');
  return `#${r}${g}${b}`;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function adaptElementsTheme(elements: readonly any[], toDark: boolean): any[] {
  return elements.map(el => {
    if (!el || typeof el !== 'object') return el;
    const copy = { ...el };
    // Determine if this element likely uses dark-on-light or light-on-dark colors
    // by checking stroke and background lightness
    if (copy.strokeColor && copy.strokeColor.startsWith('#')) {
      const r = parseInt(copy.strokeColor.slice(1, 3), 16);
      const g = parseInt(copy.strokeColor.slice(3, 5), 16);
      const b = parseInt(copy.strokeColor.slice(5, 7), 16);
      const luminance = (r * 0.299 + g * 0.587 + b * 0.114) / 255;
      const isDarkStroke = luminance < 0.5;
      // If we need dark theme but element has dark strokes → invert
      // If we need light theme but element has light strokes → invert
      if ((toDark && isDarkStroke) || (!toDark && !isDarkStroke)) {
        copy.strokeColor = invertHexColor(copy.strokeColor);
        if (copy.backgroundColor && copy.backgroundColor.startsWith('#')) {
          copy.backgroundColor = invertHexColor(copy.backgroundColor);
        }
      }
    }
    return copy;
  });
}

async function generateThumbnail(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  elements: readonly any[],
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  appState: any,
  isDark: boolean,
): Promise<{ dataUrl: string; theme: 'dark' | 'light' } | null> {
  try {
    const bgColor = isDark ? '#232329' : '#ffffff';
    const adaptedElements = adaptElementsTheme(elements, isDark);
    const canvas = await exportToCanvas({
      elements: adaptedElements,
      appState: { ...appState, viewBackgroundColor: bgColor, exportWithDarkMode: isDark },
      files: null,
      maxWidthOrHeight: THUMBNAIL_MAX,
      exportPadding: 8,
    });
    return { dataUrl: canvas.toDataURL('image/webp', 0.6), theme: isDark ? 'dark' : 'light' };
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
  const [selectedBoards, setSelectedBoards] = useState<Set<string>>(new Set());
  const [confirmDelete, setConfirmDelete] = useState<string | string[] | null>(null);
  const [renameTarget, setRenameTarget] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');
  const muiTheme = useTheme();
  const isDarkRef = useRef(muiTheme.palette.mode === 'dark');
  useEffect(() => { isDarkRef.current = muiTheme.palette.mode === 'dark'; }, [muiTheme.palette.mode]);

  // Load library from Rust backend on mount
  useEffect(() => {
    api.listWhiteboards().then(setBoards);
    (async () => {
      try {
        const personal = await api.loadLibrary();
        const extra = await api.loadExtraLibraries();
        const personalItems = personal ? JSON.parse(personal) : [];
        const extraItems = extra ? JSON.parse(extra) : [];
        const merged = [...personalItems, ...extraItems];
        console.log('[library] loaded from disk, items:', merged.length);
        const restored = restoreLibraryItems(merged, 'published');
        setLibraryItems(restored);
      } catch (e) {
        console.error('[library] failed to load libraries:', e);
        setLibraryItems([]);
      }
    })();
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
        const thumb = await generateThumbnail(elements, cleanAppState, isDarkRef.current);
        if (thumb) {
          data.preview = thumb.dataUrl;
          data.previewTheme = thumb.theme;
        }
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
      // Reload library from disk every time a board opens
      const [personal, extra] = await Promise.all([
        api.loadLibrary(),
        api.loadExtraLibraries(),
      ]);
      const personalItems = personal ? JSON.parse(personal) : [];
      const extraItems = extra ? JSON.parse(extra) : [];
      const merged = [...personalItems, ...extraItems];
      const restored = restoreLibraryItems(merged, 'published');
      setLibraryItems(restored);

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

  // Save library to disk and update state
  const saveLibraryItems = useCallback(async (items: LibraryItems) => {
    const json = JSON.stringify(items);
    setLatestLibraryJson(json);
    setLibraryItems(items);
    try {
      await api.saveLibrary(json);
      console.log('[library] saved', items.length, 'items');
    } catch (e) {
      console.error('[library] save failed:', e);
    }
  }, []);

  // Auto-save whenever Excalidraw's library changes (with empty guard)
  const handleLibraryChange = useCallback((items: LibraryItems) => {
    console.log('[library] onChange', items.length, 'items');
    saveLibraryItems(items);
  }, [saveLibraryItems]);
  const handleChange = useCallback(() => {
    setDirty(true);
  }, []);

  function toggleSelectBoard(name: string, e: React.MouseEvent) {
    e.stopPropagation();
    setSelectedBoards(prev => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  }

  async function deleteBoards(names: string | string[]) {
    const list = Array.isArray(names) ? names : [names];
    try {
      for (const name of list) {
        await api.deleteWhiteboard(name);
      }
      const updated = await api.listWhiteboards();
      setBoards(updated);
    } catch (e) {
      console.error('Failed to delete whiteboard:', e);
    }
    setConfirmDelete(null);
    setSelectedBoards(new Set());
  }

  async function handleRename(oldName: string, newName: string) {
    if (!newName.trim() || newName === oldName) {
      setRenameTarget(null);
      return;
    }
    try {
      await api.renameWhiteboard(oldName, newName);
      const updated = await api.listWhiteboards();
      setBoards(updated);
    } catch (e) {
      console.error('Failed to rename whiteboard:', e);
    }
    setRenameTarget(null);
  }

  const boardMeta = boards.map(b => {
    const { elementCount, preview, previewTheme } = parseBoardMeta(b.content);
    return { ...b, elementCount, preview, previewTheme };
  });

  if (!activeBoard) {
    return (
      <Box sx={{ p: 3 }}>
        <Typography variant="h5" sx={{ fontWeight: 600, mb: 2 }}>Whiteboards</Typography>

        <Box sx={{ display: 'flex', gap: 1, mb: 3, alignItems: 'center' }}>
          <TextField
            size="small"
            placeholder="Whiteboard name"
            value={newName}
            onChange={e => setNewName(e.target.value)}
            onKeyDown={e => { if (e.key === 'Enter') createBoard(); }}
            sx={{ flexGrow: 1 }}
          />
          <Button variant="contained" onClick={createBoard} sx={{ whiteSpace: 'nowrap' }}>
            Create
          </Button>
          {selectedBoards.size > 0 && (
            <>
              <Typography variant="body2" color="text.secondary" sx={{ mx: 0.5 }}>
                {selectedBoards.size} selected
              </Typography>
              <Button
                variant="outlined"
                color="error"
                size="small"
                startIcon={<DeleteIcon />}
                onClick={() => setConfirmDelete(Array.from(selectedBoards))}
              >
                Delete
              </Button>
            </>
          )}
        </Box>

        <Grid container spacing={2}>
          {boardMeta.map(b => {
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
                <CardActionArea onClick={() => loadBoard(b.name)} sx={{ height: '100%' }}>
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
                      <Chip label={`${b.elementCount} elements`} size="small" variant="outlined" sx={{ mt: 0.5 }} />
                    )}
                  </CardContent>
                </CardActionArea>
              </Card>
            </Grid>
            );
          })}
          {boardMeta.length === 0 && (
            <Grid size={12}>
              <Typography variant="body2" color="text.secondary" align="center" sx={{ py: 4 }}>
                No whiteboards yet. Create one to get started.
              </Typography>
            </Grid>
          )}
        </Grid>

        <Dialog open={confirmDelete !== null} onClose={() => setConfirmDelete(null)}>
          <DialogTitle>Delete whiteboard{Array.isArray(confirmDelete) && confirmDelete.length > 1 ? 's' : ''}?</DialogTitle>
          <DialogContent>
            <DialogContentText>
              {Array.isArray(confirmDelete)
                ? `Delete ${confirmDelete.length} selected whiteboards? This cannot be undone.`
                : `Delete "${confirmDelete}"? This cannot be undone.`}
            </DialogContentText>
          </DialogContent>
          <DialogActions>
            <Button onClick={() => setConfirmDelete(null)}>Cancel</Button>
            <Button
              variant="contained"
              color="error"
              onClick={() => confirmDelete && deleteBoards(confirmDelete)}
              autoFocus
            >
              Delete
            </Button>
          </DialogActions>
        </Dialog>

        <Dialog open={renameTarget !== null} onClose={() => setRenameTarget(null)} maxWidth="xs" fullWidth>
          <DialogTitle>Rename whiteboard</DialogTitle>
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
            <Button onClick={() => setRenameTarget(null)}>Cancel</Button>
            <Button
              variant="contained"
              onClick={() => renameTarget && handleRename(renameTarget, renameValue)}
            >
              Rename
            </Button>
          </DialogActions>
        </Dialog>
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
