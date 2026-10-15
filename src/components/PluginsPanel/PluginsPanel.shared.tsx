import { useCallback, useEffect, useState } from 'react';
import * as api from '../../lib/commands';
import { isPluginBackendReady } from '../../lib/commands/plugins';
import type { PluginCatalogEntry, PluginHttpRequestParams, PluginInfoDto, ThemeCatalogEntry } from '../../lib/types';
import { useTranslation } from 'react-i18next';
import { usePluginDirectory } from './usePluginDirectory';
import { useThemeHub, type InstalledThemeInfo } from './useThemeHub';

export interface PluginTestResult {
  label: string;
  ok: boolean;
  detail: string;
}

export type PluginsTab = 'installed' | 'directory' | 'themes';

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
  installFromFile: () => Promise<void>;
  uninstall: (id: string) => Promise<void>;
  runNoteReadTest: (id: string) => Promise<void>;
  runHttpRequestTest: (id: string) => Promise<void>;
  clearTestResult: (id: string) => void;
  clearError: () => void;
  // ── E10.F7: Plugin Directory / Theme Hub ──
  tab: PluginsTab;
  setTab: (tab: PluginsTab) => void;
  catalogPlugins: PluginCatalogEntry[];
  catalogLoading: boolean;
  catalogError: string | null;
  catalogOffline: boolean;
  catalogFetchedAt: string | null;
  installedIds: Set<string>;
  catalogBusyId: string | null;
  refreshCatalog: () => Promise<void>;
  installCatalogPlugin: (entry: PluginCatalogEntry) => Promise<void>;
  uninstallById: (id: string) => Promise<void>;
  catalogThemes: ThemeCatalogEntry[];
  themesLoading: boolean;
  themesError: string | null;
  themesOffline: boolean;
  themesFetchedAt: string | null;
  installedThemes: InstalledThemeInfo[];
  themesBusyId: string | null;
  refreshThemes: () => Promise<void>;
  installTheme: (entry: ThemeCatalogEntry) => Promise<void>;
  uninstallTheme: (id: string) => Promise<void>;
  applyThemeById: (entry: ThemeCatalogEntry) => void;
}

export function usePluginsPanel(): PluginsPanelState {
  const { t } = useTranslation();
  const [plugins, setPlugins] = useState<PluginInfoDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<Record<string, PluginTestResult | null>>({});
  const [backendReady] = useState<boolean>(() => isPluginBackendReady());
  const [tab, setTab] = useState<PluginsTab>('installed');

  const setErrorMaybe = useCallback((message: string) => setError(message), []);
  const setPluginsFromList = useCallback((list: PluginInfoDto[]) => setPlugins(list), []);

  // E10.F7 sub-hooks: directory + theme hub. Their error surfaces share the
  // panel error banner; installed-plugin list mutations flow into the parent.
  const directory = usePluginDirectory(plugins, setPluginsFromList, setErrorMaybe, tab === 'directory');
  const themeHub = useThemeHub(setErrorMaybe, tab === 'themes');

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
        setError(`${t('plugins.failed_on')} ${id}: ${String(e)}`);
      } finally {
        setBusyId(null);
      }
    },
    [t],
  );

  const enable = useCallback((id: string) => runAction(id, api.pluginsEnable), [runAction]);
  const disable = useCallback((id: string) => runAction(id, api.pluginsDisable), [runAction]);
  const reload = useCallback((id: string) => runAction(id, api.pluginsReload), [runAction]);

  const installFromFile = useCallback(async () => {
    setError(null);
    setBusyId('__install__');
    try {
      const installed = await api.pluginsInstallFromFile();
      if (installed) {
        setPlugins(prev =>
          prev.some(p => p.id === installed.id)
            ? prev.map(p => (p.id === installed.id ? installed : p))
            : [...prev, installed],
        );
      }
    } catch (e) {
      setError(`${t('plugins.install_failed')} ${String(e)}`);
    } finally {
      setBusyId(null);
    }
  }, [t]);

  const uninstall = useCallback(
    async (id: string) => {
      setBusyId(id);
      setError(null);
      try {
        const result = await api.pluginsUninstall(id);
        setPlugins(result.plugins);
      } catch (e) {
        setError(`${t('plugins.uninstall_failed_on')} ${id}: ${String(e)}`);
      } finally {
        setBusyId(null);
      }
    },
    [t],
  );

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
    installFromFile,
    uninstall,
    runNoteReadTest,
    runHttpRequestTest,
    clearTestResult,
    clearError,
    tab,
    setTab,
    catalogPlugins: directory.catalogPlugins,
    catalogLoading: directory.catalogLoading,
    catalogError: directory.catalogError,
    catalogOffline: directory.catalogOffline,
    catalogFetchedAt: directory.catalogFetchedAt,
    installedIds: directory.installedIds,
    catalogBusyId: directory.busyId,
    refreshCatalog: directory.refresh,
    installCatalogPlugin: directory.install,
    uninstallById: directory.uninstall,
    catalogThemes: themeHub.catalogThemes,
    themesLoading: themeHub.themesLoading,
    themesError: themeHub.themesError,
    themesOffline: themeHub.themesOffline,
    themesFetchedAt: themeHub.themesFetchedAt,
    installedThemes: themeHub.installedThemes,
    themesBusyId: themeHub.busyId,
    refreshThemes: themeHub.refresh,
    installTheme: themeHub.install,
    uninstallTheme: themeHub.uninstall,
    applyThemeById: themeHub.apply,
  };
}
