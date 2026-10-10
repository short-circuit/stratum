import { useState, useEffect } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Alert from '@mui/material/Alert';
import Select from '@mui/material/Select';
import MenuItem from '@mui/material/MenuItem';
import TextField from '@mui/material/TextField';
import Button from '@mui/material/Button';
import Switch from '@mui/material/Switch';
import Checkbox from '@mui/material/Checkbox';
import FormControlLabel from '@mui/material/FormControlLabel';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import ToggleButton from '@mui/material/ToggleButton';
import Divider from '@mui/material/Divider';
import { useTranslation } from 'react-i18next';
import { SttTestButton, TtsTestButton } from './AiTestButtons';

const PROVIDERS = [
  { value: 'ollama', labelKey: 'settings.ollama_local' },
  { value: 'openai', labelKey: 'settings.openai' },
  { value: 'anthropic', labelKey: 'settings.anthropic' },
  { value: 'google', labelKey: 'settings.google_ai' },
  { value: 'zai', labelKey: 'settings.z_ai' },
  { value: 'custom', labelKey: 'settings.custom_openai_compatible' },
  { value: 'custom-openai', labelKey: 'settings.custom_openai_api' },
  { value: 'custom-anthropic', labelKey: 'settings.custom_anthropic_api' },
];

const CAPABILITIES = ['chat', 'embedding', 'tts'] as const;

interface AITabProps {
  ai: {
    provider: string;
    endpoint: string | null;
    api_key: string | null;
    api_key_from_env: boolean;
    model: string;
    models: { name: string; capabilities: string[] }[];
    rag_enabled: boolean;
    rag_chunk_count: number;
    embedding_dimensions: number;
    use_llm_gateway_and_auth: boolean;
  };
  onAiChange: (patch: Partial<AITabProps['ai']>) => void;
  availableModels: string[];
  fetching: boolean;
  onFetchModels: () => void;
  onToggleModelCapability: (modelName: string, cap: string) => void;
  stt?: {
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
  onSttChange?: (patch: Partial<NonNullable<AITabProps['stt']>>) => void;
  tts?: {
    endpoint: string;
    api_key: string | null;
    voice: string;
    format: string;
    speed: number;
    use_llm_gateway_and_auth: boolean;
  };
  onTtsChange?: (patch: Partial<NonNullable<AITabProps['tts']>>) => void;
}

function envVarForProvider(provider: string): string {
  switch (provider) {
    case 'openai':
    case 'custom-openai':
      return 'OPENAI_API_KEY';
    case 'anthropic':
    case 'custom-anthropic':
      return 'ANTHROPIC_API_KEY';
    case 'google':
      return 'GOOGLE_API_KEY';
    default:
      return '';
  }
}

export default function AITab({
  ai,
  onAiChange,
  availableModels,
  fetching,
  onFetchModels,
  onToggleModelCapability,
  stt,
  onSttChange,
  tts,
  onTtsChange,
}: AITabProps) {
  const { t } = useTranslation();
  const modelCaps = (name: string) =>
    (ai.models || []).find(m => m.name === name)?.capabilities || [];
  const envVarName = envVarForProvider(ai.provider);
  const [isKeyMasked, setIsKeyMasked] = useState(false);

  useEffect(() => {
    setIsKeyMasked(!!(ai.api_key && ai.api_key.includes('****')));
  }, [ai.api_key]);

  return (
    <Box>
      <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>{t('settings.ai_configuration')}</Typography>

      {ai.api_key_from_env && (
        <Alert severity="warning" sx={{ mb: 2, maxWidth: 480 }}>
          <Typography variant="caption">
            <strong>{t('settings.security_notice')}</strong> {t('settings.api_key_is_set_via_the')} <strong>{envVarName}</strong> {t('settings.environment_variable')}
            {' '}{t('settings.environment_variables_are_more_secure_than_storing_keys_in_c')}
            {' '}{t('settings.to_use_a_different_key_unset_the_environment_variable_or_upd')}
          </Typography>
        </Alert>
      )}

      <Box sx={{ maxWidth: 480, display: 'flex', flexDirection: 'column', gap: 2 }}>
        <Select
          value={ai.provider || 'ollama'}
          onChange={e => onAiChange({ provider: e.target.value })}
          size="small"
          displayEmpty
        >
          {PROVIDERS.map(p => (
            <MenuItem key={p.value} value={p.value}>
              {t(p.labelKey)}
            </MenuItem>
          ))}
        </Select>

        {(ai.provider === 'ollama' ||
          ai.provider === 'custom' ||
          ai.provider === 'zai' ||
          ai.provider === 'custom-openai' ||
          ai.provider === 'custom-anthropic') && (
          <TextField
            label={t('settings.api_endpoint_url')}
            placeholder="http://localhost:11434"
            value={ai.endpoint || ''}
            onChange={e => onAiChange({ endpoint: e.target.value || null })}
            size="small"
          />
        )}

        <TextField
          label={t('settings.api_key')}
          type="password"
          placeholder={isKeyMasked ? t('settings.key_saved_enter_new_value_to_change') : 'sk-...'}
          value={isKeyMasked ? '' : (ai.api_key || '')}
          onChange={e => onAiChange({ api_key: e.target.value || null })}
          size="small"
          sx={{ '& .MuiInputBase-input': { fontFamily: 'monospace' } }}
        />

        {ai.api_key_from_env && (
          <Alert severity="info" sx={{ py: 0, px: 1.5, '& .MuiAlert-message': { py: 0.75 } }}>
            <Typography variant="caption">
              {t('settings.api_key_loaded_from')} <strong>{envVarName}</strong> {t('settings.environment_variable')}
              {ai.api_key ? ` ${t('settings.config_file_key_is_ignored_while_the_env_var_is_set')}` : ''}
            </Typography>
          </Alert>
        )}

        <TextField
          label={t('settings.default_chat_model')}
          placeholder="gpt-4o"
          value={ai.model || ''}
          onChange={e => onAiChange({ model: e.target.value })}
          size="small"
        />

        <Box>
          <Button variant="outlined" size="small" onClick={onFetchModels} disabled={fetching}>
            {fetching ? t('settings.fetching') : t('settings.fetch_available_models')}
          </Button>
        </Box>

        {availableModels.length > 0 && (
          <Box>
            <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 0.5 }}>
              {t('settings.models_click_to_enable_capabilities')}
            </Typography>
            <Box
              sx={{
                maxHeight: 200,
                overflow: 'auto',
                border: 1,
                borderColor: 'divider',
                borderRadius: 1,
              }}
            >
              {availableModels.map(m => {
                const caps = modelCaps(m);
                return (
                  <Box
                    key={m}
                    sx={{
                      display: 'flex',
                      alignItems: 'center',
                      px: 1.5,
                      py: 0.75,
                      '&:hover': { bgcolor: 'action.hover' },
                    }}
                  >
                    <Typography variant="caption" sx={{ flex: 1, fontFamily: 'monospace' }}>
                      {m}
                    </Typography>
                    <ToggleButtonGroup size="small" value={caps}>
                      {CAPABILITIES.map(cap => (
                        <ToggleButton
                          key={cap}
                          value={cap}
                          selected={caps.includes(cap)}
                          onChange={() => onToggleModelCapability(m, cap)}
                          sx={{ textTransform: 'none', fontSize: '0.7rem', px: 1, py: 0.25 }}
                        >
                          {cap}
                        </ToggleButton>
                      ))}
                    </ToggleButtonGroup>
                  </Box>
                );
              })}
            </Box>
          </Box>
        )}

        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
          <FormControlLabel
            control={
              <Switch
                checked={ai.rag_enabled}
                onChange={e => onAiChange({ rag_enabled: e.target.checked })}
              />
            }
            label={t('settings.enable_rag')}
          />
          {ai.rag_enabled && (
            <TextField
              label={t('settings.chunks')}
              type="number"
              value={ai.rag_chunk_count || 5}
              onChange={e => onAiChange({ rag_chunk_count: parseInt(e.target.value) || 5 })}
              size="small"
              slotProps={{ htmlInput: { min: 1, max: 20 } }}
              sx={{ width: 100 }}
            />
          )}
        </Box>
        <FormControlLabel
          control={
            <Checkbox
              checked={ai.use_llm_gateway_and_auth ?? false}
              onChange={e => onAiChange({ use_llm_gateway_and_auth: e.target.checked })}
            />
          }
          label={t('settings.use_gateway_and_auth_from_llm')}
          sx={{ '& .MuiFormControlLabel-label': { fontSize: '0.875rem' } }}
        />

        <TextField
          label={t('settings.embedding_dimensions_0_auto')}
          type="number"
          value={ai.embedding_dimensions || 0}
          onChange={e => onAiChange({ embedding_dimensions: parseInt(e.target.value) || 0 })}
          size="small"
          slotProps={{ htmlInput: { min: 0, max: 4096 } }}
          sx={{ width: 220 }}
          helperText={t('settings.leave_0_to_infer_the_vector_size_from_your_embed')}
        />
      </Box>

      {stt && onSttChange && (
        <Box sx={{ mt: 3 }}>
          <Divider sx={{ mb: 2 }} />
          <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
            {t('settings.voice_dictation_stt')}
          </Typography>
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
            <FormControlLabel
              control={
                <Checkbox
                  checked={stt.use_llm_gateway_and_auth ?? false}
                  onChange={e => onSttChange({ use_llm_gateway_and_auth: e.target.checked })}
                />
              }
              label={t('settings.use_gateway_and_auth_from_llm')}
              sx={{ '& .MuiFormControlLabel-label': { fontSize: '0.875rem' } }}
            />
            <SttTestButton />
          </Box>
        </Box>
      )}

      {tts && onTtsChange && (
        <Box sx={{ mt: 3 }}>
          <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
            {t('settings.text_to_speech')}
          </Typography>
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
            <FormControlLabel
              control={
                <Checkbox
                  checked={tts.use_llm_gateway_and_auth ?? false}
                  onChange={e => onTtsChange({ use_llm_gateway_and_auth: e.target.checked })}
                />
              }
              label={t('settings.use_gateway_and_auth_from_llm')}
              sx={{ '& .MuiFormControlLabel-label': { fontSize: '0.875rem' } }}
            />
            <TtsTestButton />
          </Box>
        </Box>
      )}
    </Box>
  );
}
