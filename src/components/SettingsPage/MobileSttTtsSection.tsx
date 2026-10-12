import { useState } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import TextField from '@mui/material/TextField';
import Switch from '@mui/material/Switch';
import FormControlLabel from '@mui/material/FormControlLabel';
import { SttTestButton, TtsTestButton } from './AiTestButtons';
import VoiceCaptureSection from './VoiceCaptureSection';
import { useTranslation } from 'react-i18next';

// ---------------------------------------------------------------------------
// Mobile STT/TTS configuration section.
// Mirrors the desktop AITab STT/TTS config surface so mobile has parity for
// dictation and text-to-speech settings. Kept as its own module to stay under
// the 400-line component gate.
// ---------------------------------------------------------------------------

export interface MobileSttTtsSectionProps {
  stt: {
    endpoint: string;
    api_key: string | null;
    model: string;
    diarize_model: string;
    language: string | null;
    diarize: boolean;
    auto_summarize: boolean;
    auto_identify: boolean;
  };
  updateStt: (patch: any) => void;
  tts: {
    endpoint: string;
    api_key: string | null;
    voice: string;
    format: string;
    speed: number;
  };
  updateTts: (patch: any) => void;
  voice?: { enabled: boolean; target: string };
  updateVoice?: (patch: any) => void;
}

export default function MobileSttTtsSection({ stt, updateStt, tts, updateTts, voice, updateVoice }: MobileSttTtsSectionProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);

  return (
    <Box sx={{ px: 2, pt: 3 }}>
      <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
        {t('settings.speech_amp_audio')}
      </Typography>
      <Divider sx={{ mb: 1 }} />
      <Box
        component="button"
        onClick={() => setExpanded(!expanded)}
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 0.5,
          bgcolor: 'transparent',
          border: 'none',
          cursor: 'pointer',
          color: 'text.secondary',
          fontSize: '0.8rem',
          fontWeight: 500,
          p: 0,
          '&:hover': { color: 'text.primary' },
        }}
      >
        <Typography variant="body2" color="text.secondary">
          {expanded ? t('settings.hide_dictation_voice_config') : t('settings.configure_dictation_voice')}
        </Typography>
      </Box>
      {expanded && (
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5, mt: 1 }}>
          {/* STT */}
          <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary', display: 'block', mt: 1 }}>
            {t('settings.speech_to_text_dictation')}
          </Typography>
          <TextField
            label={t('settings.stt_endpoint')}
            placeholder="http://localhost:9000"
            value={stt.endpoint}
            onChange={e => updateStt({ endpoint: e.target.value })}
            size="small"
          />
          <TextField
            label={t('settings.stt_api_key')}
            type="password"
            placeholder={stt.api_key?.includes('****') ? t('settings.key_saved_enter_new_value_to_change') : t('settings.optional')}
            value={stt.api_key?.includes('****') ? '' : (stt.api_key || '')}
            onChange={e => updateStt({ api_key: e.target.value || null })}
            size="small"
            sx={{ '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.8rem' } }}
          />
          <TextField
            label={t('settings.asr_model')}
            placeholder="whisper-1"
            value={stt.model}
            onChange={e => updateStt({ model: e.target.value })}
            size="small"
          />
          <TextField
            label={t('settings.diarization_model_label')}
            placeholder="pyannote/speaker-diarization"
            value={stt.diarize_model}
            onChange={e => updateStt({ diarize_model: e.target.value })}
            size="small"
          />
          <TextField
            label={t('settings.language')}
            placeholder="en"
            value={stt.language || ''}
            onChange={e => updateStt({ language: e.target.value || null })}
            size="small"
          />
          <FormControlLabel
            control={
              <Switch
                checked={stt.diarize}
                onChange={e => updateStt({ diarize: e.target.checked })}
              />
            }
            label={t('dictation.diarize_speakers')}
          />
          <FormControlLabel
            control={
              <Switch
                checked={stt.auto_summarize}
                onChange={e => updateStt({ auto_summarize: e.target.checked })}
              />
            }
            label={t('settings.auto_summarize_transcript')}
          />
          <FormControlLabel
            control={
              <Switch
                checked={stt.auto_identify}
                onChange={e => updateStt({ auto_identify: e.target.checked })}
              />
            }
            label={t('settings.auto_identify_speakers')}
          />
          <SttTestButton />

          {/* TTS */}
          <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary', display: 'block', mt: 2 }}>
            {t('settings.text_to_speech')}
          </Typography>
          <TextField
            label={t('settings.tts_endpoint')}
            placeholder="http://localhost:8081"
            value={tts.endpoint}
            onChange={e => updateTts({ endpoint: e.target.value })}
            size="small"
          />
          <TextField
            label={t('settings.tts_api_key')}
            type="password"
            placeholder={tts.api_key?.includes('****') ? t('settings.key_saved_enter_new_value_to_change') : t('settings.optional')}
            value={tts.api_key?.includes('****') ? '' : (tts.api_key || '')}
            onChange={e => updateTts({ api_key: e.target.value || null })}
            size="small"
            sx={{ '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.8rem' } }}
          />
          <TextField
            label={t('settings.voice')}
            placeholder="alloy"
            value={tts.voice}
            onChange={e => updateTts({ voice: e.target.value })}
            size="small"
          />
          <TextField
            label={t('settings.format')}
            placeholder="mp3"
            value={tts.format || ''}
            onChange={e => updateTts({ format: e.target.value })}
            size="small"
          />
          <TextField
            label={t('settings.speed_0_25_4_0')}
            type="number"
            value={tts.speed}
            onChange={e => updateTts({ speed: parseFloat(e.target.value) || 1.0 })}
            size="small"
            slotProps={{ htmlInput: { min: 0.25, max: 4, step: 0.05 } }}
          />
          <Typography variant="caption" color="text.disabled">
            {t('settings.speed_is_applied_at_play_time_a_value_of_1_0_is_normal_speed')}
          </Typography>
          <TtsTestButton />

          {/* Voice capture toggle */}
          <VoiceCaptureSection voice={voice} onVoiceChange={updateVoice} />
        </Box>
      )}
    </Box>
  );
}
