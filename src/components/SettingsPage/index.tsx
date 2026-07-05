import { useState, useEffect, useRef, useCallback } from 'react';
import Box from '@mui/material/Box';
import Tabs from '@mui/material/Tabs';
import Tab from '@mui/material/Tab';
import Button from '@mui/material/Button';
import Alert from '@mui/material/Alert';
import Typography from '@mui/material/Typography';
import type { SyncStatusDto, CommitLogEntry } from '../../lib/types';
import { useStore } from '../../stores/appStore';
import * as api from '../../lib/commands';
import { applyTheme } from '../../lib/theme';
import ConflictModal from '../ui/ConflictModal';
import PassphraseModal from '../ui/PassphraseModal';
import VaultTab from './VaultTab';
import ThemeTab from './ThemeTab';
import AITab from './AITab';
import ResearchTab from './ResearchTab';
import DeveloperTab from './DeveloperTab';
import SyncTab from './SyncTab';

type Tab = 'vault' | 'theme' | 'ai' | 'research' | 'developer' | 'sync';

export default function SettingsPage() {
  const { pickVaultDirectory, setThemeConfig } = useStore();
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [settings, setSettings] = useState<any>(null);
  const [saving, setSaving] = useState(false);
  const [fetching, setFetching] = useState(false);
  const [msg, setMsg] = useState('');
  const [msgSeverity, setMsgSeverity] = useState<'success' | 'error'>('success');
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [tab, setTab] = useState<Tab>('vault');
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
  const [conflictModalOpen, setConflictModalOpen] = useState(false);
  const [conflictFiles, setConflictFiles] = useState<string[]>([]);
  const [passphraseModalOpen, setPassphraseModalOpen] = useState(false);
  const [syncing, setSyncing] = useState(false);

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

  if (!settings) {
    return (
      <Box sx={{ p: 3 }}>
        <Typography variant="body2" color="text.secondary">
          Loading settings...
        </Typography>
      </Box>
    );
  }

  const ai = settings.ai;
  const research = settings.research || {
    searxng_endpoint: 'http://localhost:8888',
    max_results: 3,
    max_depth: 2,
  };
  const theme = settings.theme || {
    dark_mode: true,
    primary_color: '#f97316',
    secondary_color: '#6b7280',
    font_size: 16,
  };
  const syncSettings = settings.sync || {
    mode: 'manual',
    remote_url: null,
    branch: 'main',
    auto_commit_interval_secs: 300,
    auto_sync_interval_secs: 1800,
    ssh_key_path: null,
    commit_template:
      'stratum({datetime}): {editedfiles} edited, {newfiles} added, {deletedfiles} deleted',
  };

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const updateAi = (patch: any) => setSettings({ ...settings, ai: { ...ai, ...patch } });
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const updateVault = (patch: any) => setSettings({ ...settings, ...patch });
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const updateResearch = (patch: any) =>
    setSettings({ ...settings, research: { ...research, ...patch } });
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
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
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
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
        setConflictFiles(result.conflicts);
        setConflictModalOpen(true);
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
        setPassphraseModalOpen(true);
      } else {
        setMsg(`Sync failed: ${e}`);
      }
    } finally {
      setSyncing(false);
    }
  };

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const toggleModelCapability = (modelName: string, cap: string) => {
    const models = [...(ai.models || [])];
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const existing = models.find((m: any) => m.name === modelName);
    if (existing) {
      existing.capabilities = existing.capabilities.includes(cap)
        ? existing.capabilities.filter((c: string) => c !== cap)
        : [...existing.capabilities, cap];
    } else {
      models.push({ name: modelName, capabilities: [cap] });
    }
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
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
      setPassphraseModalOpen(false);
      if (result.status === 'conflicts') {
        setConflictFiles(result.conflicts);
        setConflictModalOpen(true);
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

  const handleResolveConflict = async (file: string) => {
    try {
      await api.resolveConflictFile(file);
      setConflictFiles(prev => prev.filter(f => f !== file));
      setMsg(`Resolved: ${file}`);
      setMsgSeverity('success');
    } catch (e) {
      setMsg(`Resolve failed: ${e}`);
      setMsgSeverity('error');
    }
  };

  const handleResolveAllConflicts = async () => {
    for (const f of conflictFiles) {
      try {
        await api.resolveConflictFile(f);
      } catch {
        /* skip */
      }
    }
    setConflictFiles([]);
    setConflictModalOpen(false);
    setMsg('All conflicts resolved.');
    setMsgSeverity('success');
  };

  const handleAbortMerge = async () => {
    try {
      await api.abortMerge();
      setConflictFiles([]);
      setConflictModalOpen(false);
      setMsg('Merge aborted.');
      setMsgSeverity('success');
    } catch (e) {
      setMsg(`Abort failed: ${e}`);
      setMsgSeverity('error');
    }
  };

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Tab bar */}
      <Box
        sx={{
          borderBottom: 1,
          borderColor: 'divider',
          display: 'flex',
          alignItems: 'center',
          flexShrink: 0,
        }}
      >
        <Tabs value={tab} onChange={(_, v: Tab) => setTab(v)} sx={{ minHeight: 40 }}>
          <Tab label="Vault" value="vault" sx={{ minHeight: 40, textTransform: 'none' }} />
          <Tab label="Theme" value="theme" sx={{ minHeight: 40, textTransform: 'none' }} />
          <Tab label="AI" value="ai" sx={{ minHeight: 40, textTransform: 'none' }} />
          <Tab label="Research" value="research" sx={{ minHeight: 40, textTransform: 'none' }} />
          <Tab label="Developer" value="developer" sx={{ minHeight: 40, textTransform: 'none' }} />
          <Tab label="Sync" value="sync" sx={{ minHeight: 40, textTransform: 'none' }} />
        </Tabs>
        <Box sx={{ flex: 1 }} />
        <Button
          variant="contained"
          onClick={handleSave}
          disabled={saving}
          sx={{ mr: 2, textTransform: 'none' }}
        >
          {saving ? 'Saving...' : 'Save'}
        </Button>
      </Box>

      {/* Message */}
      {msg && (
        <Alert severity={msgSeverity} sx={{ mx: 3, mt: 1.5 }} onClose={() => setMsg('')}>
          {msg}
        </Alert>
      )}

      {/* Tab content */}
      <Box sx={{ flex: 1, overflow: 'auto', p: 3 }}>
        {tab === 'vault' && (
          <VaultTab
            vaultPath={settings.vault_path || ''}
            onVaultPathChange={path => updateVault({ vault_path: path })}
            onBrowse={pickVaultDirectory}
          />
        )}
        {tab === 'theme' && <ThemeTab theme={theme} onThemeChange={updateTheme} />}
        {tab === 'ai' && (
          <AITab
            ai={ai}
            onAiChange={updateAi}
            availableModels={availableModels}
            fetching={fetching}
            onFetchModels={handleFetchModels}
            onToggleModelCapability={toggleModelCapability}
          />
        )}
        {tab === 'research' && (
          <ResearchTab research={research} onResearchChange={updateResearch} />
        )}
        {tab === 'developer' && (
          <DeveloperTab fetching={fetching} onReindex={handleReindex} />
        )}
        {tab === 'sync' && (
          <SyncTab
            syncSettings={syncSettings}
            onSyncChange={updateSync}
            syncStatus={syncStatus}
            commits={commits}
            commitsOpen={commitsOpen}
            onToggleCommits={handleToggleCommits}
            syncing={syncing}
            onSyncNow={handleSyncNow}
            onStartScheduler={handleStartScheduler}
          />
        )}
      </Box>

      {/* Passphrase Modal */}
      {passphraseModalOpen && (
        <PassphraseModal
          onClose={() => setPassphraseModalOpen(false)}
          onSubmit={handlePassphraseSubmit}
        />
      )}

      {/* Conflict Resolution Modal */}
      {conflictModalOpen && (
        <ConflictModal
          files={conflictFiles}
          onResolve={handleResolveConflict}
          onResolveAll={handleResolveAllConflicts}
          onAbort={handleAbortMerge}
        />
      )}
    </Box>
  );
}
