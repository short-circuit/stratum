import { useCallback, useEffect, useState } from 'react';
import {
  fetchThemeCatalog,
  getActiveThemeId,
  listInstalledThemes,
  removeInstalledTheme,
  saveInstalledTheme,
  setActiveThemeId,
  verifySha256,
} from '../../lib/pluginCatalog';
import type { ThemeCatalogEntry } from '../../lib/types';
import { useStore } from '../../stores/appStore';
import { useSettingsStore } from '../../stores/settingsStore';
import { applyTheme } from '../../lib/theme';
import { useTranslation } from 'react-i18next';

export interface InstalledThemeInfo {
  entry: ThemeCatalogEntry;
  active: boolean;
}

/** Built-in fallback theme values (matches settingsStore DEFAULT_THEME). */
const BUILTIN_THEME = {
  dark_mode: true,
  primary_color: '#f97316',
  secondary_color: '#6b7280',
  font_size: 16,
};

/**
 * E10.F7 — Theme Hub tab state. Themes are small JSON documents downloaded
 * from the theme catalog; they are stored verbatim in localStorage (installed
 * list + active id) so the hub works offline after first install. Applying a
 * theme writes both the CSS variables (applyTheme) and the persisted settings
 * theme so the Settings tab and the hub stay consistent.
 */
export function useThemeHub(onError: (message: string) => void, active: boolean) {
  const { t } = useTranslation();
  const setThemeConfig = useStore(s => s.setThemeConfig);
  const settingsStore = useSettingsStore();

  const [catalogThemes, setCatalogThemes] = useState<ThemeCatalogEntry[]>([]);
  const [themesLoading, setThemesLoading] = useState(false);
  const [themesError, setThemesError] = useState<string | null>(null);
  const [themesOffline, setThemesOffline] = useState(false);
  const [themesFetchedAt, setThemesFetchedAt] = useState<string | null>(null);
  const [installedThemes, setInstalledThemes] = useState<InstalledThemeInfo[]>([]);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [seeded, setSeeded] = useState(false);

  const syncInstalled = useCallback(() => {
    const active = getActiveThemeId();
    setInstalledThemes(listInstalledThemes().map(entry => ({ entry, active: entry.id === active })));
  }, []);

  const refresh = useCallback(async () => {
    setThemesLoading(true);
    setThemesError(null);
    try {
      const result = await fetchThemeCatalog();
      setCatalogThemes(result.entries);
      setThemesOffline(result.offline);
      setThemesFetchedAt(result.fetchedAt);
      if (result.error && result.offline) {
        setThemesError(`${t('plugins.catalog_offline')} (${result.error})`);
      } else if (result.error) {
        setThemesError(`${t('plugins.catalog_error')} ${result.error}`);
      }
    } finally {
      setThemesLoading(false);
      syncInstalled();
    }
  }, [t, syncInstalled]);

  // Fetch the catalog the first time the Themes tab is opened; never on
  // panel mount (keeps the Installed tab network-free).
  useEffect(() => {
    if (active && !seeded) {
      setSeeded(true);
      void refresh();
    }
  }, [active, seeded, refresh]);

  /** Apply entry colors to CSS vars, MUI theme config, and persisted settings. */
  const applyEntry = useCallback(
    (entry: ThemeCatalogEntry) => {
      applyTheme(entry.primary_color, entry.secondary_color, entry.dark_mode, entry.font_size);
      setThemeConfig({
        primaryHex: entry.primary_color,
        secondaryHex: entry.secondary_color,
        dark: entry.dark_mode,
        fontSize: entry.font_size,
      });
      settingsStore.updateTheme({
        dark_mode: entry.dark_mode,
        primary_color: entry.primary_color,
        secondary_color: entry.secondary_color,
        font_size: entry.font_size,
      });
    },
    [setThemeConfig, settingsStore],
  );

  const install = useCallback(
    async (entry: ThemeCatalogEntry) => {
      setBusyId(entry.id);
      setThemesError(null);
      try {
        const res = await fetch(entry.download_url);
        if (!res.ok) throw new Error(`catalog_http_${res.status}`);
        const text = await res.text();
        const theme = JSON.parse(text) as Partial<ThemeCatalogEntry>;
        if (typeof theme.dark_mode !== 'boolean' && typeof theme.primary_color !== 'string') {
          throw new Error(t('plugins.catalog_schema_invalid'));
        }
        if (entry.sha256) {
          const bytes = new TextEncoder().encode(text);
          if (!(await verifySha256(bytes, entry.sha256))) {
            throw new Error(`${t('plugins.checksum_mismatch')} (${entry.id})`);
          }
        }
        saveInstalledTheme(entry);
        setActiveThemeId(entry.id);
        applyEntry({
          ...entry,
          dark_mode: theme.dark_mode ?? entry.dark_mode,
          primary_color: theme.primary_color ?? entry.primary_color,
          secondary_color: theme.secondary_color ?? entry.secondary_color,
          font_size: theme.font_size ?? entry.font_size,
        });
        syncInstalled();
      } catch (e) {
        onError(`${t('plugins.install_failed')} ${entry.id}: ${String(e)}`);
      } finally {
        setBusyId(null);
      }
    },
    [t, applyEntry, syncInstalled, onError],
  );

  const uninstall = useCallback(
    async (id: string) => {
      setBusyId(id);
      setThemesError(null);
      const remaining = removeInstalledTheme(id);
      if (getActiveThemeId() === id) {
        // Removing the active theme falls back to the built-in theme.
        setActiveThemeId(null);
        applyEntry({
          ...BUILTIN_THEME, id: '__builtin__', name: 'Built-in', version: '1.0.0', description: 'Built-in theme', repo: '', sha256: '', download_url: '',
        });
      }
      setInstalledThemes(remaining.map(entry => ({ entry, active: false })));
      setBusyId(null);
    },
    [applyEntry],
  );

  const apply = useCallback(
    (entry: ThemeCatalogEntry) => {
      setActiveThemeId(entry.id);
      applyEntry(entry);
      syncInstalled();
    },
    [applyEntry, syncInstalled],
  );

  return {
    catalogThemes,
    themesLoading,
    themesError,
    themesOffline,
    themesFetchedAt,
    installedThemes,
    busyId,
    refresh,
    install,
    uninstall,
    apply,
    setSeeded,
  };
}
