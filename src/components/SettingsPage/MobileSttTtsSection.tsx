import { useState } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import TextField from '@mui/material/TextField';
import Switch from '@mui/material/Switch';
import FormControlLabel from '@mui/material/FormControlLabel';
import { SttTestButton, TtsTestButton } from './AiTestButtons';

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
}

export default function MobileSttTtsSection({ stt, updateStt, tts, updateTts }: MobileSttTtsSectionProps) {
  const [expanded, setExpanded] = useState(false);

  return (
    <Box sx={{ px: 2, pt: 3 }}>
      <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1 }}>
        Speech &amp; Audio
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
          {expanded ? 'Hide dictation & voice config' : 'Configure dictation & voice'}
        </Typography>
      </Box>
      {expanded && (
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1.5, mt: 1 }}>
          {/* STT */}
          <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary', display: 'block', mt: 1 }}>
            Speech-to-Text (Dictation)
          </Typography>
          <TextField
            label="STT Endpoint"
            placeholder="http://localhost:9000"
            value={stt.endpoint}
            onChange={e => updateStt({ endpoint: e.target.value })}
            size="small"
          />
          <TextField
            label="STT API Key"
            type="password"
            placeholder={stt.api_key?.includes('****') ? 'Key saved - enter new value to change' : 'Optional'}
            value={stt.api_key?.includes('****') ? '' : (stt.api_key || '')}
            onChange={e => updateStt({ api_key: e.target.value || null })}
            size="small"
            sx={{ '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.8rem' } }}
          />
          <TextField
            label="ASR Model"
            placeholder="whisper-1"
            value={stt.model}
            onChange={e => updateStt({ model: e.target.value })}
            size="small"
          />
          <TextField
            label="Diarization Model"
            placeholder="pyannote/speaker-diarization"
            value={stt.diarize_model}
            onChange={e => updateStt({ diarize_model: e.target.value })}
            size="small"
          />
          <TextField
            label="Language"
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
            label="Diarize speakers"
          />
          <FormControlLabel
            control={
              <Switch
                checked={stt.auto_summarize}
                onChange={e => updateStt({ auto_summarize: e.target.checked })}
              />
            }
            label="Auto-summarize transcript"
          />
          <FormControlLabel
            control={
              <Switch
                checked={stt.auto_identify}
                onChange={e => updateStt({ auto_identify: e.target.checked })}
              />
            }
            label="Auto-identify speakers"
          />
          <SttTestButton />

          {/* TTS */}
          <Typography variant="caption" sx={{ fontWeight: 600, color: 'text.secondary', display: 'block', mt: 2 }}>
            Text-to-Speech
          </Typography>
          <TextField
            label="TTS Endpoint"
            placeholder="http://localhost:8081"
            value={tts.endpoint}
            onChange={e => updateTts({ endpoint: e.target.value })}
            size="small"
          />
          <TextField
            label="TTS API Key"
            type="password"
            placeholder={tts.api_key?.includes('****') ? 'Key saved - enter new value to change' : 'Optional'}
            value={tts.api_key?.includes('****') ? '' : (tts.api_key || '')}
            onChange={e => updateTts({ api_key: e.target.value || null })}
            size="small"
            sx={{ '& .MuiInputBase-input': { fontFamily: 'monospace', fontSize: '0.8rem' } }}
          />
          <TextField
            label="Voice"
            placeholder="alloy"
            value={tts.voice}
            onChange={e => updateTts({ voice: e.target.value })}
            size="small"
          />
          <TextField
            label="Format"
            placeholder="mp3"
            value={tts.format || ''}
            onChange={e => updateTts({ format: e.target.value })}
            size="small"
          />
          <TextField
            label="Speed (0.25–4.0)"
            type="number"
            value={tts.speed}
            onChange={e => updateTts({ speed: parseFloat(e.target.value) || 1.0 })}
            size="small"
            slotProps={{ htmlInput: { min: 0.25, max: 4, step: 0.05 } }}
          />
          <Typography variant="caption" color="text.disabled">
            Speed is applied at play time; a value of 1.0 is normal speed.
          </Typography>
          <TtsTestButton />
        </Box>
      )}
    </Box>
  );
}
