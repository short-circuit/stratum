import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import * as api from '../lib/commands';
import type { SearchResultDto } from '../lib/types';

export default function SearchPanel() {
  const navigate = useNavigate();
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResultDto[]>([]);
  const [searching, setSearching] = useState(false);
  const [indexing, setIndexing] = useState(false);
  const [indexMsg, setIndexMsg] = useState('');

  const doSearch = async () => {
    if (!query.trim()) return;
    setSearching(true);
    try {
      const { results: r } = await api.searchBlocks(query, 20);
      setResults(r);
    } catch (e) {
      console.error(e);
    } finally {
      setSearching(false);
    }
  };

  const doReindex = async () => {
    setIndexing(true);
    try {
      const msg = await api.rebuildSearchIndex();
      setIndexMsg(msg);
    } catch (e) {
      setIndexMsg(`Failed: ${e}`);
    } finally {
      setIndexing(false);
    }
  };

  return (
    <div className="p-4 max-w-3xl mx-auto">
      <div className="flex gap-2 mb-2">
        <input
          type="text"
          value={query}
          onChange={e => setQuery(e.target.value)}
          onKeyDown={e => { if (e.key === 'Enter') doSearch(); }}
          placeholder="Search blocks..."
          className="flex-1 px-3 py-2 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-sm"
        />
        <button
          onClick={doSearch}
          disabled={searching}
          className="px-4 py-2 bg-[var(--accent-500)] text-white rounded text-sm hover:bg-[var(--accent-600)] disabled:opacity-50"
        >
          {searching ? '...' : 'Search'}
        </button>
      </div>
      <div className="flex items-center gap-2 mb-4">
        <button
          onClick={doReindex}
          disabled={indexing}
          className="text-xs text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 disabled:opacity-50"
        >
          {indexing ? 'Indexing...' : 'Rebuild search index'}
        </button>
        {indexMsg && <span className="text-xs text-gray-500">{indexMsg}</span>}
      </div>

      <div className="space-y-2">
        {results.map((r, i) => (
          <button
            key={i}
            onClick={() => navigate(`/page/${encodeURIComponent(r.page_path)}`)}
            className="w-full text-left p-3 rounded border border-gray-200 dark:border-gray-700 hover:border-[var(--accent-400)] hover:bg-[var(--accent-50)] dark:hover:bg-[var(--accent-900)]/10 transition-colors"
          >
            <div className="text-xs text-gray-500 mb-1 flex items-center gap-2">
              <span>{r.page_path}</span>
              <span className="text-gray-300">·</span>
              <span>score: {r.score.toFixed(2)}</span>
            </div>
            <p className="text-sm">{r.snippet}</p>
          </button>
        ))}
        {results.length === 0 && query && !searching && (
          <p className="text-gray-400 text-sm">No results found.</p>
        )}
      </div>
    </div>
  );
}
