import { useTranslation } from 'react-i18next';
import { SettingsSection } from '../ui/settings';
import AiConfigSection from './AiConfigSection';
import SttConfigSection from './SttConfigSection';
import TtsConfigSection from './TtsConfigSection';
import VoiceCaptureSection from './VoiceCaptureSection';
import type { DiscoveredEndpoint } from '../../lib/types';

export interface AITabProps {
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
    extra_local_hosts?: string[];
    last_discovered?: DiscoveredEndpoint[];
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
  voice?: { enabled: boolean; target: string };
  onVoiceChange?: (patch: { enabled?: boolean; target?: string }) => void;
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
  voice,
  onVoiceChange,
}: AITabProps) {
  const { t } = useTranslation();

  return (
    <SettingsSection title={t('settings.ai')}>
      <AiConfigSection
        ai={ai}
        onAiChange={onAiChange}
        availableModels={availableModels}
        fetching={fetching}
        onFetchModels={onFetchModels}
        onToggleModelCapability={onToggleModelCapability}
      />
      {stt && onSttChange && <SttConfigSection stt={stt} onSttChange={onSttChange} />}
      {tts && onTtsChange && <TtsConfigSection tts={tts} onTtsChange={onTtsChange} />}
      <VoiceCaptureSection voice={voice} onVoiceChange={onVoiceChange} />
    </SettingsSection>
  );
}
