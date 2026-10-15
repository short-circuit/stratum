import { useCallback, useEffect, useState } from 'react';
import { fetchPluginCatalog, verifySha256 } from '../../lib/pluginCatalog';
import type { PluginCatalogEntry, PluginInfoDto } from '../../lib/types';
import * as api from '../../lib/commands/plugins';
import { useTranslation } from 'react-i18next';

/** Error message prefix shared by the directory tab handlers. */
function describeError(payload: unknown): string {
  return payload instanceof Error ? payload.message : String(payload);
}

/**
 * E10.F7 — Plugin Directory tab state: catalog fetch (cached, offline-safe),
 * sha256-verified catalog install, and re-use of the existing uninstall path.
 * The parent hook supplies the live installed-plugin list so the directory
 * can mark entries already present.
 */
export function usePluginDirectory(
  installedPlugins: PluginInfoDto[],
  onInstalledChanged: (plugins: PluginInfoDto[]) => void,
  onError: (message: string) => void,
  /** When true (directory tab selected) the catalog is fetched on first open. */
  active: boolean,
) {
  const { t } = useTranslation();
  const [catalogPlugins, setCatalogPlugins] = useState<PluginCatalogEntry[]>([]);
  const [catalogLoading, setCatalogLoading] = useState(false);
  const [catalogError, setCatalogError] = useState<string | null>(null);
  const [catalogOffline, setCatalogOffline] = useState(false);
  const [catalogFetchedAt, setCatalogFetchedAt] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [seeded, setSeeded] = useState(false);

  const installedIds = new Set(installedPlugins.map(p => p.id));

  const refresh = useCallback(async () => {
    setCatalogLoading(true);
    setCatalogError(null);
    try {
      const result = await fetchPluginCatalog();
      setCatalogPlugins(result.entries);
      setCatalogOffline(result.offline);
      setCatalogFetchedAt(result.fetchedAt);
      if (result.error && result.offline) {
        setCatalogError(`${t('plugins.catalog_offline')} (${result.error})`);
      } else if (result.error) {
        setCatalogError(`${t('plugins.catalog_error')} ${result.error}`);
      }
    } finally {
      setCatalogLoading(false);
    }
  }, [t]);

  // Fetch the catalog the first time the directory tab is opened; never on
  // panel mount (keeps the Installed tab network-free).
  useEffect(() => {
    if (active && !seeded) {
      setSeeded(true);
      void refresh();
    }
  }, [active, seeded, refresh]);

  const install = useCallback(
    async (entry: PluginCatalogEntry) => {
      setBusyId(entry.id);
      setCatalogError(null);
      try {
        // Verify the artifact digest BEFORE handing bytes to the backend so a
        // tampered/mismatched download never lands in the vault.
        if (entry.sha256) {
          const res = await fetch(entry.download_url);
          if (!res.ok) throw new Error(`catalog_http_${res.status}`);
          const bytes = new Uint8Array(await res.arrayBuffer());
          if (!(await verifySha256(bytes, entry.sha256))) {
            throw new Error(`${t('plugins.checksum_mismatch')} (${entry.id})`);
          }
        }
        const installed = await api.pluginsInstallUrl(entry.download_url, entry.sha256, entry.id);
        onInstalledChanged(
          installedPlugins.some(p => p.id === installed.id)
            ? installedPlugins.map(p => (p.id === installed.id ? installed : p))
            : [...installedPlugins, installed],
        );
      } catch (e) {
        onError(`${t('plugins.install_failed')} ${entry.id}: ${describeError(e)}`);
      } finally {
        setBusyId(null);
      }
    },
    [t, installedPlugins, onInstalledChanged, onError],
  );

  const uninstall = useCallback(
    async (id: string) => {
      setBusyId(id);
      setCatalogError(null);
      try {
        const result = await api.pluginsUninstall(id);
        onInstalledChanged(result.plugins);
      } catch (e) {
        onError(`${t('plugins.uninstall_failed_on')} ${id}: ${describeError(e)}`);
      } finally {
        setBusyId(null);
      }
    },
    [t, onInstalledChanged, onError],
  );

  return {
    catalogPlugins,
    catalogLoading,
    catalogError,
    catalogOffline,
    catalogFetchedAt,
    installedIds,
    busyId,
    refresh,
    install,
    uninstall,
    setSeeded,
  };
}
