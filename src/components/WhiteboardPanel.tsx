import { useState, useEffect, useCallback, useRef } from 'react';
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
      <div className="p-4 max-w-2xl mx-auto">
        <h2 className="text-lg font-semibold mb-3">Whiteboards</h2>

        <div className="flex gap-2 mb-4">
          <input
            type="text"
            value={newName}
            onChange={e => setNewName(e.target.value)}
            onKeyDown={e => { if (e.key === 'Enter') createBoard(); }}
            placeholder="Whiteboard name"
            className="flex-1 text-sm px-2 py-1 rounded border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] bg-white dark:bg-[var(--secondary-800)]"
          />
          <button
            onClick={createBoard}
            className="px-3 py-1 bg-[var(--primary-500)] text-white text-sm rounded hover:bg-[var(--primary-600)]"
          >
            Create
          </button>
        </div>

        <div className="space-y-1">
          {boards.map(b => (
            <button
              key={b.name}
              onClick={() => loadBoard(b.name)}
              className="w-full text-left px-3 py-2 text-sm rounded hover:bg-[var(--secondary-100)] dark:hover:bg-[var(--secondary-800)]"
            >
              {b.name}
            </button>
          ))}
          {boards.length === 0 && (
            <p className="text-sm text-[var(--secondary-400)]">No whiteboards yet.</p>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      <div className="flex items-center gap-2 px-3 py-2 border-b border-[var(--secondary-200)] dark:border-[var(--secondary-700)] bg-white dark:bg-[var(--secondary-900)]">
        <button
          onClick={() => { setActiveBoard(null); setSceneData(null); }}
          className="text-sm px-2 py-1 rounded hover:bg-[var(--secondary-100)] dark:hover:bg-[var(--secondary-800)]"
        >
          ← Back
        </button>
        <span className="text-sm font-medium">{activeBoard}</span>
        <div className="flex-1" />
        <button
          onClick={saveBoard}
          className="text-sm px-3 py-1 bg-[var(--primary-500)] text-white rounded hover:bg-[var(--primary-600)]"
        >
          Save
        </button>
      </div>

      <div className="flex-1 relative min-h-0">
        {sceneData && libraryItems && (
          <Excalidraw
            key={activeBoard}
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
      </div>
    </div>
  );
}
