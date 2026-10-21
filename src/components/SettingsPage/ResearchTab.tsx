import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import { useTranslation } from 'react-i18next';
import { SettingsSection } from '../ui/settings';
import ResearchPresetsSection from './ResearchPresetsSection';

interface ResearchTabProps {
  research: { searxng_endpoint: string; max_results: number; max_depth: number };
  onResearchChange: (patch: Partial<ResearchTabProps['research']>) => void;
}

export default function ResearchTab({ research, onResearchChange }: ResearchTabProps) {
  const { t } = useTranslation();
  return (
    <SettingsSection title={t('settings.research')}>
      <Box sx={{ maxWidth: 480, display: 'flex', flexDirection: 'column', gap: 2 }}>
        <TextField
          label={t('settings.searxng_endpoint_url')}
          placeholder="http://localhost:8888"
          value={research.searxng_endpoint}
          onChange={e => onResearchChange({ searxng_endpoint: e.target.value })}
          size="small"
          helperText={t('settings.the_url_of_your_searxng_instance_e_g_http_localh')}
        />

        <TextField
          label={t('settings.max_results_per_search')}
          type="number"
          value={research.max_results}
          onChange={e => onResearchChange({ max_results: parseInt(e.target.value) || 3 })}
          size="small"
          slotProps={{ htmlInput: { min: 1, max: 10 } }}
          sx={{ width: 200 }}
        />

        <TextField
          label={t('settings.research_depth_search_read_cycles')}
          type="number"
          value={research.max_depth}
          onChange={e => onResearchChange({ max_depth: parseInt(e.target.value) || 2 })}
          size="small"
          slotProps={{ htmlInput: { min: 1, max: 5 } }}
          sx={{ width: 200 }}
        />
      </Box>

      <Box sx={{ mt: 3 }}>
        <ResearchPresetsSection research={research} onResearchChange={onResearchChange} />
      </Box>
    </SettingsSection>
  );
}
