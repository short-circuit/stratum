import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';
import { runVoiceCapture, useVoiceCapture } from './VoiceCaptureButton.shared';
import { useSettingsStore } from '../stores/settingsStore';

const apiMocks = vi.hoisted(() => ({
  dictationStart: vi.fn(),
  dictationStop: vi.fn(),
  dictationCancel: vi.fn(),
  dictationTranscribe: vi.fn(),
  dictationInsertClipRef: vi.fn(),
  micPermissionStatus: vi.fn(),
  requestMicPermission: vi.fn(),
  getSettings: vi.fn(),
}));

vi.mock('../lib/commands', () => apiMocks);

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

const RECORDING = { recording_path: '/vault/recordings/memo.flac', duration_secs: 12 };
const START = { recording_path: '/vault/recordings/memo.flac', device_name: 'Mic', sample_rate: 48000 };

describe('runVoiceCapture', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    apiMocks.dictationStart.mockResolvedValue(START);
    apiMocks.dictationStop.mockResolvedValue(RECORDING);
  });

  it('starts, stops and transcribes, reporting inserted ids', async () => {
    apiMocks.dictationTranscribe.mockResolvedValue({
      inserted_block_ids: ['blk-1', 'blk-2'],
      turns: [],
      speaker_names: {},
      num_speakers: 0,
      diarized: false,
      summary: null,
      related: [],
      tags: [],
      clip_rel_path: 'recordings/memo.flac',
      duration_secs: 12,
      markdown: '## Voice memo',
    });
    const onInserted = vi.fn();
    const res = await runVoiceCapture('journals/today.md', {
      sttConfigured: true,
      onInserted,
    });

    expect(apiMocks.dictationStart).toHaveBeenCalledWith('journals/today.md');
    expect(apiMocks.dictationStop).toHaveBeenCalled();
    expect(apiMocks.dictationTranscribe).toHaveBeenCalledWith(
      '/vault/recordings/memo.flac',
      'journals/today.md',
      expect.objectContaining({}),
    );
    expect(res.outcome).toBe('inserted');
    expect(res.insertedIds).toEqual(['blk-1', 'blk-2']);
    expect(onInserted).toHaveBeenCalledWith(['blk-1', 'blk-2']);
  });

  it('falls back to inserting a clip reference when transcription fails', async () => {
    apiMocks.dictationTranscribe.mockRejectedValue(new Error('transient failure'));
    apiMocks.dictationInsertClipRef.mockResolvedValue(['blk-clip-1']);
    const onInserted = vi.fn();

    const res = await runVoiceCapture('journals/today.md', {
      sttConfigured: true,
      onInserted,
    });

    expect(res.outcome).toBe('fallback');
    expect(res.error).toContain('transient failure');
    expect(apiMocks.dictationInsertClipRef).toHaveBeenCalledWith(
      'journals/today.md',
      '/vault/recordings/memo.flac',
      12,
    );
    expect(onInserted).toHaveBeenCalledWith(['blk-clip-1']);
  });

  it('inserts a clip reference and signals needs_config when STT is unconfigured', async () => {
    apiMocks.dictationInsertClipRef.mockResolvedValue(['blk-clip-1']);
    const res = await runVoiceCapture('journals/today.md', { sttConfigured: false });

    expect(res.outcome).toBe('needs_config');
    expect(apiMocks.dictationTranscribe).not.toHaveBeenCalled();
    expect(apiMocks.dictationInsertClipRef).toHaveBeenCalled();
  });
});

describe('useVoiceCapture', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    apiMocks.dictationStart.mockResolvedValue(START);
    apiMocks.micPermissionStatus.mockResolvedValue({ granted: true, restricted: false });
  });

  it('cancel stops the recording and does not transcribe', async () => {
    const { result } = renderHook(() => useVoiceCapture('journals/today.md'));

    await waitFor(() => expect(result.current.permission.granted).toBe(true));

    await act(async () => {
      await result.current.start();
    });
    expect(result.current.stage).toBe('recording');

    await act(async () => {
      await result.current.cancel();
    });

    expect(apiMocks.dictationCancel).toHaveBeenCalled();
    expect(apiMocks.dictationTranscribe).not.toHaveBeenCalled();
    expect(result.current.stage).toBe('idle');
  });

  it('returns to idle after a successful insert so the one-tap flow is repeatable', async () => {
    vi.useFakeTimers();
    try {
      apiMocks.dictationStop.mockResolvedValue(RECORDING);
      apiMocks.getSettings.mockResolvedValue({
        stt: { endpoint: 'http://localhost:8081', model: 'whisper-1' },
      });
      apiMocks.dictationTranscribe.mockResolvedValue({
        inserted_block_ids: ['blk-1'],
        turns: [],
        speaker_names: {},
        num_speakers: 0,
        diarized: false,
        summary: null,
        related: [],
        tags: [],
        clip_rel_path: 'recordings/memo.flac',
        duration_secs: 12,
        markdown: '## Voice memo',
      });
      const { result } = renderHook(() => useVoiceCapture('journals/today.md'));

      // STT must appear configured for the happy path (settings store default
      // has an empty endpoint, which routes the hook to `needs_config`).
      await act(async () => {
        await useSettingsStore.getState().loadSettings();
      });
      useSettingsStore.getState().setSettings({
        ...useSettingsStore.getState().settings!,
        stt: { ...useSettingsStore.getState().settings!.stt, endpoint: 'http://localhost:8081' },
      });

      // Drain the mic-permission promise (it resolves via microtasks only).
      await act(async () => {
        await Promise.resolve();
        await Promise.resolve();
      });

      await act(async () => {
        await result.current.start();
      });
      expect(result.current.stage).toBe('recording');

      await act(async () => {
        await result.current.stop();
      });
      expect(result.current.stage).toBe('inserted');

      // After the success indicator window the button must be recordable again.
      await act(async () => {
        await vi.advanceTimersByTimeAsync(3000);
      });
      expect(result.current.stage).toBe('idle');

      // Starting again is allowed and re-enters recording.
      await act(async () => {
        await result.current.start();
      });
      expect(result.current.stage).toBe('recording');
    } finally {
      vi.useRealTimers();
    }
  });
});
