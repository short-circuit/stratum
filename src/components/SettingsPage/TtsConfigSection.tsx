import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import { useTranslation } from 'react-i18next';
import { SettingsSection, SettingsRow } from '../ui/settings';
import { TtsTestButton } from './AiTestButtons';

interface TtsConfigSectionProps {
  tts: {
    endpoint: string;
    api_key: string | null;
    voice: string;
    format: string;
    speed: number;
    use_llm_gateway_and_auth: boolean;
  };
  onTtsChange: (patch: Partial<TtsConfigSectionProps['tts']>) => void;
}

export default function TtsConfigSection({ tts, onTtsChange }: TtsConfigSectionProps) {
  const { t } = useTranslation();

  return (
    <SettingsSection title={t('settings.text_to_speech')} sx={{ mt: 3 }}>
      <Box sx={{ maxWidth: 480, display: 'flex', flexDirection: 'column', gap: 2 }}>
        <TextField
          label={t('settings.tts_endpoint_optional')}
          placeholder={t('settings.uses_the_ai_endpoint_when_empty')}
          value={tts.endpoint || ''}
          onChange={e => onTtsChange({ endpoint: e.target.value })}
          size="small"
        />
        <TextField
          label={t('settings.api_key_optional')}
          type="password"
          placeholder={tts.api_key?.includes('****') ? t('settings.key_saved_enter_new_value_to_change') : ''}
          value={tts.api_key?.includes('****') ? '' : (tts.api_key || '')}
          onChange={e => onTtsChange({ api_key: e.target.value || null })}
          size="small"
          sx={{ '& .MuiInputBase-input': { fontFamily: 'monospace' } }}
        />
        <Box sx={{ display: 'flex', gap: 2 }}>
          <TextField
            label={t('settings.voice')}
            placeholder="alloy"
            value={tts.voice || ''}
            onChange={e => onTtsChange({ voice: e.target.value })}
            size="small"
            sx={{ flex: 1 }}
          />
          <TextField
            label={t('settings.format')}
            placeholder="mp3"
            value={tts.format || ''}
            onChange={e => onTtsChange({ format: e.target.value })}
            size="small"
            sx={{ flex: 1 }}
          />
        </Box>
        <TextField
          label={t('settings.speed')}
          type="number"
          slotProps={{ htmlInput: { min: 0.25, max: 4.0, step: 0.1 } }}
          value={tts.speed ?? 1.0}
          onChange={e => onTtsChange({ speed: parseFloat(e.target.value) || 1.0 })}
          size="small"
          helperText={t('settings.playback_speed_multiplier_0_25_4_0')}
        />
        <SettingsRow
          label={t('settings.use_gateway_and_auth_from_llm')}
          checked={tts.use_llm_gateway_and_auth ?? false}
          onCheckedChange={v => onTtsChange({ use_llm_gateway_and_auth: v })}
        />
        <TtsTestButton />
      </Box>
    </SettingsSection>
  );
}
