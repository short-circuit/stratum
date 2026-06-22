import { useNavigate } from 'react-router-dom';
import { useStore } from '../stores/appStore';
import { useState } from 'react';

export default function Sidebar() {
  const { pages, vault, loadPages } = useStore();
  const navigate = useNavigate();
  const [showNew, setShowNew] = useState(false);
  const [newPath, setNewPath] = useState('');
  const [newTitle, setNewTitle] = useState('');
  const { createPage, deletePage } = useStore();

  const handleCreate = async () => {
    if (!newPath) return;
    await createPage(newPath, newTitle || undefined);
    setShowNew(false);
    setNewPath('');
    setNewTitle('');
  };

  return (
    <aside className="w-64 bg-gray-50 dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 flex flex-col">
      {/* Header */}
      <div className="p-3 border-b border-gray-200 dark:border-gray-700">
        <h1 className="text-lg font-bold">Stratum</h1>
        {vault && (
          <p className="text-xs text-gray-500 dark:text-gray-400 truncate">
            {vault.path} · {vault.block_count} blocks
          </p>
        )}
      </div>

      {/* Nav */}
      <nav className="p-2 flex gap-1 text-sm">
        <button
          onClick={() => navigate('/')}
          className="flex-1 px-2 py-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700"
        >
          Pages
        </button>
        <button
          onClick={() => navigate('/search')}
          className="flex-1 px-2 py-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700"
        >
          Search
        </button>
        <button
          onClick={() => navigate('/query')}
          className="flex-1 px-2 py-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700"
        >
          Query
        </button>
      </nav>

      {/* Page list */}
      <div className="flex-1 overflow-auto p-1">
        <div className="flex items-center justify-between px-2 py-1">
          <span className="text-xs font-semibold text-gray-500 uppercase">Pages</span>
          <button
            onClick={() => setShowNew(!showNew)}
            className="text-xs px-1 rounded hover:bg-gray-200 dark:hover:bg-gray-700"
          >
            + New
          </button>
        </div>

        {showNew && (
          <div className="p-2 bg-gray-100 dark:bg-gray-750 rounded m-1">
            <input
              type="text"
              placeholder="Path (e.g., pages/my-note.md)"
              value={newPath}
              onChange={e => setNewPath(e.target.value)}
              className="w-full text-xs p-1 mb-1 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700"
            />
            <input
              type="text"
              placeholder="Title (optional)"
              value={newTitle}
              onChange={e => setNewTitle(e.target.value)}
              className="w-full text-xs p-1 mb-1 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700"
            />
            <div className="flex gap-1">
              <button
                onClick={handleCreate}
                className="text-xs px-2 py-0.5 bg-blue-500 text-white rounded hover:bg-blue-600"
              >
                Create
              </button>
              <button
                onClick={() => setShowNew(false)}
                className="text-xs px-2 py-0.5 text-gray-500 rounded hover:bg-gray-200 dark:hover:bg-gray-700"
              >
                Cancel
              </button>
            </div>
          </div>
        )}

        <ul className="space-y-0.5">
          {pages.map(page => (
            <li
              key={page.path}
              className="group flex items-center px-2 py-1 text-sm rounded cursor-pointer hover:bg-gray-200 dark:hover:bg-gray-700"
              onClick={() => navigate(`/page/${encodeURIComponent(page.path)}`)}
            >
              <span className="flex-1 truncate">
                {page.title || page.slug}
              </span>
              <span className="text-xs text-gray-400 ml-1">{page.block_count}</span>
              <button
                onClick={e => {
                  e.stopPropagation();
                  if (confirm(`Delete ${page.path}?`)) deletePage(page.path);
                }}
                className="opacity-0 group-hover:opacity-100 text-xs px-1 text-red-500 hover:bg-red-100 dark:hover:bg-red-900 rounded ml-1"
              >
                ×
              </button>
            </li>
          ))}
          {pages.length === 0 && (
            <p className="px-2 py-4 text-xs text-gray-400 text-center">
              No pages yet. Create one to get started.
            </p>
          )}
        </ul>
      </div>

      {/* Footer */}
      <div className="p-2 border-t border-gray-200 dark:border-gray-700 text-xs text-gray-400">
        <button onClick={() => loadPages()} className="hover:text-gray-600 dark:hover:text-gray-300">
          Refresh
        </button>
      </div>
    </aside>
  );
}
