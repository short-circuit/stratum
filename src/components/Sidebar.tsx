import { useNavigate } from 'react-router-dom';
import { useStore } from '../stores/appStore';
import * as api from '../lib/commands';
import { useState } from 'react';

const NAV_ITEMS = [
  { id: 'journal', label: 'Journal', path: '/journal', icon: '📅' },
  { id: 'pages', label: 'Pages', path: '/', icon: '📄' },
  { id: 'search', label: 'Search', path: '/search', icon: '🔍' },
  { id: 'query', label: 'Query', path: '/query', icon: '▷' },
  { id: 'templates', label: 'Templates', path: '/templates', icon: '📋' },
  { id: 'flashcards', label: 'Flashcards', path: '/flashcards', icon: '🃏' },
  { id: 'whiteboards', label: 'Whiteboards', path: '/whiteboards', icon: '🎨' },
] as const;

type TabId = (typeof NAV_ITEMS)[number]['id'];

export default function Sidebar() {
  const { pages, vault, loadPages } = useStore();
  const navigate = useNavigate();
  const [showNew, setShowNew] = useState(false);
  const [newPath, setNewPath] = useState('');
  const [newTitle, setNewTitle] = useState('');
  const [activeTab, setActiveTab] = useState<TabId>('journal');
  const { createPage, deletePage } = useStore();
  const [exporting, setExporting] = useState(false);

  const handleExport = async () => {
    setExporting(true);
    try {
      const dir = '/tmp/stratum-export';
      const result = await api.exportHtml(dir);
      alert(`Exported ${result.pages_exported} pages to ${dir}`);
    } catch (e) {
      alert(`Export failed: ${e}`);
    } finally {
      setExporting(false);
    }
  };

  const handleCreate = async () => {
    if (!newPath) return;
    await createPage(newPath, newTitle || undefined);
    setShowNew(false);
    setNewPath('');
    setNewTitle('');
  };

  const navigateTab = (tab: TabId, path: string) => {
    setActiveTab(tab);
    navigate(path);
  };

  return (
    <aside className="w-56 bg-gray-50 dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 flex flex-col shrink-0">
      {/* Header */}
      <div className="p-3 border-b border-gray-200 dark:border-gray-700">
        <h1 className="text-base font-bold">Stratum</h1>
        {vault && (
          <p className="text-xs text-gray-500 dark:text-gray-400 truncate mt-0.5">
            {vault.block_count} blocks · {vault.page_count} pages
          </p>
        )}
      </div>

      {/* Vertical nav list */}
      <nav className="flex-1 overflow-auto py-1">
        {NAV_ITEMS.map(item => (
          <button
            key={item.id}
            onClick={() => navigateTab(item.id, item.path)}
            className={`w-full flex items-center gap-2 px-3 py-1.5 text-xs text-left transition-colors ${
              activeTab === item.id
                ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300 font-medium'
                : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-750'
            }`}
          >
            <span className="w-4 text-center">{item.icon}</span>
            <span>{item.label}</span>
          </button>
        ))}

        {/* Separator */}
        <div className="mx-3 my-2 border-t border-gray-200 dark:border-gray-700" />

        {/* Page list (always visible below nav) */}
        <div className="px-1">
          <div className="flex items-center justify-between px-2 py-1">
            <span className="text-xs font-semibold text-gray-500 uppercase">Pages</span>
            <button
              onClick={() => setShowNew(!showNew)}
              className="text-xs px-1.5 py-0.5 rounded hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-500"
            >
              + New
            </button>
          </div>

          {showNew && (
            <div className="p-2 mx-1 mb-1 bg-gray-100 dark:bg-gray-750 rounded">
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
                No pages yet.
              </p>
            )}
          </ul>
        </div>
      </nav>

      {/* Footer */}
      <div className="p-2 border-t border-gray-200 dark:border-gray-700 text-xs text-gray-400 flex items-center justify-between">
        <button onClick={() => loadPages()} className="hover:text-gray-600 dark:hover:text-gray-300">
          Refresh
        </button>
        <button
          onClick={handleExport}
          disabled={exporting}
          className="hover:text-gray-600 dark:hover:text-gray-300 disabled:opacity-50"
        >
          {exporting ? '...' : 'Export'}
        </button>
        <span className="text-gray-300 dark:text-gray-600">v0.2</span>
      </div>
    </aside>
  );
}
