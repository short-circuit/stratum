import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Fab from '@mui/material/Fab';
import CircularProgress from '@mui/material/CircularProgress';
import Paper from '@mui/material/Paper';
import Typography from '@mui/material/Typography';
import MicIcon from '@mui/icons-material/Mic';
import StopIcon from '@mui/icons-material/Stop';
import CloseIcon from '@mui/icons-material/Close';
import SettingsIcon from '@mui/icons-material/Settings';
import { useVoiceCapture, formatTimer } from './VoiceCaptureButton.shared';
import type { VoiceCaptureButtonProps } from './VoiceCaptureButton.desktop';

/** Mobile bottom-right voice-capture FAB plus a fixed permission/status surface. */
export default function VoiceCaptureButtonMobile({
  targetPath,
  onInserted,
}: VoiceCaptureButtonProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const vc = useVoiceCapture(targetPath, { onInserted });

  return (
    <>
      {vc.permission.denied && (
        <Paper
          elevation={3}
          sx={{
            position: 'fixed',
            top: 'calc(var(--safe-area-inset-top, 0px) + 8px)',
            left: 12,
            right: 12,
            zIndex: 1300,
            px: 1.5,
            py: 1,
            display: 'flex',
            alignItems: 'center',
            gap: 1,
          }}
        >
          <Typography variant="body2" sx={{ flex: 1 }}>
            {t('voice_capture.denied_banner')}
          </Typography>
          <Button size="small" onClick={vc.permission.requestPermission} sx={{ textTransform: 'none' }}>
            {t('voice_capture.enable')}
          </Button>
        </Paper>
      )}

      {vc.stage === 'fallback' && (
        <Paper
          elevation={3}
          sx={{
            position: 'fixed',
            top: 'calc(var(--safe-area-inset-top, 0px) + 8px)',
            left: 12,
            right: 12,
            zIndex: 1300,
            px: 1.5,
            py: 1,
          }}
        >
          <Typography variant="body2">{t('voice_capture.error_fallback_inserted')}</Typography>
          <Box sx={{ display: 'flex', gap: 1, mt: 0.5 }}>
            <Button size="small" onClick={vc.retry} sx={{ textTransform: 'none' }}>
              {t('voice_capture.retry_transcription')}
            </Button>
            <Button size="small" onClick={vc.dismiss} sx={{ textTransform: 'none' }}>
              {t('common.cancel')}
            </Button>
          </Box>
        </Paper>
      )}

      {vc.stage === 'needs_config' && (
        <Paper
          elevation={3}
          sx={{
            position: 'fixed',
            top: 'calc(var(--safe-area-inset-top, 0px) + 8px)',
            left: 12,
            right: 12,
            zIndex: 1300,
            px: 1.5,
            py: 1,
          }}
        >
          <Typography variant="body2">{t('voice_capture.configure_stt_prompt')}</Typography>
          <Button
            size="small"
            startIcon={<SettingsIcon />}
            onClick={() => navigate('/settings')}
            sx={{ textTransform: 'none', mt: 0.5 }}
          >
            {t('voice_capture.configure_stt')}
          </Button>
        </Paper>
      )}

      {vc.stage === 'error' && (
        <Paper
          elevation={3}
          sx={{
            position: 'fixed',
            top: 'calc(var(--safe-area-inset-top, 0px) + 8px)',
            left: 12,
            right: 12,
            zIndex: 1300,
            px: 1.5,
            py: 1,
          }}
        >
          <Typography variant="body2" color="error">
            {vc.error ?? t('voice_capture.error_retry')}
          </Typography>
        </Paper>
      )}

      {vc.stage === 'recording' && (
        <Paper
          elevation={4}
          sx={{
            position: 'fixed',
            bottom: 'calc(var(--safe-area-inset-bottom, 0px) + 84px)',
            right: 16,
            zIndex: 1200,
            px: 1.5,
            py: 1,
            display: 'flex',
            alignItems: 'center',
            gap: 1,
          }}
        >
          <Chip
            icon={<MicIcon />}
            label={formatTimer(vc.elapsed)}
            color="error"
            size="small"
            aria-label={t('voice_capture.recording')}
          />
          <Fab size="small" color="primary" onClick={vc.stop} aria-label={t('voice_capture.stop_save')}>
            <StopIcon />
          </Fab>
          <Fab size="small" color="inherit" onClick={vc.cancel} aria-label={t('voice_capture.cancel')}>
            <CloseIcon />
          </Fab>
        </Paper>
      )}

      {vc.stage === 'transcribing' && (
        <Paper
          elevation={4}
          sx={{
            position: 'fixed',
            bottom: 'calc(var(--safe-area-inset-bottom, 0px) + 84px)',
            right: 16,
            zIndex: 1200,
            px: 1.5,
            py: 1,
            display: 'flex',
            alignItems: 'center',
            gap: 1,
          }}
        >
          <CircularProgress size={16} />
          <Typography variant="body2">{vc.stageLabel || t('dictation.working')}</Typography>
        </Paper>
      )}

      {(vc.stage === 'idle' || vc.stage === 'inserted') && (
        <Fab
          color="primary"
          aria-label={t('voice_capture.record')}
          onClick={vc.start}
          sx={{
            position: 'fixed',
            bottom: 'calc(var(--safe-area-inset-bottom, 0px) + 84px)',
            right: 16,
            zIndex: 1200,
          }}
        >
          <MicIcon />
        </Fab>
      )}
      {vc.stage === 'inserted' && (
        <Chip
          size="small"
          color="success"
          label={t('voice_capture.memo_inserted')}
          sx={{
            position: 'fixed',
            bottom: 'calc(var(--safe-area-inset-bottom, 0px) + 148px)',
            right: 16,
            zIndex: 1200,
          }}
        />
      )}
    </>
  );
}
