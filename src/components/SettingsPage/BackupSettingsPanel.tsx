import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Alert from '@mui/material/Alert';
import { useTranslation } from 'react-i18next';
import { SettingsSection, SettingsRow } from '../ui/settings';
import type { BackupSettings, BackupStatusDto } from '../../lib/types';

interface BackupSettingsPanelProps {
  backup: BackupSettings;
  backupStatus: BackupStatusDto | null;
  onBackupChange: (patch: Partial<BackupSettings>) => void;
  onRunBackupNow: () => Promise<void>;
  onTestWebdav: () => Promise<void>;
  testing: boolean;
  running: boolean;
  webdavTestResult: string | null;
}

const MODES = ['folder', 'webdav', 'both'] as const;

export default function BackupSettingsPanel({
  backup,
  backupStatus,
  onBackupChange,
  onRunBackupNow,
  onTestWebdav,
  testing,
  running,
  webdavTestResult,
}: BackupSettingsPanelProps) {
  const { t } = useTranslation();
  const modeKey = (m: string) =>
    m === 'webdav'
      ? t('settings.backup_mode_webdav')
      : m === 'both'
        ? t('settings.backup_mode_both')
        : t('settings.backup_mode_folder');
  const usesFolder = backup.mode === 'folder' || backup.mode === 'both';
  const usesWebdav = backup.mode === 'webdav' || backup.mode === 'both';

  return (
    <SettingsSection title={t('settings.backup')} sx={{ mt: 1.5 }}>
      {/* Enable */}
      <SettingsRow
        label={t('settings.backup_enable')}
        checked={backup.enabled}
        onCheckedChange={v => onBackupChange({ enabled: v })}
      />

      {/* Mode */}
      <Typography
        variant="caption"
        sx={{ fontWeight: 500, color: 'text.secondary', display: 'block', mt: 1, mb: 0.75 }}
      >
        {t('settings.backup_mode')}
      </Typography>
      <Box sx={{ display: 'flex', gap: 0.5, flexWrap: 'wrap', mb: 1 }}>
        {MODES.map(mode => (
          <Box
            key={mode}
            component="button"
            onClick={() => onBackupChange({ mode })}
            sx={{
              px: 2,
              py: 1,
              borderRadius: 1,
              border: 'none',
              cursor: 'pointer',
              fontSize: '0.75rem',
              fontWeight: 600,
              textTransform: 'capitalize',
              bgcolor: backup.mode === mode ? 'var(--primary-500)' : 'action.selected',
              color: backup.mode === mode ? '#fff' : 'text.primary',
              '&:hover': { opacity: 0.85 },
            }}
          >
            {modeKey(mode)}
          </Box>
        ))}
      </Box>

      {/* Folder target */}
      {usesFolder && (
        <TextField
          label={t('settings.backup_folder_path')}
          placeholder="/path/to/backup"
          value={backup.folder_path || ''}
          onChange={e => onBackupChange({ folder_path: e.target.value || null })}
          size="small"
          fullWidth
          sx={{ mb: 1.5, '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.75rem' } }}
        />
      )}

      {/* WebDAV target */}
      {usesWebdav && (
        <Box sx={{ mb: 1.5 }}>
          <TextField
            label={t('settings.backup_webdav_url')}
            placeholder="https://dav.example.com/backup"
            value={backup.webdav_url || ''}
            onChange={e => onBackupChange({ webdav_url: e.target.value || null })}
            size="small"
            fullWidth
            sx={{ mb: 1, '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.75rem' } }}
          />
          <Box sx={{ display: 'flex', gap: 1 }}>
            <TextField
              label={t('settings.backup_webdav_username')}
              value={backup.webdav_username || ''}
              onChange={e => onBackupChange({ webdav_username: e.target.value || null })}
              size="small"
              sx={{ flex: 1 }}
            />
            <TextField
              label={t('settings.backup_webdav_password')}
              type="password"
              value={backup.webdav_password || ''}
              onChange={e => onBackupChange({ webdav_password: e.target.value || null })}
              size="small"
              sx={{ flex: 1 }}
            />
          </Box>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mt: 1 }}>
            <Button
              size="small"
              variant="outlined"
              disabled={testing || !backup.webdav_url}
              onClick={onTestWebdav}
            >
              {testing ? t('settings.testing') : t('settings.backup_test_connection')}
            </Button>
            {webdavTestResult && (
              <Typography
                variant="caption"
                sx={{
                  color: webdavTestResult === 'ok' ? 'success.main' : 'error.main',
                  fontWeight: 600,
                }}
              >
                {webdavTestResult === 'ok'
                  ? t('settings.backup_test_ok')
                  : webdavTestResult}
              </Typography>
            )}
          </Box>
        </Box>
      )}

      {/* Interval */}
      <TextField
        label={t('settings.backup_interval_minutes')}
        type="number"
        value={backup.interval_minutes}
        onChange={e => onBackupChange({ interval_minutes: parseInt(e.target.value) || 60 })}
        size="small"
        slotProps={{ htmlInput: { min: 1 } }}
        sx={{ width: 200, mb: 1.5, display: 'block' }}
      />

      {/* Exclude */}
      <TextField
        label={t('settings.backup_exclude')}
        value={backup.exclude.join(', ')}
        onChange={e =>
          onBackupChange({
            exclude: e.target.value
              .split(',')
              .map(s => s.trim())
              .filter(Boolean),
          })
        }
        size="small"
        fullWidth
        helperText={t('settings.backup_exclude_help')}
        sx={{ mb: 1.5, '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.75rem' } }}
      />

      {/* Run now + status */}
      <Button
        size="small"
        variant="contained"
        disabled={running}
        onClick={onRunBackupNow}
        sx={{ mb: 1 }}
      >
        {running ? t('settings.backup_running') : t('settings.backup_run_now')}
      </Button>

      {backupStatus && (
        <Box sx={{ mb: 1 }}>
          <Typography variant="caption" sx={{ display: 'block', color: 'text.secondary' }}>
            {t('settings.backup_last_run')}{' '}
            {backupStatus.last_run ? new Date(backupStatus.last_run).toLocaleString() : '—'}
          </Typography>
          {backupStatus.files_copied != null && (
            <Typography variant="caption" sx={{ display: 'block', color: 'text.secondary' }}>
              {t('settings.backup_files_copied')} {backupStatus.files_copied}
            </Typography>
          )}
          {backupStatus.files_deleted != null && (
            <Typography variant="caption" sx={{ display: 'block', color: 'text.secondary' }}>
              {t('settings.backup_files_deleted')} {backupStatus.files_deleted}
            </Typography>
          )}
          {backupStatus.last_success && !backupStatus.last_error && (
            <Typography variant="caption" sx={{ display: 'block', color: 'success.main' }}>
              {t('settings.backup_no_errors')}
            </Typography>
          )}
          {backupStatus.last_error && (
            <Typography variant="caption" sx={{ display: 'block', color: 'error.main' }}>
              {t('settings.backup_failed')} {backupStatus.last_error}
            </Typography>
          )}
        </Box>
      )}

      {backupStatus?.last_success && (
        <Alert severity="success" sx={{ mb: 1 }}>
          {t('settings.backup_success')}
        </Alert>
      )}

      <Typography variant="caption" color="text.disabled" sx={{ display: 'block' }}>
        {t('settings.backup_manual_note')}
      </Typography>
    </SettingsSection>
  );
}
