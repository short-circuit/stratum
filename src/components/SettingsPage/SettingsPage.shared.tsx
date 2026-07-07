//! Shared state, types, and logic for SettingsPage.
//! Provides the `useSettingsPage()` hook consumed by both desktop and mobile variants.

import { useState, useEffect, useRef, useCallback } from 'react';
import type { SyncStatusDto, CommitLogEntry } from '../../lib/types';
import { useStore } from '../../stores/appStore';
import { useSyncModalStore } from '../../stores/syncModalStore';
import * as api from '../../lib/commands';
import { applyTheme } from '../../lib/theme';

export type SettingsTab = 'vault' | 'theme' | 'ai' | 'research' | 'developer' | 'sync';

export interface SettingsData {
  vault_path?: string;
  ai?: {
    provider: string;
    endpoint: string | null;
    api_key: string | null;
    model: string;
    models: { name: string; capabilities: string[] }[];
    rag_enabled: boolean;
    rag_chunk_count: number;
  };
  research?: {
    searxng_endpoint: string;
    max_results: number;
    max_depth: number;
  };
  theme?: {
    dark_mode: boolean;
    primary_color: string;
    secondary_color: string;
    font_size: number;
  };
  sync?: {
    mode: string;
    remote_url: string | null;
    branch: string;
    auto_commit_interval_secs: number;
    auto_sync_interval_secs: number;
    ssh_key_path: string | null;
    commit_template: string;
  };
  [key: string]: unknown;
}

export function useSettingsPage() {
  const { pickVaultDirectory, setThemeConfig } = useStore();
  const [settings, setSettings] = useState<any>(null);
  const [saving, setSaving] = useState(false);
  const [fetching, setFetching] = useState(false);
  const [msg, setMsg] = useState('');
  const [msgSeverity, setMsgSeverity] = useState<'success' | 'error'>('success');
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [tab, setTab] = useState<SettingsTab>('vault');
  const savedThemeRef = useRef<{
    primary: string;
    secondary: string;
    dark: boolean;
    fontSize: number;
  } | null>(null);
  const muiSyncTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [syncStatus, setSyncStatus] = useState<SyncStatusDto | null>(null);
  const [commits, setCommits] = useState<CommitLogEntry[]>([]);
  const [commitsOpen, setCommitsOpen] = useState(false);
  const [syncing, setSyncing] = useState(false);

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
        setMsg(`Load failed: ${err}`);
        setMsgSeverity('error');
      });

    api.getSyncStatus().then(setSyncStatus).catch(() => {});
    api.getCommitLog().then(setCommits).catch(() => {});

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
      setMsg('Saved.');
      setMsgSeverity('success');
    } catch (e) {
      setMsg(`Save failed: ${e}`);
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
      setMsg(`Found ${models.length} models`);
      setMsgSeverity('success');
    } catch (e) {
      setMsg(`Fetch failed: ${e}`);
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
      const count = await api.reindexVault();
      setMsg(`Reindexed ${count} pages.`);
    } catch (e) {
      setMsg(`Reindex failed: ${e}`);
      setMsgSeverity('error');
    } finally {
      setFetching(false);
    }
  };

  const handleSyncNow = async () => {
    setSyncing(true);
    setMsg('');
    try {
      const result = await api.syncVault();
      setSyncStatus(result);
      if (result.status === 'conflicts') {
        syncModal.showConflictModal(result.conflicts);
      }
      setMsg(
        result.status === 'ok'
          ? 'Sync completed.'
          : result.status === 'conflicts'
            ? `Conflicts in ${result.conflicts.length} file(s).`
            : 'Sync had errors.'
      );
    } catch (e) {
      const errStr = String(e);
      if (errStr.includes('NeedsPassphrase') || errStr.includes('passphrase')) {
        syncModal.showPassphraseModal();
      } else {
        setMsg(`Sync failed: ${e}`);
      }
    } finally {
      setSyncing(false);
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
      api.getCommitLog().then(setCommits).catch(() => {});
    }
    setCommitsOpen(!commitsOpen);
  };

  const handleStartScheduler = async () => {
    try {
      await api.startSyncScheduler();
      setMsg('Scheduler started.');
      setMsgSeverity('success');
    } catch (e) {
      setMsg(`Scheduler error: ${e}`);
      setMsgSeverity('error');
    }
  };

  const handlePassphraseSubmit = async (passphrase: string) => {
    setSyncing(true);
    try {
      const result = await api.syncVaultWithPassphrase(passphrase);
      setSyncStatus(result);
      syncModal.hidePassphraseModal();
      if (result.status === 'conflicts') {
        syncModal.showConflictModal(result.conflicts);
      }
      setMsg(
        result.status === 'ok'
          ? 'Sync completed.'
          : result.status === 'conflicts'
            ? `Conflicts in ${result.conflicts.length} file(s).`
            : 'Sync had errors.'
      );
    } catch (e) {
      setMsg(`Sync failed: ${e}`);
    } finally {
      setSyncing(false);
    }
  };

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

    // Derived
    ai,
    research,
    theme,
    syncSettings,

    // Actions
    setMsg,
    setMsgSeverity,
    setSyncing,
    updateAi,
    updateVault,
    updateResearch,
    updateTheme,
    updateSync,
    handleSave,
    handleFetchModels,
    handleReindex,
    handleSyncNow,
    toggleModelCapability,
    handleToggleCommits,
    handleStartScheduler,
    handlePassphraseSubmit,
    pickVaultDirectory,
  };
}
