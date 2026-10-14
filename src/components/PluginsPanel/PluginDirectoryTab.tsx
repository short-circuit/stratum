import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Card from '@mui/material/Card';
import CardContent from '@mui/material/CardContent';
import Chip from '@mui/material/Chip';
import CircularProgress from '@mui/material/CircularProgress';
import Stack from '@mui/material/Stack';
import Typography from '@mui/material/Typography';
import RefreshIcon from '@mui/icons-material/Refresh';
import CloudOffIcon from '@mui/icons-material/CloudOff';
import DownloadIcon from '@mui/icons-material/Download';
import DeleteOutlinedIcon from '@mui/icons-material/DeleteOutlined';
import type { PluginCatalogEntry } from '../../lib/types';
import { useTranslation } from 'react-i18next';

export interface PluginDirectoryTabProps {
  entries: PluginCatalogEntry[];
  loading: boolean;
  offline: boolean;
  error: string | null;
  fetchedAt: string | null;
  installedIds: Set<string>;
  busyId: string | null;
  onInstall: (entry: PluginCatalogEntry) => void;
  onUninstall: (id: string) => void;
  onRefresh: () => void;
  onClearError: () => void;
  compact?: boolean;
}

/**
 * E10.F7 — Plugin Directory tab (shared by desktop + mobile variants).
 * Lists community plugins from the cached catalog with verified-sha256
 * install / uninstall. Never breaks when offline: cached entries render with
 * an offline badge and a Retry action.
 */
export default function PluginDirectoryTab({
  entries,
  loading,
  offline,
  error,
  fetchedAt,
  installedIds,
  busyId,
  onInstall,
  onUninstall,
  onRefresh,
  onClearError,
  compact,
}: PluginDirectoryTabProps) {
  const { t } = useTranslation();

  return (
    <Box>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 1 }}>
        <Box>
          <Typography variant={compact ? 'subtitle1' : 'h6'} sx={{ fontWeight: 600 }}>
            {t('plugins.directory_title')}
          </Typography>
          <Typography variant="caption" color="text.secondary">
            {t('plugins.directory_subtitle')}
            {offline && (
              <Chip
                icon={<CloudOffIcon />}
                label={t('plugins.offline_badge')}
                size="small"
                color="warning"
                variant="outlined"
                sx={{ ml: 1, height: 18, fontSize: '0.62rem' }}
              />
            )}
            {fetchedAt && (
              <Typography variant="caption" color="text.disabled" component="span" sx={{ ml: 1 }}>
                {t('plugins.catalog_fetched_at')} {new Date(fetchedAt).toLocaleString()}
              </Typography>
            )}
          </Typography>
        </Box>
        <Button size="small" startIcon={<RefreshIcon />} onClick={onRefresh} disabled={loading} sx={{ textTransform: 'none' }}>
          {t('common.refresh')}
        </Button>
      </Box>

      {error && (
        <Typography
          variant="body2"
          color="error"
          sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 2, mb: 1, cursor: 'pointer' }}
          onClick={onClearError}
        >
          {error}
          <Box component="span" sx={{ fontSize: '0.75rem', textDecoration: 'underline' }}>
            {t('plugins.click_to_dismiss')}
          </Box>
        </Typography>
      )}

      {loading ? (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, py: 4, justifyContent: 'center' }}>
          <CircularProgress size={18} />
          <Typography variant="body2" color="text.secondary">{t('plugins.loading_plugins')}</Typography>
        </Box>
      ) : entries.length === 0 ? (
        <Box sx={{ py: 4, textAlign: 'center' }}>
          <Typography variant="body2" color="text.secondary">{t('plugins.directory_empty')}</Typography>
          {error && (
            <Button size="small" onClick={onRefresh} sx={{ mt: 1, textTransform: 'none' }}>
              {t('common.retry')}
            </Button>
          )}
        </Box>
      ) : (
        <Stack spacing={1.5}>
          {entries.map(entry => {
            const installed = installedIds.has(entry.id);
            const busy = busyId === entry.id;
            return (
              <Card key={entry.id} variant="outlined">
                <CardContent sx={{ p: 2 }}>
                  <Box sx={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 2 }}>
                    <Box sx={{ minWidth: 0 }}>
                      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flexWrap: 'wrap' }}>
                        <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>{entry.name}</Typography>
                        <Chip label={`v${entry.version}`} size="small" variant="outlined" sx={{ height: 20, fontSize: '0.68rem' }} />
                        {installed && (
                          <Chip label={t('plugins.installed')} size="small" color="success" variant="outlined" sx={{ height: 18, fontSize: '0.62rem' }} />
                        )}
                      </Box>
                      <Typography variant="caption" color="text.disabled" sx={{ fontFamily: 'monospace', display: 'block', mb: 0.5 }}>
                        {entry.id}
                      </Typography>
                      {entry.description && (
                        <Typography variant="body2" color="text.secondary" sx={{ mb: 0.5 }}>
                          {entry.description}
                        </Typography>
                      )}
                      {entry.author && (
                        <Typography variant="caption" color="text.disabled" sx={{ display: 'block' }}>
                          {t('plugins.by_author')} {entry.author}
                        </Typography>
                      )}
                      {entry.permissions.length > 0 && (
                        <Stack direction="row" spacing={0.5} sx={{ flexWrap: 'wrap', gap: 0.5, mt: 0.75 }}>
                          {entry.permissions.map(perm => (
                            <Chip key={perm} label={perm} size="small" sx={{ height: 18, fontSize: '0.62rem' }} />
                          ))}
                        </Stack>
                      )}
                    </Box>
                    <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-end', gap: 1 }}>
                      {installed ? (
                        <Button
                          size="small"
                          variant="outlined"
                          color="error"
                          startIcon={<DeleteOutlinedIcon />}
                          disabled={busy}
                          onClick={() => onUninstall(entry.id)}
                          sx={{ textTransform: 'none' }}
                        >
                          {busy ? '…' : t('plugins.uninstall')}
                        </Button>
                      ) : (
                        <Button
                          size="small"
                          variant="contained"
                          startIcon={<DownloadIcon />}
                          disabled={busy}
                          onClick={() => onInstall(entry)}
                          sx={{ textTransform: 'none' }}
                        >
                          {busy ? t('plugins.installing') : t('plugins.install')}
                        </Button>
                      )}
                    </Box>
                  </Box>
                  <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mt: 0.5 }}>
                    {t('plugins.verified_sha256')}
                  </Typography>
                </CardContent>
              </Card>
            );
          })}
        </Stack>
      )}
    </Box>
  );
}
