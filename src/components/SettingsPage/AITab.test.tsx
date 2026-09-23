import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import AITab from './AITab';

// The test buttons (SttTestButton/TtsTestButton) call the Tauri command surface;
// stub the commands module so the tab renders in jsdom without a runtime.
vi.mock('../../lib/commands', () => ({
  sttTestConnection: vi.fn().mockResolvedValue({ ok: false, models: [], latency_ms: 0, error: 'stub' }),
  ttsGenerate: vi.fn().mockResolvedValue({ audio_b64: '', mime: 'audio/mpeg', byte_len: 0, model: 'stub' }),
}));

function makeProps(overrides: any = {}) {
  return {
    ai: {
      provider: 'ollama',
      endpoint: null,
      api_key: null,
      api_key_from_env: false,
      model: '',
      models: [],
      rag_enabled: false,
      rag_chunk_count: 3,
      embedding_dimensions: 0,
      use_llm_gateway_and_auth: false,
    },
    onAiChange: vi.fn(),
    availableModels: [],
    fetching: false,
    onFetchModels: vi.fn(),
    onToggleModelCapability: vi.fn(),
    stt: {
      endpoint: '',
      api_key: null,
      model: '',
      diarize_model: '',
      language: null,
      diarize: false,
      auto_summarize: false,
      auto_identify: false,
      use_llm_gateway_and_auth: false,
    },
    onSttChange: vi.fn(),
    tts: {
      endpoint: '',
      api_key: null,
      voice: 'alloy',
      format: 'mp3',
      speed: 1.0,
      use_llm_gateway_and_auth: false,
    },
    onTtsChange: vi.fn(),
    ...overrides,
  };
}

// The checkbox label is repeated in each capability section; the ordering is
// stable (AI config, STT, TTS) so index-based queries assert the right binding.
describe('AITab — use_llm_gateway_and_auth checkboxes', () => {
  it('renders all three capability checkboxes unchecked by default', () => {
    render(<AITab {...makeProps()} />);

    const boxes = screen.getAllByLabelText('Use gateway and auth from LLM');
    expect(boxes).toHaveLength(3);
    for (const b of boxes) {
      expect(b).not.toBeChecked();
    }
  });

  it('binds onAiChange to ai.use_llm_gateway_and_auth when toggled', () => {
    const props = makeProps();
    render(<AITab {...props} />);
    const ragBox = screen.getAllByLabelText('Use gateway and auth from LLM')[0];
    fireEvent.click(ragBox);
    expect(props.onAiChange).toHaveBeenCalledWith({ use_llm_gateway_and_auth: true });
  });

  it('binds onSttChange to stt.use_llm_gateway_and_auth when toggled', () => {
    const props = makeProps();
    render(<AITab {...props} />);
    const sttBox = screen.getAllByLabelText('Use gateway and auth from LLM')[1];
    fireEvent.click(sttBox);
    expect(props.onSttChange).toHaveBeenCalledWith({ use_llm_gateway_and_auth: true });
  });

  it('binds onTtsChange to tts.use_llm_gateway_and_auth when toggled', () => {
    const props = makeProps();
    render(<AITab {...props} />);
    const ttsBox = screen.getAllByLabelText('Use gateway and auth from LLM')[2];
    fireEvent.click(ttsBox);
    expect(props.onTtsChange).toHaveBeenCalledWith({ use_llm_gateway_and_auth: true });
  });

  it('reflects a persisted true value as checked (round-trip), and renders TTS checkbox active', () => {
    const props = makeProps({
      ai: { ...makeProps().ai, use_llm_gateway_and_auth: true },
    });
    render(<AITab {...props} />);

    // RAG checkbox reflects the persisted value.
    expect(screen.getAllByLabelText('Use gateway and auth from LLM')[0]).toBeChecked();

    // The TTS checkbox renders enabled (not gated/disabled) per the backend decision.
    const ttsBox = screen.getAllByLabelText('Use gateway and auth from LLM')[2];
    expect(ttsBox).not.toBeDisabled();
  });
});
