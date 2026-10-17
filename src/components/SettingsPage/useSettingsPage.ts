//! Shared state, types, and logic for SettingsPage.
//! Provides the `useSettingsPage()` hook consumed by both desktop and mobile variants.
//! Extracted from SettingsPage.shared.tsx during the E6 sizing-gate refactor.

import { useState, useEffect, useRef, useCallback } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { useStore } from '../../stores/appStore';
import { useSyncModalStore } from '../../stores/syncModalStore';
import { useSyncStore } from '../../stores/syncStore';
import * as api from '../../lib/commands';
import { applyTheme } from '../../lib/theme';
import i18n, { setLocale } from '../../i18n';
import type { SettingsTab } from './settingsTypes';

export type { SettingsTab, SettingsData } from './settingsTypes';

const DEFAULT_BACKUP = {
  enabled: false,
  mode: 'folder',
  interval_minutes: 60,
  exclude: ['.git', '.pkm/blocks.db', '.pkm/search', '.pkm/backup_state.json', '.pkm/sync_state.json', '.DS_Store'],
  webdav_url: null,
  webdav_username: null,
  webdav_password: null,
  folder_path: null,
};

export function useSettingsPage() {
  const { pickVaultDirectory, setThemeConfig, loadPages } = useStore(useShallow(
    s => ({ pickVaultDirectory: s.pickVaultDirectory, setThemeConfig: s.setThemeConfig, loadPages: s.loadPages }),
  ));
  const [settings, setSettings] = useState<any>(null);
  const [saving, setSaving] = useState(false);
  const [fetching, setFetching] = useState(false);
  const [msg, setMsg] = useState('');
  const [msgSeverity, setMsgSeverity] = useState<'success' | 'error'>('success');
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [tab, setTab] = useState<SettingsTab>('vault');
  const [reindexProgress, setReindexProgress] = useState<{ message: string; percent: number } | null>(null);
  const savedThemeRef = useRef<{
    primary: string;
    secondary: string;
    dark: boolean;
    fontSize: number;
  } | null>(null);
  const muiSyncTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const syncStatus = useSyncStore(s => s.syncStatus);
  const commits = useSyncStore(s => s.commits);
  const syncing = useSyncStore(s => s.syncing);
  const setSyncing = useSyncStore(s => s.setSyncing);
  const [commitsOpen, setCommitsOpen] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [backupStatus, setBackupStatus] = useState<any>(null);
  const [testing, setTesting] = useState(false);
  const [running, setRunning] = useState(false);
  const [webdavTestResult, setWebdavTestResult] = useState<string | null>(null);

  const syncModal = useSyncModalStore();

  const syncMuiTheme = useCallback(
    (primary: string, secondary: string, dark: boolean, fontSize: number) => {
      if (muiSyncTimer.current) clearTimeout(muiSyncTimer.current);
      muiSyncTimer.current = setTimeout(() => {
        setThemeConfig({ primaryHex: primary, secondaryHex: secondary, dark, fontSize });
      }, 150);
    },
    [setThemeConfig]
  );

  useEffect(() => {
    api
      .getSettings()
      .then(s => {
        setSettings(s);
        if (s.theme) {
          const p = s.theme.primary_color || '#f97316';
          const sec = s.theme.secondary_color || '#6b7280';
          const d = s.theme.dark_mode ?? true;
          const f = s.theme.font_size || 16;
          savedThemeRef.current = { primary: p, secondary: sec, dark: d, fontSize: f };
          applyTheme(p, sec, d, f);
          setThemeConfig({ primaryHex: p, secondaryHex: sec, dark: d, fontSize: f });
        }
      })
      .catch(err => {
        setMsg(`${i18n.t('settings.load_failed')} ${err}`);
        setMsgSeverity('error');
      });

    useSyncStore.getState().fetchSyncStatus();
    useSyncStore.getState().fetchCommitLog();
    api.getBackupStatus().then(setBackupStatus).catch(() => {});

    return () => {
      const saved = savedThemeRef.current;
      if (saved) {
        applyTheme(saved.primary, saved.secondary, saved.dark, saved.fontSize);
        setThemeConfig({
          primaryHex: saved.primary,
          secondaryHex: saved.secondary,
          dark: saved.dark,
          fontSize: saved.fontSize,
        });
      }
      if (muiSyncTimer.current) clearTimeout(muiSyncTimer.current);
    };
  }, [setThemeConfig]);

  // Listen for reindex progress events (throttled to prevent UI thrashing)
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let lastUpdate = 0;
    import('@tauri-apps/api/event').then(({ listen }) => {
      listen<{ message: string; percent: number }>('reindex-progress', (event) => {
        const now = Date.now();
        // Only update UI every 200ms to avoid thrashing with many files
        if (event.payload.percent < 1.0 && now - lastUpdate < 200) return;
        lastUpdate = now;
        setReindexProgress(event.payload);
        if (event.payload.percent >= 1.0) {
          // Final update always goes through
          setReindexProgress(event.payload);
        }
      }).then((fn) => { unlisten = fn; });
    });
    return () => { unlisten?.(); };
  }, []);

  // Derived settings slices with defaults
  const ai = settings?.ai;
  const research = settings?.research || {
    searxng_endpoint: 'http://localhost:8888',
    max_results: 3,
    max_depth: 2,
  };
  const theme = settings?.theme || {
    dark_mode: true,
    primary_color: '#f97316',
    secondary_color: '#6b7280',
    font_size: 16,
  };
  const syncSettings = settings?.sync || {
    mode: 'manual',
    remote_url: null,
    branch: 'main',
    auto_commit_interval_secs: 300,
    auto_sync_interval_secs: 1800,
    ssh_key_path: null,
    commit_template:
      'stratum({datetime}): {editedfiles} edited, {newfiles} added, {deletedfiles} deleted',
  };
  const tts = settings?.tts || {
    endpoint: '',
    api_key: null,
    voice: 'alloy',
    format: 'mp3',
    speed: 1.0,
    use_llm_gateway_and_auth: false,
  };
  const tray = settings?.tray || {
    enabled: true,
    close_to_tray: true,
    quick_capture_shortcut: 'CmdOrCtrl+Shift+Space',
    capture_target: 'journal',
    inbox_path: 'Inbox.md',
  };
  const voice = settings?.voice || { enabled: true, target: 'journal:today' };
  const backup = settings?.sync?.backup || DEFAULT_BACKUP;
  const reminders = settings?.reminders || { enabled: true, sound: true };
  const layout = settings?.layout || { pages_view: 'flat' };

  const updateAi = (patch: any) => setSettings({ ...settings, ai: { ...ai, ...patch } });
  const updateVault = (patch: any) => setSettings({ ...settings, ...patch });
  const updateResearch = (patch: any) =>
    setSettings({ ...settings, research: { ...research, ...patch } });
  const updateTheme = (patch: any) => {
    const newTheme = { ...theme, ...patch };
    setSettings({ ...settings, theme: newTheme });
    applyTheme(newTheme.primary_color, newTheme.secondary_color, newTheme.dark_mode, newTheme.font_size);
    syncMuiTheme(
      newTheme.primary_color,
      newTheme.secondary_color,
      newTheme.dark_mode,
      newTheme.font_size || 16
    );
  };
  const updateSync = (patch: any) =>
    setSettings({ ...settings, sync: { ...syncSettings, ...patch } });
  const updateBackup = (patch: any) =>
    setSettings({
      ...settings,
      sync: { ...syncSettings, backup: { ...(syncSettings.backup || DEFAULT_BACKUP), ...patch } },
    });
  const updateStt = (patch: any) =>
    setSettings({ ...settings, stt: { ...(settings.stt || {}), ...patch } });
  const updateTts = (patch: any) =>
    setSettings({ ...settings, tts: { ...(settings.tts || {}), ...patch } });
  const updateTray = (patch: any) =>
    setSettings({ ...settings, tray: { ...(settings.tray || {}), ...patch } });
  const updateVoice = (patch: any) =>
    setSettings({ ...settings, voice: { ...(settings.voice || {}), ...patch } });
  const updateReminders = (patch: any) =>
    setSettings({ ...settings, reminders: { ...(settings.reminders || {}), ...patch } });
  const updateLayout = (patch: any) =>
    setSettings({ ...settings, layout: { ...layout, ...patch } });
  const locale = settings?.locale ?? 'en';
  const onLocaleChange = (loc: string) => {
    setSettings({ ...settings, locale: loc });
    void setLocale(loc);
    api.saveSettings({ ...settings, locale: loc }).catch(() => {});
  };

  const handleSave = async () => {
    setSaving(true);
    setMsg('');
    try {
      await api.saveSettings(settings);
      const p = theme.primary_color;
      const sec = theme.secondary_color;
      const d = theme.dark_mode;
      const f = theme.font_size || 16;
      savedThemeRef.current = { primary: p, secondary: sec, dark: d, fontSize: f };
      if (muiSyncTimer.current) clearTimeout(muiSyncTimer.current);
      setThemeConfig({ primaryHex: p, secondaryHex: sec, dark: d, fontSize: f });
      setMsg(i18n.t('settings.saved'));
      setMsgSeverity('success');
    } catch (e) {
      setMsg(`${i18n.t('settings.save_failed')} ${e}`);
      setMsgSeverity('error');
    } finally {
      setSaving(false);
    }
  };

  const handleFetchModels = async () => {
    setFetching(true);
    setMsg('');
    try {
      const models = await api.fetchModels();
      setAvailableModels(models);
      setMsg(i18n.t('settings.found_models', { count: models.length }));
      setMsgSeverity('success');
    } catch (e) {
      setMsg(`${i18n.t('settings.fetch_failed')} ${e}`);
      setMsgSeverity('error');
    } finally {
      setFetching(false);
    }
  };

  const handleReindex = async () => {
    setFetching(true);
    setMsg('');
    setMsgSeverity('success');
    try {
      const result = await api.reindexVault();
      if (result.failed > 0) {
        setMsg(`${i18n.t('settings.reindexed')} ${result.succeeded} ok, ${result.failed} failed out of ${result.processed}`);
        setMsgSeverity('error');
      } else {
        setMsg(i18n.t('settings.reindexed_count', { count: result.succeeded }));
      }
      await loadPages();
    } catch (e) {
      setMsg(`${i18n.t('page.reindex_failed')} ${e}`);
      setMsgSeverity('error');
    } finally {
      setFetching(false);
    }
  };

  const handleRepair = async () => {
    setFetching(true);
    setMsg('');
    setMsgSeverity('success');
    try {
      const result = await api.repairDbFromDisk();
      if (result.failed > 0) {
        setMsg(`${i18n.t('settings.repaired')} ${result.succeeded} ok, ${result.failed} failed out of ${result.processed}`);
        setMsgSeverity('error');
      } else {
        setMsg(i18n.t('settings.repaired_count', { count: result.succeeded }));
      }
      await loadPages();
    } catch (e) {
      setMsg(`${i18n.t('settings.repair_failed')} ${e}`);
      setMsgSeverity('error');
    } finally {
      setFetching(false);
    }
  };

  const handleNormalizeAll = useCallback(async () => {
    setFetching(true);
    setMsg('');
    setMsgSeverity('success');
    try {
      const count = await api.normalizeAllFiles();
      setMsg(i18n.t('settings.normalized_count', { count }));
    } catch (e) {
      setMsg(`${i18n.t('page.normalize_failed')} ${e}`);
      setMsgSeverity('error');
    } finally {
      setFetching(false);
    }
  }, []);

  const handleSyncNow = async () => {
    setMsg('');
    try {
      const result = await useSyncStore.getState().doSync();
      if (result.status === 'conflicts') {
        syncModal.showConflictModal(result.conflicts);
      }
      setMsg(
        result.status === 'ok'
          ? i18n.t('settings.sync_completed')
          : result.status === 'conflicts'
            ? `${i18n.t('settings.conflicts_in')} ${result.conflicts.length} file(s).`
            : i18n.t('settings.sync_had_errors')
      );
    } catch (e) {
      const errStr = String(e);
      if (errStr.includes('NeedsPassphrase') || errStr.includes('passphrase')) {
        syncModal.showPassphraseModal();
      } else {
        setMsg(`${i18n.t('settings.sync_failed')} ${e}`);
      }
    }
  };

  const runBackupNow = async () => {
    setRunning(true);
    setMsg('');
    setMsgSeverity('success');
    try {
      const status = await api.runBackup();
      setBackupStatus(status);
      setMsg(
        status.last_error
          ? `${i18n.t('settings.backup_failed')} ${status.last_error}`
          : i18n.t('settings.backup_success')
      );
      if (status.last_error) setMsgSeverity('error');
    } catch (e) {
      setBackupStatus({ last_success: false, last_error: String(e) });
      setMsg(`${i18n.t('settings.backup_failed')} ${e}`);
      setMsgSeverity('error');
    } finally {
      setRunning(false);
    }
  };

  const testWebdavConnection = async () => {
    if (!backup.webdav_url) return;
    setTesting(true);
    setWebdavTestResult(null);
    try {
      const result = await api.testWebdav(
        backup.webdav_url,
        backup.webdav_username,
        backup.webdav_password
      );
      setWebdavTestResult(result);
    } catch (e) {
      setWebdavTestResult(String(e));
    } finally {
      setTesting(false);
    }
  };

  const toggleModelCapability = (modelName: string, cap: string) => {
    const models = [...(ai.models || [])];
    const existing = models.find((m: any) => m.name === modelName);
    if (existing) {
      existing.capabilities = existing.capabilities.includes(cap)
        ? existing.capabilities.filter((c: string) => c !== cap)
        : [...existing.capabilities, cap];
    } else {
      models.push({ name: modelName, capabilities: [cap] });
    }
    updateAi({ models: models.filter((m: any) => m.capabilities.length > 0) });
  };

  const handleToggleCommits = () => {
    if (!commitsOpen && commits.length === 0) {
      useSyncStore.getState().fetchCommitLog();
    }
    setCommitsOpen(!commitsOpen);
  };

  const handleStartScheduler = async () => {
    try {
      await api.startSyncScheduler();
      setMsg(i18n.t('settings.scheduler_started'));
      setMsgSeverity('success');
    } catch (e) {
      setMsg(`${i18n.t('settings.scheduler_error')} ${e}`);
      setMsgSeverity('error');
    }
  };

  const handleResetOnboarding = async () => {
    setMsg('');
    setMsgSeverity('success');
    try {
      await api.setOnboardingPending(true);
      setMsg(i18n.t('settings.onboarding_reset_done'));
    } catch (e) {
      setMsg(`${i18n.t('settings.reset_onboarding_help')} ${e}`);
      setMsgSeverity('error');
    }
  };

  const handleInstallStarterTemplates = async () => {
    setInstalling(true);
    setMsg('');
    setMsgSeverity('success');
    try {
      const names = await api.installStarterTemplates();
      setMsg(i18n.t('settings.templates_installed', { count: names.length }));
    } catch (e) {
      setMsg(`${i18n.t('settings.install_starter_templates')} ${e}`);
      setMsgSeverity('error');
    } finally {
      setInstalling(false);
    }
  };

  const handlePassphraseSubmit = async (passphrase: string) => {
    try {
      const result = await useSyncStore.getState().doSyncWithPassphrase(passphrase);
      syncModal.hidePassphraseModal();
      if (result.status === 'conflicts') {
        syncModal.showConflictModal(result.conflicts);
      }
      setMsg(
        result.status === 'ok'
          ? i18n.t('settings.sync_completed')
          : result.status === 'conflicts'
            ? `${i18n.t('settings.conflicts_in')} ${result.conflicts.length} file(s).`
            : i18n.t('settings.sync_had_errors')
      );
    } catch (e) {
      setMsg(`${i18n.t('settings.sync_failed')} ${e}`);
    }
  };

  // Wrap the store's folder-pick action so the Settings form reflects the newly
  // selected vault path immediately.
  //
  // The backend `pick_vault_directory` / `pick_android_directory` commands return
  // a `VaultInfo` and update the in-memory `VaultState.vault_path`, and
  // `appStore.pickVaultDirectory` persists that into the zustand `vault`. The
  // local `settings` state (fetched once on mount) is stale after a pick, and a
  // naive re-fetch of `get_settings` can still return the OLD path when a stale
  // `.pkm/config.toml` exists in the previous vault. So the picked path is read
  // from the store (the authoritative source) and merged into `settings`.
  const handlePickVaultDirectory = useCallback(async () => {
    try {
      await pickVaultDirectory();
      const pickedPath = useStore.getState().vault?.path;
      if (pickedPath) {
        setSettings((prev: any) =>
          prev ? { ...prev, vault_path: pickedPath } : prev
        );
      }
    } catch {
      // The store action surfaces errors via `useStore`'s `error` field; nothing
      // to update in the form when the pick is cancelled or fails.
    }
  }, [pickVaultDirectory]);

  return {
    // State
    settings,
    saving,
    fetching,
    msg,
    msgSeverity,
    availableModels,
    tab,
    setTab,
    syncStatus,
    commits,
    commitsOpen,
    syncing,
    backupStatus,
    testing,
    running,
    webdavTestResult,

    // Derived
    ai,
    research,
    theme,
    syncSettings,
    backup,
    stt: settings?.stt,
    tts,
    tray,
    voice,
    reminders,
    layout,
    locale,
    onLocaleChange,

    // Actions
    setMsg,
    setMsgSeverity,
    setSyncing,
    updateAi,
    updateVault,
    updateResearch,
    updateTheme,
    updateSync,
    updateBackup,
    runBackupNow,
    testWebdavConnection,
    updateStt,
    updateTts,
    updateTray,
    updateVoice,
    updateReminders,
    updateLayout,
    handleSave,
    handleFetchModels,
    handleReindex,
    handleRepair,
    handleNormalizeAll,
    reindexProgress,
    setReindexProgress,
    handleSyncNow,
    toggleModelCapability,
    handleToggleCommits,
    handleStartScheduler,
    handlePassphraseSubmit,
    handlePickVaultDirectory,
    handleResetOnboarding,
    handleInstallStarterTemplates,
    installing,
  };
}
