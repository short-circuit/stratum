import { useEffect, useRef, useState } from 'react';
import { useParams } from 'react-router-dom';
import { useStore } from '../stores/appStore';
import * as api from '../lib/commands';
import OutlinerEditor from './OutlinerEditor';
import BacklinksPanel from './BacklinksPanel';
import SuggestedConnectionsPanel from './SuggestedConnectionsPanel';

export default function PageView() {
  const { pagePath } = useParams<{ pagePath: string }>();
  const { currentPage, openPage, deletePage } = useStore();
  const [editorKey, setEditorKey] = useState(0);
  const [noteMenuOpen, setNoteMenuOpen] = useState(false);
  const [reindexing, setReindexing] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  // Close menu on outside click
  useEffect(() => {
    if (!noteMenuOpen) return;
    const handleClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setNoteMenuOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, [noteMenuOpen]);

  useEffect(() => {
    if (pagePath) {
      openPage(decodeURIComponent(pagePath));
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setEditorKey(k => k + 1);
    }
  }, [pagePath, openPage]);

  const handleReindex = async () => {
    if (!currentPage) return;
    setReindexing(true);
    try {
      await api.reindexPage(currentPage.path);
      setEditorKey(k => k + 1);
    } catch (e) {
      console.error('Reindex failed:', e);
    } finally {
      setReindexing(false);
      setNoteMenuOpen(false);
    }
  };

  const handleDelete = async () => {
    if (!currentPage) return;
    if (!confirm(`Delete "${currentPage.title || currentPage.slug}"?`)) return;
    try {
      await deletePage(currentPage.path);
      setNoteMenuOpen(false);
    } catch (e) {
      console.error('Delete failed:', e);
    }
  };

  if (!pagePath) {
    return (
      <div className="flex items-center justify-center h-full text-[var(--secondary-400)]">
        <div className="text-center">
          <h2 className="text-xl font-semibold mb-2">Stratum PKM</h2>
          <p className="text-sm">Select a page from the sidebar or create a new one.</p>
        </div>
      </div>
    );
  }

  if (!currentPage) {
    return <div className="p-4 text-[var(--secondary-400)]">Loading...</div>;
  }

  return (
    <div className="h-full flex flex-col">
      {/* Page header */}
      <div className="px-6 py-3 border-b border-[var(--secondary-200)] dark:border-[var(--secondary-700)] flex items-center gap-3">
        <h1 className="text-lg font-bold">
          {currentPage.title || currentPage.slug}
        </h1>
        <span className="text-xs text-[var(--secondary-400)]">
          {currentPage.path}
        </span>
        <div className="ml-auto relative" ref={menuRef}>
          <button
            onClick={() => setNoteMenuOpen(!noteMenuOpen)}
            disabled={reindexing}
            className="text-xs px-2 py-1 rounded border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] hover:bg-[var(--secondary-100)] dark:hover:bg-[var(--secondary-700)] text-[var(--secondary-600)] dark:text-[var(--secondary-400)] disabled:opacity-50"
          >
            {reindexing ? '…' : 'Note'}
          </button>
          {noteMenuOpen && (
            <div className="absolute right-0 top-full mt-1 bg-white dark:bg-[var(--secondary-800)] border border-[var(--secondary-200)] dark:border-[var(--secondary-700)] rounded-lg shadow-lg z-50 min-w-[160px] py-1">
              <div className="px-3 py-1 text-xs font-semibold text-[var(--secondary-400)] uppercase tracking-wider">
                Actions
              </div>
              <button
                onClick={handleReindex}
                disabled={reindexing}
                className="w-full text-left px-3 py-1.5 text-sm hover:bg-[var(--secondary-100)] dark:hover:bg-[var(--secondary-700)] text-[var(--secondary-700)] dark:text-[var(--secondary-300)] disabled:opacity-50"
              >
                Reindex Note
              </button>
              <div className="my-1 border-t border-[var(--secondary-200)] dark:border-[var(--secondary-700)]" />
              <button
                onClick={handleDelete}
                className="w-full text-left px-3 py-1.5 text-sm text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20"
              >
                Delete Page
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Editor + Backlinks */}
      <div className="flex-1 overflow-auto bg-[var(--secondary-50)] dark:bg-[var(--secondary-800)]">
        <OutlinerEditor key={editorKey} pagePath={currentPage.path} />
      </div>

      {/* Backlinks dock */}
      <BacklinksPanel pagePath={currentPage.path} />

      {/* Suggested connections */}
      <SuggestedConnectionsPanel pagePath={currentPage.path} />

    </div>
  );
}
