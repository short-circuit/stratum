import { useState } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import { useTranslation } from 'react-i18next';
import * as api from '../../lib/commands';

const PRESETS = [
  'https://searx.be',
  'https://search.bus-hit.me',
  'https://priv.au',
  'https://searx.tiekoetter.com',
];

interface ResearchPresetsSectionProps {
  research: { searxng_endpoint: string };
  onResearchChange: (patch: { searxng_endpoint: string }) => void;
}

type RowState = { testing: boolean; ok?: boolean; detail?: string };

export default function ResearchPresetsSection({ research, onResearchChange }: ResearchPresetsSectionProps) {
  const { t } = useTranslation();
  const [states, setStates] = useState<Record<string, RowState>>({});

  const testInstance = async (url: string) => {
    setStates(s => ({ ...s, [url]: { testing: true } }));
    try {
      const res = await api.pingSearchInstance(url);
      if (res.ok) {
        setStates(s => ({
          ...s,
          [url]: { testing: false, ok: true, detail: t('settings.instance_reachable', { latency_ms: res.latency_ms }) },
        }));
      } else {
        setStates(s => ({ ...s, [url]: { testing: false, ok: false, detail: res.error ?? t('settings.instance_unreachable') } }));
      }
    } catch (e) {
      setStates(s => ({ ...s, [url]: { testing: false, ok: false, detail: e instanceof Error ? e.message : String(e) } }));
    }
  };

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1, maxWidth: 480 }}>
      <Typography variant="subtitle2" color="text.secondary">
        {t('settings.search_presets')}
      </Typography>
      <Typography variant="caption" color="text.secondary">
        {t('settings.search_presets_help')}
      </Typography>
      {PRESETS.map(url => {
        const state = states[url];
        const selected = research.searxng_endpoint === url;
        return (
          <Box
            key={url}
            sx={{
              display: 'flex',
              alignItems: 'center',
              gap: 1,
              flexWrap: 'wrap',
              border: 1,
              borderColor: selected ? 'primary.main' : 'divider',
              borderRadius: 1,
              p: 1,
            }}
          >
            {selected && <CheckCircleIcon color="primary" fontSize="small" />}
            <Typography variant="caption" sx={{ fontFamily: 'monospace', flex: '1 1 auto' }}>
              {url}
            </Typography>
            {state?.ok && state.detail && (
              <Typography variant="caption" color="success.main">
                {state.detail}
              </Typography>
            )}
            {state?.ok === false && state.detail && (
              <Typography variant="caption" color="error.main">
                {state.detail}
              </Typography>
            )}
            <Button size="small" onClick={() => testInstance(url)} disabled={state?.testing}>
              {t('settings.test_instance')}
            </Button>
            <Button size="small" onClick={() => onResearchChange({ searxng_endpoint: url })}>
              {t('settings.use_this_instance')}
            </Button>
          </Box>
        );
      })}
    </Box>
  );
}
