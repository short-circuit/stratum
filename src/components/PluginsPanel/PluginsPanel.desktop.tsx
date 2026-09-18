import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import Chip from '@mui/material/Chip';
import CircularProgress from '@mui/material/CircularProgress';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import RefreshIcon from '@mui/icons-material/Refresh';
import ExtensionIcon from '@mui/icons-material/Extension';
import AddIcon from '@mui/icons-material/Add';
import DeleteOutlinedIcon from '@mui/icons-material/DeleteOutlined';
import { usePluginsPanel } from './PluginsPanel.shared';
import { StatusChip } from './StatusChip';
import EmptyState from '../ui/EmptyState';

export default function PluginsPanelDesktop() {
  const {
    plugins,
    loading,
    error,
    busyId,
    testResults,
    backendReady,
    refresh,
    enable,
    disable,
    reload,
    installFromFile,
    uninstall,
    runNoteReadTest,
    runHttpRequestTest,
    clearTestResult,
    clearError,
  } = usePluginsPanel();

  const installing = busyId === '__install__';

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      <Box sx={{ flex: 1, overflow: 'auto', p: 3, maxWidth: 760, mx: 'auto', width: '100%' }}>
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 0.5 }}>
          <Typography variant="h5" sx={{ fontWeight: 600 }}>Plugins</Typography>
          <Box sx={{ display: 'flex', gap: 1 }}>
            <Button
              size="small"
              startIcon={<RefreshIcon />}
              onClick={() => void refresh()}
              disabled={loading || installing}
              sx={{ textTransform: 'none' }}
            >
              Refresh
            </Button>
            <Button
              size="small"
              variant="contained"
              startIcon={<AddIcon />}
              onClick={() => void installFromFile()}
              disabled={loading || installing}
              sx={{ textTransform: 'none' }}
            >
              {installing ? 'Installing…' : 'Install'}
            </Button>
          </Box>
        </Box>
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 2 }}>
          WASM plugins loaded from <Box component="code" sx={{ bgcolor: 'action.hover', px: 0.5, borderRadius: 0.5 }}>~/.pkm/plugins</Box>.
          {!backendReady && ' The plugin backend has not been wired yet — showing mock data so the UI is usable.'}
        </Typography>

        {error && (
          <Box sx={{ mb: 2 }}>
            <Typography
              variant="body2"
              color="error"
              sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 2, cursor: 'pointer' }}
              onClick={clearError}
            >
              {error}
              <Box component="span" sx={{ fontSize: '0.75rem', textDecoration: 'underline' }}>dismiss</Box>
            </Typography>
          </Box>
        )}

        {loading ? (
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, py: 4, justifyContent: 'center' }}>
            <CircularProgress size={18} />
            <Typography variant="body2" color="text.secondary">Loading plugins...</Typography>
          </Box>
        ) : plugins.length === 0 ? (
          <EmptyState
            icon={<ExtensionIcon sx={{ fontSize: 40 }} />}
            message="No plugins installed"
            description="Install a plugin.wasm file, or drop one into ~/.pkm/plugins/ and press Refresh."
            actionLabel="Install"
            onAction={() => void installFromFile()}
          />
        ) : (
          <Stack spacing={1.5}>
            {plugins.map(plugin => {
              const test = testResults[plugin.id];
              const busy = busyId === plugin.id;
              return (
                <Card key={plugin.id} variant="outlined">
                  <CardContent sx={{ p: 2 }}>
                    <Box sx={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 2 }}>
                      <Box sx={{ minWidth: 0 }}>
                        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flexWrap: 'wrap' }}>
                          <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>{plugin.name}</Typography>
                          <Chip
                            label={`v${plugin.version}`}
                            size="small"
                            variant="outlined"
                            sx={{ height: 20, fontSize: '0.68rem' }}
                          />
                        </Box>
                        <Typography variant="caption" color="text.disabled" sx={{ fontFamily: 'monospace', display: 'block', mb: 1 }}>
                          {plugin.id}
                        </Typography>
                        {plugin.permissions.length > 0 && (
                          <Stack direction="row" spacing={0.5} sx={{ flexWrap: 'wrap', gap: 0.5 }}>
                            {plugin.permissions.map(perm => (
                              <Chip key={perm} label={perm} size="small" sx={{ height: 18, fontSize: '0.62rem' }} />
                            ))}
                          </Stack>
                        )}
                      </Box>
                      <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-end', gap: 1 }}>
                        <StatusChip plugin={plugin} />
                        <Box sx={{ display: 'flex', gap: 0.5 }}>
                          {plugin.enabled ? (
                            <Button size="small" variant="outlined" disabled={busy} onClick={() => void disable(plugin.id)} sx={{ textTransform: 'none' }}>
                              {busy ? '…' : 'Disable'}
                            </Button>
                          ) : (
                            <Button size="small" variant="contained" disabled={busy} onClick={() => void enable(plugin.id)} sx={{ textTransform: 'none' }}>
                              {busy ? '…' : 'Enable'}
                            </Button>
                          )}
                          <Button size="small" variant="outlined" disabled={busy} onClick={() => void reload(plugin.id)} sx={{ textTransform: 'none' }}>
                            {busy ? '…' : 'Reload'}
                          </Button>
                          <Button
                            size="small"
                            variant="outlined"
                            color="error"
                            startIcon={<DeleteOutlinedIcon />}
                            disabled={busy}
                            onClick={() => void uninstall(plugin.id)}
                            sx={{ textTransform: 'none' }}
                          >
                            {busy ? '…' : 'Uninstall'}
                          </Button>
                        </Box>
                      </Box>
                    </Box>

                    <Box sx={{ mt: 1.5, display: 'flex', gap: 1, alignItems: 'center', flexWrap: 'wrap' }}>
                      <Button size="small" disabled={busy} onClick={() => void runNoteReadTest(plugin.id)} sx={{ textTransform: 'none' }}>
                        Test note_read
                      </Button>
                      <Button size="small" disabled={busy} onClick={() => void runHttpRequestTest(plugin.id)} sx={{ textTransform: 'none' }}>
                        Test http_request
                      </Button>
                      {test && (
                        <Typography
                          variant="caption"
                          color={test.ok ? 'primary' : 'error'}
                          sx={{ flex: '1 1 auto', textAlign: 'right', cursor: 'pointer', userSelect: 'none' }}
                          onClick={() => clearTestResult(plugin.id)}
                          title="Click to dismiss"
                        >
                          {test.label}: {test.ok ? 'ok' : 'failed'} — {test.detail}
                        </Typography>
                      )}
                    </Box>
                  </CardContent>
                </Card>
              );
            })}
          </Stack>
        )}
      </Box>
    </Box>
  );
}
