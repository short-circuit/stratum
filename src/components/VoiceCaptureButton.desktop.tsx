import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import CircularProgress from '@mui/material/CircularProgress';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import MicIcon from '@mui/icons-material/Mic';
import StopIcon from '@mui/icons-material/Stop';
import CloseIcon from '@mui/icons-material/Close';
import SettingsIcon from '@mui/icons-material/Settings';
import { useVoiceCapture, formatTimer } from './VoiceCaptureButton.shared';

export interface VoiceCaptureButtonProps {
  targetPath: string;
  onInserted?: (ids: string[]) => void;
}

/** Desktop header voice-capture button (compact inline control, no popover). */
export default function VoiceCaptureButtonDesktop({
  targetPath,
  onInserted,
}: VoiceCaptureButtonProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const vc = useVoiceCapture(targetPath, { onInserted });

  if (vc.stage === 'idle') {
    return (
      <Button
        size="small"
        variant="outlined"
        startIcon={<MicIcon />}
        onClick={vc.start}
        aria-label={t('voice_capture.record')}
        sx={{ textTransform: 'none', whiteSpace: 'nowrap' }}
      >
        {t('voice_capture.record')}
      </Button>
    );
  }

  if (vc.stage === 'recording') {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
        <Chip
          icon={<MicIcon />}
          label={`${t('voice_capture.recording')} ${formatTimer(vc.elapsed)}`}
          color="error"
          size="small"
          aria-label={t('voice_capture.recording')}
        />
        <Button size="small" variant="contained" startIcon={<StopIcon />} onClick={vc.stop} sx={{ textTransform: 'none' }}>
          {t('voice_capture.stop_save')}
        </Button>
        <IconButton size="small" onClick={vc.cancel} aria-label={t('voice_capture.cancel')}>
          <CloseIcon fontSize="small" />
        </IconButton>
      </Box>
    );
  }

  if (vc.stage === 'transcribing') {
    return (
      <Chip
        icon={<CircularProgress size={12} />}
        label={vc.stageLabel || t('dictation.working')}
        size="small"
      />
    );
  }

  if (vc.stage === 'needs_config') {
    return (
      <Button
        size="small"
        variant="outlined"
        startIcon={<SettingsIcon />}
        onClick={() => navigate('/settings')}
        aria-label={t('voice_capture.configure_stt')}
        sx={{ textTransform: 'none' }}
      >
        {t('voice_capture.configure_stt')}
      </Button>
    );
  }

  if (vc.stage === 'fallback') {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
        <Chip size="small" label={t('voice_capture.error_fallback_inserted')} color="warning" />
        <Button size="small" onClick={vc.retry} sx={{ textTransform: 'none' }}>
          {t('voice_capture.retry_transcription')}
        </Button>
      </Box>
    );
  }

  if (vc.stage === 'error') {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
        <Typography variant="caption" color="error">
          {vc.error ?? t('voice_capture.error_retry')}
        </Typography>
        <Button size="small" onClick={vc.dismiss} sx={{ textTransform: 'none' }}>
          {t('common.back')}
        </Button>
      </Box>
    );
  }

  // inserted
  return (
    <Chip size="small" color="success" label={t('voice_capture.memo_inserted')} />
  );
}
