import { useState } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';
import Alert from '@mui/material/Alert';
import { useTranslation } from 'react-i18next';
import { SettingsSection } from '../ui/settings';
import * as api from '../../lib/commands';
import type { DiscoveredEndpoint } from '../../lib/types';

interface LocalDiscoverySectionProps {
  ai: {
    provider: string;
    endpoint: string | null;
    extra_local_hosts?: string[];
    last_discovered?: DiscoveredEndpoint[];
  };
  onAiChange: (patch: {
    provider?: string;
    endpoint?: string | null;
    extra_local_hosts?: string[];
    last_discovered?: DiscoveredEndpoint[];
  }) => void;
}

function endpointStatusLabel(
  ep: DiscoveredEndpoint,
  t: (key: string, opts?: Record<string, unknown>) => string,
): string {
  if (ep.status === 'available') {
    return `${t('settings.endpoint_available')} · ${ep.models.length} ${t('settings.models_count', { count: ep.models.length })}`;
  }
  return t('settings.endpoint_unreachable');
}

export default function LocalDiscoverySection({ ai, onAiChange }: LocalDiscoverySectionProps) {
  const { t } = useTranslation();
  const [discovered, setDiscovered] = useState<DiscoveredEndpoint[]>(ai.last_discovered ?? []);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [adoptedHost, setAdoptedHost] = useState<string | null>(null);

  const handleDiscover = async () => {
    setScanning(true);
    setError(null);
    try {
      const list = await api.discoverLocalAi();
      setDiscovered(list);
      onAiChange({ last_discovered: list });
    } catch (e) {
      setError(t('settings.scanning_failed', { error: e instanceof Error ? e.message : String(e) }));
    } finally {
      setScanning(false);
    }
  };

  const handleAdopt = (ep: DiscoveredEndpoint) => {
    onAiChange({
      provider: ep.kind === 'ollama' ? 'ollama' : 'custom',
      endpoint: ep.host,
    });
    setAdoptedHost(ep.host);
  };

  const handleExtraHostsChange = (value: string) => {
    const lines = value
      .split('\n')
      .map(l => l.trim())
      .filter(Boolean);
    onAiChange({ extra_local_hosts: lines });
  };

  const extraHostsValue = (ai.extra_local_hosts ?? []).join('\n');

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5, maxWidth: 480 }}>
      <SettingsSection title={t('settings.local_endpoints')} />

      <Box>
        <Button variant="outlined" size="small" onClick={handleDiscover} disabled={scanning}>
          {scanning ? t('settings.discovering') : t('settings.discover_local_models')}
        </Button>
      </Box>

      {error && (
        <Alert severity="error" sx={{ py: 0, px: 1.5 }}>
          <Typography variant="caption">{error}</Typography>
        </Alert>
      )}

      {discovered.length === 0 ? (
        <Typography variant="caption" color="text.secondary">
          {t('settings.no_local_endpoints')}
        </Typography>
      ) : (
        discovered.map(ep => {
          const isAvailable = ep.status === 'available';
          const isCurrent = ai.endpoint != null && ai.endpoint === ep.host;
          const isHint = isAvailable && !isCurrent;
          return (
            <Box
              key={ep.host}
              sx={{ border: 1, borderColor: 'divider', borderRadius: 1, p: 1.5 }}
            >
              <Typography variant="body2">
                <strong>{ep.name}</strong>
              </Typography>
              <Typography variant="caption" sx={{ display: 'block', fontFamily: 'monospace' }}>
                {ep.host}
              </Typography>
              <Typography variant="caption" color={isAvailable ? 'success.main' : 'error.main'}>
                {endpointStatusLabel(ep, t)}
              </Typography>
              {isAvailable && ep.models.length > 0 && (
                <Typography variant="caption" sx={{ display: 'block', fontFamily: 'monospace' }}>
                  {ep.models.join(', ')}
                </Typography>
              )}
              {!isAvailable && ep.error && (
                <Typography variant="caption" color="error.main" sx={{ display: 'block' }}>
                  {ep.error}
                </Typography>
              )}
              {isHint && (
                <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
                  {t('settings.endpoints_are_hints')}
                </Typography>
              )}
              {isAvailable && (
                <Box sx={{ mt: 0.5 }}>
                  <Button size="small" onClick={() => handleAdopt(ep)}>
                    {t('settings.use_this_endpoint')}
                  </Button>
                  {adoptedHost === ep.host && (
                    <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
                      {t('settings.remember_to_save_endpoint')}
                    </Typography>
                  )}
                </Box>
              )}
            </Box>
          );
        })
      )}

      <TextField
        label={t('settings.extra_local_hosts')}
        multiline
        minRows={2}
        value={extraHostsValue}
        onChange={e => handleExtraHostsChange(e.target.value)}
        size="small"
        helperText={t('settings.extra_local_hosts_help')}
        slotProps={{ htmlInput: { style: { fontFamily: 'monospace' } } }}
      />
    </Box>
  );
}
