import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import type { SyncStatusDto } from '../../lib/types';

// ---------------------------------------------------------------------------
// SyncControlsPanel — sync status badge, sync-now / scheduler controls.
// Extracted from SyncTab.tsx during the E6 sizing-gate refactor.
// ---------------------------------------------------------------------------

export default function SyncControlsPanel({
  syncSettings,
  syncStatus,
  syncing,
  onSyncNow,
  onStartScheduler,
}: {
  syncSettings: {
    mode: string;
  };
  syncStatus: SyncStatusDto | null;
  syncing: boolean;
  onSyncNow: () => void;
  onStartScheduler: () => Promise<void>;
}) {
  return (
    <Box>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, mb: 0.75 }}>
        <Button
          variant="contained"
          onClick={onSyncNow}
          disabled={syncing}
          sx={{
            textTransform: 'none',
            bgcolor: 'var(--primary-500)',
            '&:hover': { opacity: 0.85 },
          }}
        >
          {syncing ? 'Syncing...' : 'Sync Now'}
        </Button>
        {syncStatus && (
          <Box
            sx={{
              px: 1.5,
              py: 0.25,
              borderRadius: 1,
              fontSize: '0.65rem',
              fontWeight: 600,
              textTransform: 'uppercase',
              letterSpacing: '0.05em',
              color: '#fff',
              bgcolor:
                syncStatus.status === 'ok'
                  ? '#10b981'
                  : syncStatus.status === 'conflicts'
                    ? '#ef4444'
                    : syncStatus.status === 'no_repo'
                      ? '#eab308'
                      : '#6b7280',
            }}
          >
            {syncStatus.status === 'ok' && 'OK'}
            {syncStatus.status === 'conflicts' &&
              `Conflicts (${syncStatus.conflicts.length})`}
            {syncStatus.status === 'no_repo' && 'No Repo'}
            {syncStatus.status !== 'ok' &&
              syncStatus.status !== 'conflicts' &&
              syncStatus.status !== 'no_repo' &&
              syncStatus.status}
            {(syncStatus.ahead > 0 || syncStatus.behind > 0) && (
              <Box component="span" sx={{ ml: 0.5, fontWeight: 400 }}>
                +{syncStatus.ahead}/-{syncStatus.behind}
              </Box>
            )}
          </Box>
        )}
        {['auto_commit', 'auto_sync', 'background'].includes(syncSettings.mode) && (
          <Button
            variant="outlined"
            size="small"
            onClick={onStartScheduler}
            sx={{ textTransform: 'none', fontSize: '0.75rem' }}
          >
            Start Scheduler
          </Button>
        )}
      </Box>
      {syncStatus && (
        <Box sx={{ display: 'flex', gap: 2, alignItems: 'center' }}>
          {syncStatus.branch && (
            <Typography
              variant="caption"
              color="text.disabled"
              sx={{ fontFamily: 'monospace' }}
            >
              {syncStatus.branch}
            </Typography>
          )}
          {syncStatus.last_sync_time && (
            <Typography variant="caption" color="text.disabled">
              Last sync: {new Date(syncStatus.last_sync_time).toLocaleString()}
            </Typography>
          )}
        </Box>
      )}
    </Box>
  );
}
