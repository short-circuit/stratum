import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import Button from '@mui/material/Button';
import LinearProgress from '@mui/material/LinearProgress';
import { useTranslation } from 'react-i18next';

// ---------------------------------------------------------------------------
// Mobile developer tools section.
// Extracted from SettingsPage.mobile.tsx during the mobile/desktop parity pass.
// Mirrors the desktop DeveloperTab surface — reindex, repair, normalize-all,
// and live reindex progress.
// ---------------------------------------------------------------------------

export interface MobileDeveloperSectionProps {
  fetching: boolean;
  onReindex: () => void;
  onRepair: () => void;
  onNormalizeAll: () => void;
  reindexProgress: { message: string; percent: number } | null;
}

export default function MobileDeveloperSection({
  fetching,
  onReindex,
  onRepair,
  onNormalizeAll,
  reindexProgress,
}: MobileDeveloperSectionProps) {
  const { t } = useTranslation();
  return (
    <Box sx={{ px: 2, pt: 3 }}>
      <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
        {t('settings.developer')}
      </Typography>
      <Divider sx={{ mb: 1.5 }} />
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
        {t('settings.re_sync_all_pages_from_disk_into_the_database')}
      </Typography>
      <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap', mb: 1 }}>
        <Button
          variant="contained"
          color="error"
          onClick={onReindex}
          disabled={fetching}
          size="small"
          sx={{ textTransform: 'none' }}
        >
          {fetching ? t('settings.reindexing') : t('settings.reindex_all')}
        </Button>
        <Button
          variant="outlined"
          color="error"
          onClick={onRepair}
          disabled={fetching}
          size="small"
          sx={{ textTransform: 'none' }}
        >
          {fetching ? t('settings.repairing') : t('settings.repair_db_from_disk')}
        </Button>
      </Box>
      {reindexProgress && (
        <Box sx={{ mb: 2 }}>
          <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
            {reindexProgress.message}
          </Typography>
          <LinearProgress
            variant="determinate"
            value={reindexProgress.percent * 100}
            sx={{ height: 6, borderRadius: 3 }}
          />
        </Box>
      )}
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1 }}>
        {t('settings.parse_every_md_file_through_the_block_parser')}
      </Typography>
      <Button
        variant="contained"
        color="warning"
        onClick={onNormalizeAll}
        disabled={fetching}
        size="small"
        sx={{ textTransform: 'none' }}
      >
        {fetching ? t('settings.normalizing') : t('settings.normalize_all_files')}
      </Button>
    </Box>
  );
}
