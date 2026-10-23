import Box from '@mui/material/Box';
import { useTranslation } from 'react-i18next';
import { SettingsSection, SettingsRow } from '../ui/settings';

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
    <SettingsSection title={t('voice_capture.settings_toggle_label')} sx={{ mt: 3 }}>
      <Box sx={{ maxWidth: 480 }}>
        <SettingsRow
          label={t('voice_capture.enable')}
          checked={voice?.enabled ?? true}
          onCheckedChange={v => onVoiceChange?.({ enabled: v })}
          description={t('voice_capture.settings_target_today')}
        />
      </Box>
    </SettingsSection>
  );
}
