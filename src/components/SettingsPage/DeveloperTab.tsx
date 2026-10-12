import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import LinearProgress from '@mui/material/LinearProgress';
import { useTranslation } from 'react-i18next';

interface DeveloperTabProps {
  fetching: boolean;
  onReindex: () => Promise<void>;
  onRepair: () => Promise<void>;
  onNormalizeAll: () => Promise<void>;
  reindexProgress: { message: string; percent: number } | null;
  onResetOnboarding: () => Promise<void>;
  onInstallTemplates: () => Promise<void>;
  installing: boolean;
}

export default function DeveloperTab({ fetching, onReindex, onRepair, onNormalizeAll, reindexProgress, onResetOnboarding, onInstallTemplates, installing }: DeveloperTabProps) {
  const { t } = useTranslation();
  return (
    <Box>
      <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>{t('settings.developer_tools')}</Typography>

      <Box sx={{ maxWidth: 480 }}>
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1.5 }}>
          {t('settings.re_sync_all_pages_from_disk_into_the_database')}
        </Typography>
        <Button
          variant="contained"
          color="error"
          onClick={onReindex}
          disabled={fetching}
        >
          {fetching ? t('settings.reindexing') : t('settings.reindex_all')}
        </Button>
        <Button
          variant="outlined"
          color="error"
          onClick={onRepair}
          disabled={fetching}
          sx={{ ml: 1 }}
        >
          {fetching ? t('settings.repairing') : t('settings.repair_db_from_disk')}
        </Button>
        {reindexProgress && (
          <Box sx={{ mt: 1.5 }}>
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
      </Box>

      <Box sx={{ maxWidth: 480, mt: 3 }}>
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1.5 }}>
          {t('settings.parse_every_md_file_through_the_block_parser')}
        </Typography>
        <Button
          variant="contained"
          color="warning"
          onClick={onNormalizeAll}
          disabled={fetching}
        >
          {fetching ? t('settings.normalizing') : t('settings.normalize_all_files')}
        </Button>
      </Box>

      <Box sx={{ maxWidth: 480, mt: 3 }}>
        <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
          {t('settings.developer')}
        </Typography>
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1.5 }}>
          {t('settings.reset_onboarding_help')}
        </Typography>
        <Button
          variant="outlined"
          onClick={onResetOnboarding}
          disabled={installing}
        >
          {t('settings.reset_onboarding')}
        </Button>
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1.5, mt: 2 }}>
          {t('settings.install_starter_templates_help')}
        </Typography>
        <Button
          variant="outlined"
          onClick={onInstallTemplates}
          disabled={installing}
        >
          {installing ? t('onboarding.installing') : t('settings.install_starter_templates')}
        </Button>
      </Box>
    </Box>
  );
}
