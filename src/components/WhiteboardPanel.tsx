import { useState, useEffect, useCallback } from 'react';
import { Tldraw, createTLStore, loadSnapshot, getSnapshot } from 'tldraw';
import 'tldraw/tldraw.css';
import * as api from '../lib/commands';

interface Board {
  name: string;
  path: string;
  content: string;
}

export default function WhiteboardPanel() {
  const [boards, setBoards] = useState<Board[]>([]);
  const [activeBoard, setActiveBoard] = useState<string | null>(null);
  const [newName, setNewName] = useState('');
  const [store] = useState(() => createTLStore());
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    api.listWhiteboards().then(setBoards);
  }, []);

  const loadBoard = useCallback(async (name: string) => {
    try {
      const content = await api.loadWhiteboard(name);
      if (content) {
        try {
          const data = JSON.parse(content);
          if (data.document) {
            loadSnapshot(store, data);
          }
        } catch {
          // Start with empty board
        }
      }
      setActiveBoard(name);
      setLoaded(true);
    } catch (e) {
      console.error('Failed to load whiteboard:', e);
    }
  }, [store]);

  const createBoard = async () => {
    if (!newName.trim()) return;
    await api.saveWhiteboard(newName, JSON.stringify({ document: { store: {} } }));
    setNewName('');
    const updated = await api.listWhiteboards();
    setBoards(updated);
    loadBoard(newName);
  };

  const saveBoard = useCallback(async () => {
    if (!activeBoard) return;
    try {
      const snapshot = getSnapshot(store);
      await api.saveWhiteboard(activeBoard, JSON.stringify(snapshot));
    } catch (e) {
      console.error('Failed to save whiteboard:', e);
    }
  }, [activeBoard, store]);

  if (!activeBoard) {
    return (
      <div className="p-4 max-w-2xl mx-auto">
        <h2 className="text-lg font-semibold mb-3">Whiteboards</h2>

        {/* Create new */}
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

        {/* Board list */}
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
      {/* Toolbar */}
      <div className="flex items-center gap-2 px-3 py-2 border-b border-[var(--secondary-200)] dark:border-[var(--secondary-700)] bg-white dark:bg-[var(--secondary-900)]">
        <button
          onClick={() => { setActiveBoard(null); setLoaded(false); }}
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

      {/* Canvas */}
      <div className="flex-1">
        {loaded && (
          <Tldraw store={store} />
        )}
      </div>
    </div>
  );
}
