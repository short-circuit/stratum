import { useState } from 'react';
import * as api from '../lib/commands';
import type { ConnectionSuggestion } from '../lib/types';
import { useNavigate } from 'react-router-dom';

interface Props {
  pagePath: string;
}

export default function SuggestedConnectionsPanel({ pagePath }: Props) {
  const navigate = useNavigate();
  const [suggestions, setSuggestions] = useState<ConnectionSuggestion[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [collapsed, setCollapsed] = useState(true);

  const findConnections = async () => {
    setBusy(true);
    setSuggestions(null);
    try {
      const result = await api.suggestConnections(pagePath);
      setSuggestions(result);
    } catch (e) {
      console.error('Failed to find connections:', e);
    } finally {
      setBusy(false);
    }
  };

  const addLink = async (targetTitle: string) => {
    try {
      await api.insertBlock(pagePath, `See also: [[${targetTitle}]]`);
      findConnections();
    } catch (e) {
      console.error('Failed to add link:', e);
    }
  };

  return (
    <div className="border-t border-[var(--secondary-200)] dark:border-[var(--secondary-700)]">
      <button
        onClick={() => setCollapsed(!collapsed)}
        className="w-full flex items-center justify-between px-4 py-2 text-xs font-semibold text-[var(--secondary-500)] uppercase hover:bg-[var(--secondary-50)] dark:hover:bg-[var(--secondary-800)] transition-colors"
      >
        <span>Suggested Connections</span>
        <span className={`transform transition-transform ${collapsed ? '' : 'rotate-90'}`}>&#9654;</span>
      </button>

      {!collapsed && (
        <div className="px-4 pb-3">
          <div className="mb-2">
            <button
              onClick={findConnections}
              disabled={busy}
              className="text-[11px] px-2 py-1 rounded bg-[var(--primary-500)] text-white hover:bg-[var(--primary-600)] disabled:opacity-50 transition-colors"
            >
              {busy ? 'Searching...' : 'Find'}
            </button>
          </div>

          {suggestions && suggestions.length === 0 && (
            <p className="text-xs text-[var(--secondary-400)]">No connections found.</p>
          )}

          {suggestions && suggestions.length > 0 && (
            <ul className="space-y-2">
              {suggestions.map(s => (
                <li key={s.page_path} className="text-xs">
                  <div className="flex items-start justify-between gap-2">
                    <button
                      onClick={() => navigate(`/page/${encodeURIComponent(s.page_path)}`)}
                      className="font-medium text-[var(--primary-600)] dark:text-[var(--primary-400)] hover:underline text-left leading-tight truncate"
                    >
                      {s.title}
                    </button>
                    <button
                      onClick={() => addLink(s.title)}
                      className="shrink-0 text-[10px] px-1.5 py-0.5 rounded bg-[var(--secondary-100)] dark:bg-[var(--secondary-700)] hover:bg-[var(--secondary-200)] dark:hover:bg-[var(--secondary-600)] transition-colors"
                      title="Add [[wiki-link]] to this page"
                    >
                      + Link
                    </button>
                  </div>
                  {s.snippet && (
                    <p className="text-[10px] text-[var(--secondary-400)] mt-0.5 line-clamp-2">{s.snippet}</p>
                  )}
                  <div className="flex items-center gap-1 mt-0.5">
                    <span className="text-[9px] text-[var(--secondary-400)]">
                      Relevance: {Math.min(100, s.score)}%
                    </span>
                  </div>
                </li>
              ))}
            </ul>
          )}

          {suggestions === null && !busy && (
            <p className="text-[10px] text-[var(--secondary-400)]">
              Click "Find" to discover related notes.
            </p>
          )}
        </div>
      )}
    </div>
  );
}
