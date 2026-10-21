import { useState } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import TextField from '@mui/material/TextField';
import type { SyncStatusDto, CommitLogEntry, BackupSettings, BackupStatusDto } from '../../lib/types';
import { useTranslation } from 'react-i18next';
import { SettingsSection, SettingsAccordion } from '../ui/settings';
import CommitLogPanel from './CommitLogPanel';
import SyncControlsPanel from './SyncControlsPanel';
import BackupSettingsPanel from './BackupSettingsPanel';

// ---------------------------------------------------------------------------
// Mobile sync configuration section.
// Extracted from SettingsPage.mobile.tsx (E6 sizing-gate + mobile/desktop
// parity pass) so the mobile settings screen stays under the component gate.
// Mirrors the desktop SyncTab surface — sync mode, remote/branch, SSH key,
// auto-commit template, auto-sync interval, and scheduler controls.
// ---------------------------------------------------------------------------

export interface MobileSyncSectionProps {
  syncSettings: {
    mode: string;
    remote_url: string | null;
    branch: string;
    auto_commit_interval_secs: number;
    auto_sync_interval_secs: number;
    ssh_key_path: string | null;
    commit_template: string;
  };
  updateSync: (patch: any) => void;
  syncStatus: SyncStatusDto | null;
  syncing: boolean;
  onSyncNow: () => void;
  onStartScheduler: () => Promise<void>;
  commits: CommitLogEntry[];
  commitsOpen: boolean;
  onToggleCommits: () => void;
  backup: BackupSettings;
  backupStatus: BackupStatusDto | null;
  updateBackup: (patch: Partial<BackupSettings>) => void;
  runBackupNow: () => Promise<void>;
  testWebdavConnection: () => Promise<void>;
  testing: boolean;
  running: boolean;
  webdavTestResult: string | null;
}

const MODES = ['manual', 'auto_commit', 'auto_sync', 'background'] as const;

const MODE_HELP_KEYS: Record<string, string> = {
  manual: 'settings.sync_only_when_you_click_the_sync_button_no_automatic_commit',
  auto_commit: 'settings.auto_commit_mode_help',
  auto_sync: 'settings.automatic_commits_periodic_push_pull_to_remote',
  background: 'settings.full_background_sync_commits_push_and_pull_happen_automatica',
};

const TEMPLATE_PLACEHOLDERS = [
  '{datetime}', '{editedfiles}', '{newfiles}', '{deletedfiles}', '{count}',
] as const;

export default function MobileSyncSection({
  syncSettings,
  updateSync,
  syncStatus,
  syncing,
  onSyncNow,
  onStartScheduler,
  commits,
  commitsOpen,
  onToggleCommits,
  backup,
  backupStatus,
  updateBackup,
  runBackupNow,
  testWebdavConnection,
  testing,
  running,
  webdavTestResult,
}: MobileSyncSectionProps) {
  const { t } = useTranslation();
  const [syncExpanded, setSyncExpanded] = useState(false);
  const insertPlaceholder = (placeholder: string) => {
    // Insert the placeholder at the current cursor of the template field via a
    // data attribute lookup (mirrors SyncTab behavior but on the mobile field).
    const input = document.querySelector<HTMLInputElement>(
      '[data-mobile-template-input]'
    );
    if (input) {
      const start = input.selectionStart ?? input.value.length;
      const end = input.selectionEnd ?? start;
      const before = input.value.substring(0, start);
      const after = input.value.substring(end);
      const next = before + placeholder + after;
      // Use the native value setter so React's value tracker stays intact, then
      // dispatch `input` (React 19 controlled inputs listen for this) — this
      // mirrors desktop SyncTab behaviour.
      const setter = Object.getOwnPropertyDescriptor(
        HTMLInputElement.prototype,
        'value'
      )?.set;
      setter?.call(input, next);
      input.dispatchEvent(new Event('input', { bubbles: true }));
    }
  };

  const templatePreview = syncSettings.commit_template
    .replace('{datetime}', new Date().toISOString().slice(0, 19).replace('T', ' '))
    .replace('{editedfiles}', '3')
    .replace('{newfiles}', '1')
    .replace('{deletedfiles}', '0')
    .replace('{count}', '4');

  return (
    <SettingsSection title={t('settings.sync')} variant="mobile" divider sx={{ pb: 3 }}>
      <SettingsAccordion
        summary={
          <Typography variant="body2" color="text.secondary">
            {syncExpanded ? t('settings.hide_sync_config') : t('settings.configure_sync')}
          </Typography>
        }
        expanded={syncExpanded}
        onChange={setSyncExpanded}
      >
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5 }}>
          {/* Sync mode selector */}
      <Typography variant="caption" sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 0.75 }}>
        {t('settings.sync_mode')}
      </Typography>
      <Box sx={{ display: 'flex', gap: 0.5, flexWrap: 'wrap', mb: 0.5 }}>
        {MODES.map(mode => (
          <Box
            key={mode}
            component="button"
            onClick={() => updateSync({ mode })}
            sx={{
              px: 2,
              py: 1,
              borderRadius: 1,
              border: 'none',
              cursor: 'pointer',
              fontSize: '0.75rem',
              fontWeight: 600,
              textTransform: 'capitalize',
              bgcolor: syncSettings.mode === mode ? 'var(--primary-500)' : 'action.selected',
              color: syncSettings.mode === mode ? '#fff' : 'text.primary',
              '&:hover': { opacity: 0.85 },
            }}
          >
            {mode === 'auto_commit' ? t('settings.auto_commit') : mode === 'auto_sync' ? t('settings.auto_sync') : mode}
          </Box>
        ))}
      </Box>
      <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mb: 2 }}>
        {t(MODE_HELP_KEYS[syncSettings.mode] || MODE_HELP_KEYS.manual)}
      </Typography>

      {/* Remote & branch */}
      <Typography variant="caption" sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 0.75 }}>
        {t('settings.remote_amp_branch')}
      </Typography>
      <Box sx={{ display: 'flex', gap: 1, mb: 2 }}>
        <TextField
          size="small"
          placeholder="git@github.com:user/vault.git"
          value={syncSettings.remote_url || ''}
          onChange={e => updateSync({ remote_url: e.target.value || null })}
          sx={{
            flex: 1,
            '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.75rem' },
          }}
        />
        <TextField
          size="small"
          placeholder="main"
          value={syncSettings.branch}
          onChange={e => updateSync({ branch: e.target.value || 'main' })}
          sx={{
            width: 100,
            '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.75rem' },
          }}
        />
      </Box>

      {/* SSH key path */}
      <Typography variant="caption" sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 0.75 }}>
        {t('settings.ssh_key_path')}
      </Typography>
      <Box sx={{ display: 'flex', gap: 1, alignItems: 'center', mb: 0.25 }}>
        <TextField
          size="small"
          placeholder="~/.ssh/id_ed25519"
          value={syncSettings.ssh_key_path || ''}
          onChange={e => updateSync({ ssh_key_path: e.target.value || null })}
          sx={{
            flex: 1,
            '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.75rem' },
          }}
        />
        <Box
          sx={{
            px: 1.5,
            py: 0.25,
            borderRadius: 1,
            fontSize: '0.65rem',
            fontWeight: 600,
            textTransform: 'uppercase',
            letterSpacing: '0.05em',
            bgcolor: syncSettings.ssh_key_path ? '#10b981' : '#6b7280',
            color: '#fff',
            flexShrink: 0,
          }}
        >
          {syncSettings.ssh_key_path ? t('settings.set') : t('settings.agent')}
        </Box>
      </Box>
      <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mb: 2 }}>
        {t('settings.leave_empty_to_use_ssh_agent')}
      </Typography>

      {/* Auto-commit settings (conditional) */}
      {['auto_commit', 'auto_sync', 'background'].includes(syncSettings.mode) && (
        <Box sx={{ mb: 2 }}>
          <Typography variant="caption" sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 0.75 }}>
            {t('settings.auto_commit_settings')}
          </Typography>
          <TextField
            label={t('settings.commit_interval_seconds')}
            type="number"
            value={syncSettings.auto_commit_interval_secs}
            onChange={e =>
              updateSync({ auto_commit_interval_secs: parseInt(e.target.value) || 30 })
            }
            size="small"
            slotProps={{ htmlInput: { min: 30 } }}
            fullWidth
            sx={{ mb: 1.5 }}
          />
          <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
            {t('settings.commit_message_template')}
          </Typography>
          <TextField
            size="small"
            value={syncSettings.commit_template}
            onChange={e => updateSync({ commit_template: e.target.value })}
            slotProps={{ htmlInput: { 'data-mobile-template-input': '' } }}
            fullWidth
            sx={{
              mb: 0.75,
              '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.75rem' },
            }}
          />
          <Box sx={{ display: 'flex', gap: 0.5, flexWrap: 'wrap', mb: 0.75 }}>
            {TEMPLATE_PLACEHOLDERS.map(placeholder => (
              <Box
                key={placeholder}
                component="button"
                onClick={() => insertPlaceholder(placeholder)}
                sx={{
                  px: 1,
                  py: 0.25,
                  borderRadius: 0.5,
                  border: '1px solid',
                  borderColor: 'divider',
                  bgcolor: 'action.hover',
                  cursor: 'pointer',
                  fontSize: '0.65rem',
                  fontFamily: 'monospace',
                  color: 'text.secondary',
                  '&:hover': { bgcolor: 'action.selected' },
                }}
              >
                {placeholder}
              </Box>
            ))}
          </Box>
          <Box
            sx={{
              px: 1,
              py: 0.75,
              borderRadius: 0.5,
              bgcolor: 'action.hover',
              fontSize: '0.7rem',
              fontFamily: 'monospace',
              color: 'text.disabled',
            }}
          >
            {t('settings.preview')} {templatePreview}
          </Box>
        </Box>
      )}

      {/* Auto-sync settings (conditional) */}
      {['auto_sync', 'background'].includes(syncSettings.mode) && (
        <Box sx={{ mb: 2 }}>
          <Typography variant="caption" sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 0.75 }}>
            {t('settings.auto_sync_settings')}
          </Typography>
          <TextField
            label={t('settings.pull_push_interval_seconds')}
            type="number"
            value={syncSettings.auto_sync_interval_secs}
            onChange={e =>
              updateSync({ auto_sync_interval_secs: parseInt(e.target.value) || 60 })
            }
            size="small"
            slotProps={{ htmlInput: { min: 60 } }}
            fullWidth
          />
        </Box>
      )}

      {/* Controls: sync now + scheduler */}
      <SyncControlsPanel
        syncSettings={{ mode: syncSettings.mode }}
        syncStatus={syncStatus}
        syncing={syncing}
        onSyncNow={onSyncNow}
        onStartScheduler={onStartScheduler}
      />

      {/* Commit log */}
      <Box sx={{ mt: 1.5 }}>
        <CommitLogPanel
          commits={commits}
          commitsOpen={commitsOpen}
          onToggleCommits={onToggleCommits}
        />
      </Box>

      {/* Cloud backup */}
      <Divider sx={{ my: 1.5 }} />
      <BackupSettingsPanel
        backup={backup}
        backupStatus={backupStatus}
        onBackupChange={updateBackup}
        onRunBackupNow={runBackupNow}
        onTestWebdav={testWebdavConnection}
        testing={testing}
        running={running}
        webdavTestResult={webdavTestResult}
      />
        </Box>
      </SettingsAccordion>
    </SettingsSection>
  );
}
