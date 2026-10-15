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
import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import DeleteOutlinedIcon from '@mui/icons-material/DeleteOutlined';
import PaletteOutlinedIcon from '@mui/icons-material/PaletteOutlined';
import type { ThemeCatalogEntry } from '../../lib/types';
import type { InstalledThemeInfo } from './useThemeHub';
import { useTranslation } from 'react-i18next';

export interface ThemeHubTabProps {
  entries: ThemeCatalogEntry[];
  loading: boolean;
  offline: boolean;
  error: string | null;
  fetchedAt: string | null;
  installedThemes: InstalledThemeInfo[];
  busyId: string | null;
  onInstall: (entry: ThemeCatalogEntry) => void;
  onUninstall: (id: string) => void;
  onApply: (entry: ThemeCatalogEntry) => void;
  onRefresh: () => void;
  onClearError: () => void;
  compact?: boolean;
}

/**
 * E10.F7 — Theme Hub tab (shared by desktop + mobile variants). Lists themes
 * from the theme catalog; installing verifies the sha256 of the theme
 * document, persists it locally, and applies it immediately. Uninstalling an
 * active theme falls back to the built-in theme.
 */
export default function ThemeHubTab({
  entries,
  loading,
  offline,
  error,
  fetchedAt,
  installedThemes,
  busyId,
  onInstall,
  onUninstall,
  onApply,
  onRefresh,
  onClearError,
  compact,
}: ThemeHubTabProps) {
  const { t } = useTranslation();
  const installedIds = new Set(installedThemes.map(i => i.entry.id));
  const activeId = installedThemes.find(i => i.active)?.entry.id ?? null;

  return (
    <Box>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 1 }}>
        <Box>
          <Typography variant={compact ? 'subtitle1' : 'h6'} sx={{ fontWeight: 600 }}>
            {t('plugins.themes_title')}
          </Typography>
          <Typography variant="caption" color="text.secondary">
            {t('plugins.themes_subtitle')}
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
      ) : entries.length === 0 && installedThemes.length === 0 ? (
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
          {/* Installed (offline-safe) themes first */}
          {installedThemes.map(({ entry, active }) => {
            const busy = busyId === entry.id;
            return (
              <Card key={entry.id} variant="outlined">
                <CardContent sx={{ p: 2 }}>
                  <ThemeRow
                    entry={entry}
                    trailing={
                      <>
                        {active && (
                          <Chip icon={<CheckCircleIcon />} label={t('plugins.active_theme')} size="small" color="success" variant="outlined" sx={{ height: 18, fontSize: '0.62rem' }} />
                        )}
                        <Button size="small" variant="outlined" disabled={busy || active} onClick={() => onApply(entry)} sx={{ textTransform: 'none' }}>
                          {t('plugins.apply')}
                        </Button>
                        <Button size="small" variant="outlined" color="error" startIcon={<DeleteOutlinedIcon />} disabled={busy} onClick={() => onUninstall(entry.id)} sx={{ textTransform: 'none' }}>
                          {busy ? '…' : t('plugins.uninstall')}
                        </Button>
                      </>
                    }
                  />
                </CardContent>
              </Card>
            );
          })}

          {/* Catalog themes not yet installed */}
          {entries
            .filter(entry => !installedIds.has(entry.id))
            .map(entry => {
              const busy = busyId === entry.id;
              return (
                <Card key={entry.id} variant="outlined">
                  <CardContent sx={{ p: 2 }}>
                    <ThemeRow
                      entry={entry}
                      trailing={
                        entry.id === activeId ? (
                          <Chip icon={<CheckCircleIcon />} label={t('plugins.active_theme')} size="small" color="success" variant="outlined" sx={{ height: 18, fontSize: '0.62rem' }} />
                        ) : (
                          <Button size="small" variant="contained" startIcon={<DownloadIcon />} disabled={busy} onClick={() => onInstall(entry)} sx={{ textTransform: 'none' }}>
                            {busy ? t('plugins.installing') : t('plugins.install')}
                          </Button>
                        )
                      }
                    />
                  </CardContent>
                </Card>
              );
            })}
        </Stack>
      )}
    </Box>
  );
}

function ThemeRow({ entry, trailing }: { entry: ThemeCatalogEntry; trailing: React.ReactNode }) {
  const { t } = useTranslation();
  return (
    <Box sx={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 2 }}>
      <Box sx={{ minWidth: 0 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flexWrap: 'wrap' }}>
          <PaletteOutlinedIcon color="primary" fontSize="small" />
          <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>{entry.name}</Typography>
          <Chip label={`v${entry.version}`} size="small" variant="outlined" sx={{ height: 20, fontSize: '0.68rem' }} />
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
        <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mt: 0.5 }}>
          {entry.dark_mode ? t('plugins.theme_dark') : t('plugins.theme_light')} · {entry.primary_color}
        </Typography>
      </Box>
      <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-end', gap: 1 }}>
        {trailing}
      </Box>
    </Box>
  );
}
