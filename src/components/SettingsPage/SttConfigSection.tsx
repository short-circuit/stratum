import Box from '@mui/material/Box';
import TextField from '@mui/material/TextField';
import Switch from '@mui/material/Switch';
import FormControlLabel from '@mui/material/FormControlLabel';
import { useTranslation } from 'react-i18next';
import { SettingsSection, SettingsRow } from '../ui/settings';
import { SttTestButton } from './AiTestButtons';

interface SttConfigSectionProps {
  stt: {
    endpoint: string;
    api_key: string | null;
    model: string;
    diarize_model: string;
    language: string | null;
    diarize: boolean;
    auto_summarize: boolean;
    auto_identify: boolean;
    use_llm_gateway_and_auth: boolean;
  };
  onSttChange: (patch: Partial<SttConfigSectionProps['stt']>) => void;
}

export default function SttConfigSection({ stt, onSttChange }: SttConfigSectionProps) {
  const { t } = useTranslation();

  return (
    <SettingsSection title={t('settings.voice_dictation_stt')} sx={{ mt: 3 }}>
      <Box sx={{ maxWidth: 480, display: 'flex', flexDirection: 'column', gap: 2 }}>
        <TextField
          label={t('settings.transcription_endpoint_openai_compatible')}
          placeholder="http://localhost:8081"
          value={stt.endpoint || ''}
          onChange={e => onSttChange({ endpoint: e.target.value })}
          size="small"
        />
        <TextField
          label={t('settings.api_key_optional')}
          type="password"
          placeholder={stt.api_key?.includes('****') ? t('settings.key_saved_enter_new_value_to_change') : ''}
          value={stt.api_key?.includes('****') ? '' : (stt.api_key || '')}
          onChange={e => onSttChange({ api_key: e.target.value || null })}
          size="small"
          sx={{ '& .MuiInputBase-input': { fontFamily: 'monospace' } }}
        />
        <Box sx={{ display: 'flex', gap: 2 }}>
          <TextField
            label={t('settings.transcription_model')}
            value={stt.model || ''}
            onChange={e => onSttChange({ model: e.target.value })}
            size="small"
            sx={{ flex: 1 }}
          />
          <TextField
            label={t('settings.diarization_model')}
            value={stt.diarize_model || ''}
            onChange={e => onSttChange({ diarize_model: e.target.value })}
            size="small"
            sx={{ flex: 1 }}
          />
        </Box>
        <TextField
          label={t('settings.language_hint_optional')}
          placeholder="en"
          value={stt.language || ''}
          onChange={e => onSttChange({ language: e.target.value || null })}
          size="small"
        />
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, flexWrap: 'wrap' }}>
          <FormControlLabel
            control={<Switch checked={stt.diarize} onChange={e => onSttChange({ diarize: e.target.checked })} />}
            label={t('dictation.diarize_speakers')}
          />
          <FormControlLabel
            control={<Switch checked={stt.auto_summarize} onChange={e => onSttChange({ auto_summarize: e.target.checked })} />}
            label={t('settings.auto_summarize')}
          />
          <FormControlLabel
            control={<Switch checked={stt.auto_identify} onChange={e => onSttChange({ auto_identify: e.target.checked })} />}
            label={t('settings.auto_identify_voices')}
          />
        </Box>
        <SettingsRow
          label={t('settings.use_gateway_and_auth_from_llm')}
          checked={stt.use_llm_gateway_and_auth ?? false}
          onCheckedChange={v => onSttChange({ use_llm_gateway_and_auth: v })}
        />
        <SttTestButton />
      </Box>
    </SettingsSection>
  );
}
