import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import Button from '@mui/material/Button';
import Alert from '@mui/material/Alert';
import TextField from '@mui/material/TextField';
import { useSettingsPage } from './SettingsPage.shared';
import MobileThemeSection from './MobileThemeSection';
import MobileAiAccordion from './MobileAiAccordion';

export default function SettingsPageMobile() {
  const {
    settings,
    saving,
    fetching,
    msg,
    msgSeverity,
    syncing,
    syncStatus,
    theme,
    updateTheme,
    ai,
    updateAi,
    research,
    setMsg,
    updateVault,
    updateResearch,
    handleSave,
    handleReindex,
    handleRepair,
    handleSyncNow,
    pickVaultDirectory,
  } = useSettingsPage();

  if (!settings) {
    return (
      <Box sx={{ p: 2 }}>
        <Typography variant="body2" color="text.secondary">
          Loading settings...
        </Typography>
      </Box>
    );
  }

  return (
    <Box sx={{ height: '100%', overflow: 'auto' }}>
      {/* Save button + message */}
      <Box sx={{ p: 2, pb: 0 }}>
        <Button
          variant="contained"
          onClick={handleSave}
          disabled={saving}
          fullWidth
          sx={{ textTransform: 'none', mb: 1 }}
        >
          {saving ? 'Saving...' : 'Save'}
        </Button>
        {msg && (
          <Alert severity={msgSeverity} onClose={() => setMsg('')} sx={{ mb: 1 }}>
            {msg}
          </Alert>
        )}
      </Box>

      {/* ─── Vault Section ─── */}
      <Box sx={{ px: 2, pt: 2 }}>
        <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
          Vault
        </Typography>
        <Divider sx={{ mb: 1.5 }} />
        <TextField
          label="Vault Path"
          value={settings.vault_path || ''}
          onChange={e => updateVault({ vault_path: e.target.value })}
          fullWidth
          size="small"
          sx={{ mb: 1, '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.8rem' } }}
        />
        <Button variant="outlined" size="small" onClick={pickVaultDirectory}>
          Browse
        </Button>
      </Box>

      {/* ─── Theme Section ─── */}
      <MobileThemeSection theme={theme} updateTheme={updateTheme} />

      {/* ─── AI Section (collapsible) ─── */}
      <MobileAiAccordion ai={ai} updateAi={updateAi} />

      {/* ─── Research Section ─── */}
      <Box sx={{ px: 2, pt: 2 }}>
        <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
          Research
        </Typography>
        <Divider sx={{ mb: 1.5 }} />
        <TextField
          label="SearXNG Endpoint"
          placeholder="http://localhost:8888"
          value={research.searxng_endpoint}
          onChange={e => updateResearch({ searxng_endpoint: e.target.value })}
          fullWidth
          size="small"
          helperText="URL of your SearXNG instance"
        />
      </Box>

      {/* ─── Developer Section ─── */}
      <Box sx={{ px: 2, pt: 3 }}>
        <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
          Developer
        </Typography>
        <Divider sx={{ mb: 1.5 }} />
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
          Re-sync all pages from disk into the database. Idempotent.
        </Typography>
        <Button
          variant="contained"
          color="error"
          onClick={handleReindex}
          disabled={fetching}
          size="small"
          sx={{ textTransform: 'none' }}
        >
          {fetching ? 'Reindexing...' : 'Rebuild Index'}
        </Button>
        <Button
          variant="outlined"
          color="error"
          onClick={handleRepair}
          disabled={fetching}
          size="small"
          sx={{ textTransform: 'none', ml: 1 }}
        >
          {fetching ? 'Repairing...' : 'Repair DB'}
        </Button>
      </Box>

      {/* ─── Sync Section ─── */}
      <Box sx={{ px: 2, pt: 3, pb: 3 }}>
        <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
          Sync
        </Typography>
        <Divider sx={{ mb: 1.5 }} />
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, mb: 1 }}>
          <Button
            variant="contained"
            onClick={handleSyncNow}
            disabled={syncing}
            size="small"
            sx={{ textTransform: 'none' }}
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
        </Box>
        {syncStatus?.branch && (
          <Typography variant="caption" color="text.disabled" sx={{ fontFamily: 'monospace', display: 'block' }}>
            {syncStatus.branch}
          </Typography>
        )}
        {syncStatus?.last_sync_time && (
          <Typography variant="caption" color="text.disabled" sx={{ display: 'block' }}>
            Last sync: {new Date(syncStatus.last_sync_time).toLocaleString()}
          </Typography>
        )}
      </Box>

    </Box>
  );
}
