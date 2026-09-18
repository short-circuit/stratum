import { useCallback, useEffect, useState } from 'react';
import * as api from '../../lib/commands';
import { isPluginBackendReady } from '../../lib/commands/plugins';
import type { PluginHttpRequestParams, PluginInfoDto } from '../../lib/types';

export interface PluginTestResult {
  label: string;
  ok: boolean;
  detail: string;
}

export interface PluginsPanelState {
  plugins: PluginInfoDto[];
  loading: boolean;
  error: string | null;
  busyId: string | null;
  testResults: Record<string, PluginTestResult | null>;
  backendReady: boolean;
  refresh: () => Promise<void>;
  enable: (id: string) => Promise<void>;
  disable: (id: string) => Promise<void>;
  reload: (id: string) => Promise<void>;
  runNoteReadTest: (id: string) => Promise<void>;
  runHttpRequestTest: (id: string) => Promise<void>;
  clearTestResult: (id: string) => void;
  clearError: () => void;
}

export function usePluginsPanel(): PluginsPanelState {
  const [plugins, setPlugins] = useState<PluginInfoDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<Record<string, PluginTestResult | null>>({});
  const [backendReady] = useState<boolean>(() => isPluginBackendReady());

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await api.pluginsList();
      setPlugins(result.plugins);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const runAction = useCallback(
    async (id: string, action: (id: string) => Promise<PluginInfoDto>) => {
      setBusyId(id);
      setError(null);
      try {
        const updated = await action(id);
        setPlugins(prev => {
          const exists = prev.some(p => p.id === updated.id);
          if (exists) return prev.map(p => (p.id === updated.id ? updated : p));
          return [...prev, updated];
        });
      } catch (e) {
        setError(`Failed on ${id}: ${String(e)}`);
      } finally {
        setBusyId(null);
      }
    },
    [],
  );

  const enable = useCallback((id: string) => runAction(id, api.pluginsEnable), [runAction]);
  const disable = useCallback((id: string) => runAction(id, api.pluginsDisable), [runAction]);
  const reload = useCallback((id: string) => runAction(id, api.pluginsReload), [runAction]);

  const runNoteReadTest = useCallback(async (id: string) => {
    setBusyId(id);
    setError(null);
    try {
      const result = await api.pluginNoteRead('test-plugin.md');
      setTestResults(prev => ({
        ...prev,
        [id]: {
          label: 'note_read',
          ok: true,
          detail: `${result.path} (${result.content.length} bytes)`,
        },
      }));
    } catch (e) {
      setTestResults(prev => ({
        ...prev,
        [id]: { label: 'note_read', ok: false, detail: String(e) },
      }));
    } finally {
      setBusyId(null);
    }
  }, []);

  const runHttpRequestTest = useCallback(async (id: string) => {
    setBusyId(id);
    setError(null);
    try {
      const params: PluginHttpRequestParams = {
        method: 'GET',
        url: 'https://example.com/data.json',
        timeout_ms: 2000,
      };
      const result = await api.pluginHttpRequest(params);
      setTestResults(prev => ({
        ...prev,
        [id]: {
          label: 'http_request',
          ok: true,
          detail: `HTTP ${result.status} (${result.body.length} bytes body)`,
        },
      }));
    } catch (e) {
      setTestResults(prev => ({
        ...prev,
        [id]: { label: 'http_request', ok: false, detail: String(e) },
      }));
    } finally {
      setBusyId(null);
    }
  }, []);

  const clearTestResult = useCallback((id: string) => {
    setTestResults(prev => {
      if (!(id in prev)) return prev;
      const next = { ...prev };
      delete next[id];
      return next;
    });
  }, []);

  const clearError = useCallback(() => setError(null), []);

  return {
    plugins,
    loading,
    error,
    busyId,
    testResults,
    backendReady,
    refresh,
    enable,
    disable,
    reload,
    runNoteReadTest,
    runHttpRequestTest,
    clearTestResult,
    clearError,
  };
}
