import Box from '@mui/material/Box';
import Divider from '@mui/material/Divider';
import FormControlLabel from '@mui/material/FormControlLabel';
import Switch from '@mui/material/Switch';
import Typography from '@mui/material/Typography';
import { useTranslation } from 'react-i18next';

export interface VoiceCaptureSectionProps {
  voice?: { enabled: boolean; target: string };
  onVoiceChange?: (patch: { enabled?: boolean; target?: string }) => void;
}

/** Settings section for the one-tap voice-capture feature (E9.F4). */
export default function VoiceCaptureSection({
  voice,
  onVoiceChange,
}: VoiceCaptureSectionProps) {
  const { t } = useTranslation();

  return (
    <Box sx={{ mt: 3 }}>
      <Divider sx={{ mb: 2 }} />
      <Typography variant="subtitle2" sx={{ mb: 1.5, color: 'text.secondary' }}>
        {t('voice_capture.settings_toggle_label')}
      </Typography>
      <Box sx={{ maxWidth: 480 }}>
        <FormControlLabel
          control={
            <Switch
              checked={voice?.enabled ?? true}
              onChange={(e) => onVoiceChange?.({ enabled: e.target.checked })}
            />
          }
          label={t('voice_capture.enable')}
        />
        <Typography variant="caption" sx={{ mt: 0.5, display: 'block', color: 'text.secondary' }}>
          {t('voice_capture.settings_target_today')}
        </Typography>
      </Box>
    </Box>
  );
}
