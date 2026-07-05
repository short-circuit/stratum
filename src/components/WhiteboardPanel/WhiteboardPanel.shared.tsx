import { useState, useEffect, useCallback, useRef } from 'react';
import { useTheme } from '@mui/material/styles';
import { restore, restoreLibraryItems } from '@excalidraw/excalidraw';
import type { SceneData, LibraryItems, ExcalidrawImperativeAPI } from '@excalidraw/excalidraw/types';
import * as api from '../../lib/commands';
import { setLatestLibraryJson } from '../../lib/libraryStore';
import { generateThumbnail } from './editorUtils';
import type { Board } from './BoardGallery';

export const emptyScene = { elements: [], appState: { viewBackgroundColor: '#ffffff' } };
const AUTOSAVE_MS = 800;

export interface WhiteboardPanelState {
  boards: Board[];
  activeBoard: string | null;
  sceneData: SceneData | null;
  libraryItems: LibraryItems | null;
  excalidrawRef: React.MutableRefObject<ExcalidrawImperativeAPI | null>;
  dirty: boolean;
  loadBoard: (name: string) => Promise<void>;
  createBoard: (name: string) => Promise<void>;
  deleteBoards: (names: string | string[]) => Promise<void>;
  handleRename: (oldName: string, newName: string) => Promise<void>;
  navigateBack: () => void;
  handleChange: () => void;
  handleLibraryChange: (items: LibraryItems) => void;
}

export function useWhiteboardPanel(): WhiteboardPanelState {
  const [boards, setBoards] = useState<Board[]>([]);
  const [activeBoard, setActiveBoard] = useState<string | null>(null);
  const [sceneData, setSceneData] = useState<SceneData | null>(null);
  const [libraryItems, setLibraryItems] = useState<LibraryItems | null>(null);
  const excalidrawRef = useRef<ExcalidrawImperativeAPI | null>(null);
  const autoSaveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [dirty, setDirty] = useState(false);
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

  const createBoard = async (name: string) => {
    if (!name.trim()) return;
    await api.saveWhiteboard(name, JSON.stringify(emptyScene));
    const updated = await api.listWhiteboards();
    setBoards(updated);
    loadBoard(name);
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
  }

  async function handleRename(oldName: string, newName: string) {
    if (!newName.trim() || newName === oldName) return;
    try {
      await api.renameWhiteboard(oldName, newName);
      const updated = await api.listWhiteboards();
      setBoards(updated);
    } catch (e) {
      console.error('Failed to rename whiteboard:', e);
    }
  }

  return {
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
  };
}
