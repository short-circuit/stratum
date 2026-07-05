import { useState, useEffect, useCallback, startTransition } from 'react';
import { useSearchParams } from 'react-router-dom';
import * as api from '../../lib/commands';
import type { SearchResultDto } from '../../lib/types';

export function useSearchPanel() {
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

  return {
    query,
    setQuery,
    results,
    searching,
    indexing,
    indexMsg,
    doSearch,
    handleSearch,
    doReindex,
  };
}
