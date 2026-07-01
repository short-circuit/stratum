import { useState, useEffect, useCallback, startTransition } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import * as api from '../lib/commands';
import type { SearchResultDto } from '../lib/types';

export default function SearchPanel() {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const [query, setQuery] = useState(searchParams.get('q') || '');
  const [results, setResults] = useState<SearchResultDto[]>([]);
  const [searching, setSearching] = useState(false);
  const [indexing, setIndexing] = useState(false);
  const [indexMsg, setIndexMsg] = useState('');

  const doSearch = useCallback(async (q: string) => {
    if (!q.trim()) return;
    setSearching(true);
    try {
      if (q.startsWith('#')) {
        const { results: r } = await api.searchByTag(q.slice(1));
        setResults(r);
      } else {
        const { results: r } = await api.searchBlocks(q, 20);
        setResults(r);
      }
    } catch (e) {
      console.error(e);
    } finally {
      setSearching(false);
    }
  }, []);

  // Auto-search when URL has ?q= param on mount or change
  useEffect(() => {
    const q = searchParams.get('q');
    if (q) {
      startTransition(() => { doSearch(q); });
    }
  }, [searchParams, doSearch]);

  const handleSearch = () => {
    if (!query.trim()) return;
    setSearchParams(query ? { q: query } : {});
    doSearch(query);
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
          onKeyDown={e => { if (e.key === 'Enter') handleSearch(); }}
          placeholder="Search blocks... (prefix with # to search tags)"
          className="flex-1 px-3 py-2 rounded border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] bg-white dark:bg-[var(--secondary-800)] text-sm"
        />
        <button
          onClick={handleSearch}
          disabled={searching}
          className="px-4 py-2 bg-[var(--primary-500)] text-white rounded text-sm hover:bg-[var(--primary-600)] disabled:opacity-50"
        >
          {searching ? '...' : 'Search'}
        </button>
      </div>
      <div className="flex items-center gap-2 mb-4">
        <button
          onClick={doReindex}
          disabled={indexing}
          className="text-xs text-[var(--secondary-400)] hover:text-[var(--secondary-600)] dark:hover:text-[var(--secondary-300)] disabled:opacity-50"
        >
          {indexing ? 'Indexing...' : 'Rebuild search index'}
        </button>
        {indexMsg && <span className="text-xs text-[var(--secondary-500)]">{indexMsg}</span>}
      </div>

      <div className="space-y-2">
        {results.map((r, i) => (
          <button
            key={i}
            onClick={() => navigate(`/page/${encodeURIComponent(r.page_path)}`)}
            className="w-full text-left p-3 rounded border border-[var(--secondary-200)] dark:border-[var(--secondary-700)] hover:border-[var(--primary-400)] hover:bg-[var(--primary-50)] dark:hover:bg-[var(--primary-900)]/10 transition-colors"
          >
            <div className="text-xs text-[var(--secondary-500)] mb-1 flex items-center gap-2">
              <span>{r.page_path}</span>
              <span className="text-[var(--secondary-300)]">·</span>
              <span>score: {r.score.toFixed(2)}</span>
            </div>
            <p className="text-sm">{r.snippet}</p>
          </button>
        ))}
        {results.length === 0 && query && !searching && (
          <p className="text-[var(--secondary-400)] text-sm">No results found.</p>
        )}
      </div>
    </div>
  );
}
