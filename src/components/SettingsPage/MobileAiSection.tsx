import { useTranslation } from 'react-i18next';
import Typography from '@mui/material/Typography';
import { SettingsSection, SettingsAccordion } from '../ui/settings';
import AiConfigSection from './AiConfigSection';
import SttConfigSection from './SttConfigSection';
import TtsConfigSection from './TtsConfigSection';
import VoiceCaptureSection from './VoiceCaptureSection';
import type { AITabProps } from './AITab';

// ---------------------------------------------------------------------------
// Mobile AI provider configuration (single collapsible accordion, E11.F3).
// Extracted from SettingsPage.mobile.tsx during the E11.F3 merge so the mobile
// settings screen stays under the 400-line component gate.
//
// Parity: mirrors the desktop AITab one-to-one — provider + STT/TTS + voice
// capture — inside one accordion; reuses the shared desktop content sections
// so the two surfaces cannot drift.
// ---------------------------------------------------------------------------

interface MobileAiSectionProps {
  ai: AITabProps['ai'];
  updateAi: AITabProps['onAiChange'];
  availableModels: string[];
  fetching: boolean;
  onFetchModels: () => void;
  onToggleModelCapability: (modelName: string, cap: string) => void;
  stt: NonNullable<AITabProps['stt']>;
  updateStt: NonNullable<AITabProps['onSttChange']>;
  tts: NonNullable<AITabProps['tts']>;
  updateTts: NonNullable<AITabProps['onTtsChange']>;
  voice: NonNullable<AITabProps['voice']>;
  updateVoice: (patch: { enabled?: boolean; target?: string }) => void;
}

export default function MobileAiSection({
  ai,
  updateAi,
  availableModels,
  fetching,
  onFetchModels,
  onToggleModelCapability,
  stt,
  updateStt,
  tts,
  updateTts,
  voice,
  updateVoice,
}: MobileAiSectionProps) {
  const { t } = useTranslation();

  return (
    <SettingsSection title={t('settings.ai')} variant="mobile" divider>
      <SettingsAccordion
        summary={open =>
          open ? (
            <Typography variant="body2" color="text.secondary">{t('settings.hide_provider_config')}</Typography>
          ) : (
            <Typography variant="body2" color="text.secondary">{t('settings.configure_ai_provider')}</Typography>
          )
        }
      >
        <AiConfigSection ai={ai} onAiChange={updateAi} availableModels={availableModels} fetching={fetching} onFetchModels={onFetchModels} onToggleModelCapability={onToggleModelCapability} />
        <SttConfigSection stt={stt} onSttChange={updateStt} />
        <TtsConfigSection tts={tts} onTtsChange={updateTts} />
        <VoiceCaptureSection voice={voice} onVoiceChange={updateVoice} />
      </SettingsAccordion>
    </SettingsSection>
  );
}
