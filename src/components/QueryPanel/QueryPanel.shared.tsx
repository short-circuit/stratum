import { useCallback, useEffect, useRef, useState } from 'react';
import * as api from '../../lib/commands';
import type { SavedQuery } from '../../lib/types';

export interface QueryResult {
  columns: string[];
  rows: string[][];
}

export function useDatalogQuery() {
  const [datalog, setDatalog] = useState(
    '{:query [:find ?b ?content :where [?b :block/marker "TODO"] [?b :block/content ?content]]}'
  );
  const [result, setResult] = useState<QueryResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);

  const doQuery = async () => {
    setRunning(true);
    setError(null);
    try {
      const res = await api.runQuery(datalog);
      setResult(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  };

  const resetQuery = () => {
    setDatalog('{:query [:find ?b :where [?b :block/marker "TODO"]]}');
  };

  return { datalog, setDatalog, result, error, running, doQuery, resetQuery };
}

export interface SavedQueriesState {
  saved: SavedQuery[];
  loading: boolean;
  loadError: string | null;
  actionError: string | null;
  refresh: () => Promise<void>;
  saveCurrent: (name: string) => Promise<void>;
  deleteQuery: (name: string) => Promise<void>;
  renameQuery: (oldName: string, newName: string) => Promise<void>;
  clearActionError: () => void;
}

export function useSavedQueries(currentDatalog: string): SavedQueriesState {
  const [saved, setSaved] = useState<SavedQuery[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const mountedRef = useRef(true);
  // Keep the latest editor content readable from stable callbacks without
  // recreating them on every keystroke.
  const datalogRef = useRef(currentDatalog);

  useEffect(() => {
    datalogRef.current = currentDatalog;
  }, [currentDatalog]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const refresh = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const list = await api.listSavedQueries();
      if (mountedRef.current) {
        setSaved(list);
        setLoading(false);
      }
    } catch (e) {
      if (mountedRef.current) {
        setLoadError(String(e));
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const saveCurrent = useCallback(
    async (name: string) => {
      setActionError(null);
      try {
        await api.saveSavedQuery(name, datalogRef.current);
      } catch (e) {
        setActionError(String(e));
        return;
      }
      await refresh();
    },
    [refresh],
  );

  const deleteQuery = useCallback(
    async (name: string) => {
      setActionError(null);
      try {
        await api.deleteSavedQuery(name);
      } catch (e) {
        setActionError(String(e));
        return;
      }
      await refresh();
    },
    [refresh],
  );

  const renameQuery = useCallback(
    async (oldName: string, newName: string) => {
      setActionError(null);
      try {
        await api.renameSavedQuery(oldName, newName);
      } catch (e) {
        setActionError(String(e));
        return;
      }
      await refresh();
    },
    [refresh],
  );

  const clearActionError = useCallback(() => setActionError(null), []);

  return {
    saved,
    loading,
    loadError,
    actionError,
    refresh,
    saveCurrent,
    deleteQuery,
    renameQuery,
    clearActionError,
  };
}

/** Shared helper to format the last-updated timestamp for display. */
export function formatUpdatedAt(updatedAt: string): string {
  if (!updatedAt) return '';
  const d = new Date(updatedAt);
  if (Number.isNaN(d.getTime())) return updatedAt;
  return d.toLocaleString();
}
