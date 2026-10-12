import { useState } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import * as api from '../../lib/commands';
import { useTranslation } from 'react-i18next';

// ---------------------------------------------------------------------------
// Connection / playback test buttons for the AI settings tab.
// Extracted from AITab.tsx during the E6 sizing-gate refactor so the tab stays
// under the 400-line component gate.
// ---------------------------------------------------------------------------

export function SttTestButton() {
  const { t } = useTranslation();
  const [state, setState] = useState<'idle' | 'testing' | 'ok' | 'error'>('idle');
  const [detail, setDetail] = useState('');

  const test = async () => {
    setState('testing');
    setDetail('');
    try {
      const res = await api.sttTestConnection();
      if (res.ok) {
        setState('ok');
        setDetail(`${res.models.length} model(s) · ${res.latency_ms}ms`);
      } else {
        setState('error');
        setDetail(res.error || 'Failed');
      }
    } catch (e) {
      setState('error');
      setDetail(String(e));
    }
  };

  return (
    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
      <Button variant="outlined" size="small" onClick={test} disabled={state === 'testing'}>
        {state === 'testing' ? t('settings.testing') : t('settings.test_connection')}
      </Button>
      {state === 'ok' && (
        <Typography variant="caption" color="success.main">
          {t('settings.connected')} {detail}
        </Typography>
      )}
      {state === 'error' && (
        <Typography variant="caption" color="error.main">
          {detail}
        </Typography>
      )}
    </Box>
  );
}

export function TtsTestButton() {
  const { t } = useTranslation();
  const [state, setState] = useState<'idle' | 'generating' | 'error'>('idle');
  const [detail, setDetail] = useState('');

  const test = async () => {
    setState('generating');
    setDetail('');
    try {
      const res = await api.ttsGenerate(t('settings.hello_this_is_a_test_of_stratum_text_to_speech'));
      const audio = new Audio(`data:${res.mime};base64,${res.audio_b64}`);
      audio.play();
      setState('idle');
      setDetail(`${t('settings.generating_played')} ${(res.byte_len / 1024).toFixed(1)} KiB audio`);
    } catch (e) {
      setState('error');
      setDetail(String(e));
    }
  };

  return (
    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
      <Button
        variant="outlined"
        size="small"
        onClick={test}
        disabled={state === 'generating'}
      >
        {state === 'generating' ? t('settings.generating') : t('settings.test_play_voice')}
      </Button>
      {state === 'idle' && detail && (
        <Typography variant="caption" color="success.main">
          {detail}
        </Typography>
      )}
      {state === 'error' && (
        <Typography variant="caption" color="error.main">
          {detail}
        </Typography>
      )}
    </Box>
  );
}
