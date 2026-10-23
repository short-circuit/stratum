import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import TextField from '@mui/material/TextField';
import type { SyncStatusDto, CommitLogEntry, BackupSettings, BackupStatusDto } from '../../lib/types';
import CommitLogPanel from './CommitLogPanel';
import SyncControlsPanel from './SyncControlsPanel';
import BackupSettingsPanel from './BackupSettingsPanel';
import { SettingsSection } from '../ui/settings';
import { useTranslation } from 'react-i18next';

interface SyncTabProps {
  syncSettings: {
    mode: string;
    remote_url: string | null;
    branch: string;
    auto_commit_interval_secs: number;
    auto_sync_interval_secs: number;
    ssh_key_path: string | null;
    commit_template: string;
  };
  onSyncChange: (patch: Partial<SyncTabProps['syncSettings']>) => void;
  syncStatus: SyncStatusDto | null;
  commits: CommitLogEntry[];
  commitsOpen: boolean;
  onToggleCommits: () => void;
  syncing: boolean;
  onSyncNow: () => void;
  onStartScheduler: () => Promise<void>;
  backup: BackupSettings;
  backupStatus: BackupStatusDto | null;
  onBackupChange: (patch: Partial<BackupSettings>) => void;
  onRunBackupNow: () => Promise<void>;
  onTestWebdav: () => Promise<void>;
  testing: boolean;
  running: boolean;
  webdavTestResult: string | null;
}

export default function SyncTab({
  syncSettings,
  onSyncChange,
  syncStatus,
  commits,
  commitsOpen,
  onToggleCommits,
  syncing,
  onSyncNow,
  onStartScheduler,
  backup,
  backupStatus,
  onBackupChange,
  onRunBackupNow,
  onTestWebdav,
  testing,
  running,
  webdavTestResult,
}: SyncTabProps) {
  const { t } = useTranslation();
  return (
    <SettingsSection title={t('settings.sync')}>
      <Box sx={{ maxWidth: 480, display: 'flex', flexDirection: 'column', gap: 2 }}>
        {/* Section 1 — Sync Mode */}
        <Box>
          <Typography
            variant="caption"
            sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 0.75 }}
          >
            {t('settings.sync_mode')}
          </Typography>
          <Box sx={{ display: 'flex', gap: 0.5, flexWrap: 'wrap' }}>
            {['manual', 'auto_commit', 'auto_sync', 'background'].map(mode => (
              <Box
                key={mode}
                component="button"
                onClick={() => onSyncChange({ mode })}
                sx={{
                  px: 2,
                  py: 1,
                  borderRadius: 1,
                  border: 'none',
                  cursor: 'pointer',
                  fontSize: '0.75rem',
                  fontWeight: 600,
                  textTransform: 'capitalize',
                  bgcolor:
                    syncSettings.mode === mode ? 'var(--primary-500)' : 'action.selected',
                  color: syncSettings.mode === mode ? '#fff' : 'text.primary',
                  '&:hover': { opacity: 0.85 },
                }}
              >
                {mode === 'manual'
                  ? t('settings.sync_mode_manual')
                  : mode === 'auto_commit'
                    ? t('settings.auto_commit')
                    : mode === 'auto_sync'
                      ? t('settings.auto_sync')
                      : t('settings.sync_mode_background')}
              </Box>
            ))}
          </Box>
          <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mt: 0.75 }}>
            {syncSettings.mode === 'manual' &&
              t('settings.sync_only_when_you_click_the_sync_button_no_automatic_commit')}
            {syncSettings.mode === 'auto_commit' &&
              t('settings.auto_commit_mode_help')}
            {syncSettings.mode === 'auto_sync' &&
              t('settings.automatic_commits_periodic_push_pull_to_remote')}
            {syncSettings.mode === 'background' &&
              t('settings.full_background_sync_commits_push_and_pull_happen_automatica')}
          </Typography>
        </Box>

        {/* Section 2 — Remote & Branch */}
        <Box>
          <Typography
            variant="caption"
            sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 0.75 }}
          >
            {t('settings.remote_branch')}
          </Typography>
          <Box sx={{ display: 'flex', gap: 1 }}>
            <TextField
              size="small"
              placeholder="git@github.com:user/vault.git"
              value={syncSettings.remote_url || ''}
              onChange={e => onSyncChange({ remote_url: e.target.value || null })}
              sx={{
                flex: 1,
                '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.75rem' },
              }}
            />
            <TextField
              size="small"
              placeholder="main"
              value={syncSettings.branch}
              onChange={e => onSyncChange({ branch: e.target.value || 'main' })}
              sx={{
                width: 120,
                '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.75rem' },
              }}
            />
          </Box>
        </Box>

        {/* Section 3 — SSH Key */}
        <Box>
          <Typography
            variant="caption"
            sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 0.75 }}
          >
            {t('settings.ssh_key_path')}
          </Typography>
          <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
            <TextField
              size="small"
              placeholder="~/.ssh/id_ed25519"
              value={syncSettings.ssh_key_path || ''}
              onChange={e => onSyncChange({ ssh_key_path: e.target.value || null })}
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
              {syncSettings.ssh_key_path ? t('settings.ssh_key_set') : t('settings.ssh_key_agent')}
            </Box>
          </Box>
          <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mt: 0.25 }}>
            {t('settings.leave_empty_to_use_ssh_agent')}
          </Typography>
        </Box>

        {/* Section 4 — Auto-Commit Settings */}
        {['auto_commit', 'auto_sync', 'background'].includes(syncSettings.mode) && (
          <Box>
            <Typography
              variant="caption"
              sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 0.75 }}
            >
              {t('settings.auto_commit_settings')}
            </Typography>
            <TextField
              label={t('settings.commit_interval_seconds')}
              type="number"
              value={syncSettings.auto_commit_interval_secs}
              onChange={e =>
                onSyncChange({ auto_commit_interval_secs: parseInt(e.target.value) || 30 })
              }
              size="small"
              slotProps={{ htmlInput: { min: 30 } }}
              sx={{ width: 200, mb: 1.5 }}
            />
            <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
              {t('settings.commit_message_template')}
            </Typography>
            <TextField
              size="small"
              value={syncSettings.commit_template}
              onChange={e => onSyncChange({ commit_template: e.target.value })}
              slotProps={{ htmlInput: { 'data-template-input': '' } }}
              sx={{
                width: '100%',
                mb: 0.75,
                '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.75rem' },
              }}
            />
            <Box sx={{ display: 'flex', gap: 0.5, flexWrap: 'wrap', mb: 1 }}>
              {['{datetime}', '{editedfiles}', '{newfiles}', '{deletedfiles}', '{count}'].map(
                placeholder => (
                  <Box
                    key={placeholder}
                    component="button"
                    onClick={() => {
                      const input = document.querySelector(
                        '[data-template-input]'
                      ) as HTMLInputElement;
                      if (input) {
                        const start = input.selectionStart ?? input.value.length;
                        const end = input.selectionEnd ?? start;
                        const before = input.value.substring(0, start);
                        const after = input.value.substring(end);
                        input.value = before + placeholder + after;
                        input.selectionStart = input.selectionEnd =
                          start + placeholder.length;
                        input.focus();
                        input.dispatchEvent(new Event('input', { bubbles: true }));
                      }
                    }}
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
                )
              )}
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
              {t('settings.preview')}{' '}
              {syncSettings.commit_template
                .replace('{datetime}', new Date().toISOString().slice(0, 19).replace('T', ' '))
                .replace('{editedfiles}', '3')
                .replace('{newfiles}', '1')
                .replace('{deletedfiles}', '0')
                .replace('{count}', '4')}
            </Box>
          </Box>
        )}

        {/* Section 5 — Auto-Sync Settings */}
        {['auto_sync', 'background'].includes(syncSettings.mode) && (
          <Box>
            <Typography
              variant="caption"
              sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mb: 0.75 }}
            >
              {t('settings.auto_sync_settings')}
            </Typography>
            <TextField
              label={t('settings.pull_push_interval_seconds')}
              type="number"
              value={syncSettings.auto_sync_interval_secs}
              onChange={e =>
                onSyncChange({ auto_sync_interval_secs: parseInt(e.target.value) || 60 })
              }
              size="small"
              slotProps={{ htmlInput: { min: 60 } }}
              sx={{ width: 200 }}
            />
          </Box>
        )}

        {/* Section 6 — Controls */}
        <SyncControlsPanel
          syncSettings={syncSettings}
          syncStatus={syncStatus}
          syncing={syncing}
          onSyncNow={onSyncNow}
          onStartScheduler={onStartScheduler}
        />

        {/* Section 7 — Recent Commits */}
        <CommitLogPanel
          commits={commits}
          commitsOpen={commitsOpen}
          onToggleCommits={onToggleCommits}
        />

        {/* Section 8 — Cloud Backup */}
        <BackupSettingsPanel
          backup={backup}
          backupStatus={backupStatus}
          onBackupChange={onBackupChange}
          onRunBackupNow={onRunBackupNow}
          onTestWebdav={onTestWebdav}
          testing={testing}
          running={running}
          webdavTestResult={webdavTestResult}
        />
      </Box>
    </SettingsSection>
  );
}
