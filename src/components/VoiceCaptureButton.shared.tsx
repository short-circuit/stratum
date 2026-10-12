import { useCallback, useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';
import * as api from '../lib/commands';
import { getPlatform } from '../lib/platform';
import { useSettingsStore } from '../stores/settingsStore';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type VoiceStage =
  | 'idle'
  | 'recording'
  | 'transcribing'
  | 'inserted'
  | 'needs_config'
  | 'fallback'
  | 'error';

export interface RunVoiceCaptureOptions {
  diarize?: boolean;
  summarize?: boolean;
  identify?: boolean;
  /** Override for automation/unit tests; defaults to the STT endpoint being set. */
  sttConfigured?: boolean;
  onStage?: (stage: VoiceStage) => void;
  onInserted?: (ids: string[]) => void;
}

export interface RunVoiceCaptureResult {
  outcome: 'inserted' | 'needs_config' | 'fallback' | 'error';
  insertedIds: string[];
  error?: string;
}

export interface UseVoiceCaptureOptions {
  diarize?: boolean;
  summarize?: boolean;
  identify?: boolean;
  onInserted?: (ids: string[]) => void;
}

/** Map a backing `dictation:stage` payload to the shared UI stage labels. */
export const VOICE_STAGE_LABEL_KEYS: Record<string, string> = {
  transcribing: 'dictation.transcribing',
  diarizing: 'dictation.separating_speakers',
  identifying: 'dictation.identifying_voices',
  summarizing: 'dictation.summarizing',
  linking: 'dictation.linking_notes_tags',
};

// ---------------------------------------------------------------------------
// Pipeline (single code path, reused by the hook and unit tests)
// ---------------------------------------------------------------------------

/**
 * Run the shared voice pipeline after a clip has been saved: transcribe, or on
 * failure fall back to inserting a clip-reference placeholder so the recording
 * is never lost. When STT is not configured the clip-reference is inserted
 * and the caller is signalled to prompt for configuration.
 */
export async function runVoiceCapture(
  targetPath: string,
  options: RunVoiceCaptureOptions = {},
): Promise<RunVoiceCaptureResult> {
  options.onStage?.('recording');
  await api.dictationStart(targetPath);
  options.onStage?.('recording');
  const stop = await api.dictationStop();
  return finalizeVoiceMemo(stop.recording_path, stop.duration_secs, targetPath, options);
}

/** Common tail shared by the interactive hook and `runVoiceCapture`. */
export async function finalizeVoiceMemo(
  recordingPath: string,
  durationSecs: number,
  targetPath: string,
  options: RunVoiceCaptureOptions = {},
): Promise<RunVoiceCaptureResult> {
  const { onStage, onInserted, sttConfigured = true, diarize, summarize, identify } = options;
  if (!sttConfigured) {
    const ids = await api.dictationInsertClipRef(targetPath, recordingPath, durationSecs);
    onInserted?.(ids);
    onStage?.('needs_config');
    return { outcome: 'needs_config', insertedIds: ids };
  }
  onStage?.('transcribing');
  try {
    const res = await api.dictationTranscribe(recordingPath, targetPath, {
      diarize,
      summarize,
      identify,
    });
    onInserted?.(res.inserted_block_ids);
    onStage?.('inserted');
    return { outcome: 'inserted', insertedIds: res.inserted_block_ids };
  } catch (e) {
    const msg = String(e);
    let insertedIds: string[] = [];
    try {
      insertedIds = await api.dictationInsertClipRef(targetPath, recordingPath, durationSecs);
      onInserted?.(insertedIds);
    } catch {
      // Even the fallback insertion failed — surface the original error only.
    }
    onStage?.('fallback');
    return { outcome: 'fallback', insertedIds, error: msg };
  }
}

// ---------------------------------------------------------------------------
// Mic permission
// ---------------------------------------------------------------------------

export interface UseMicPermission {
  granted: boolean;
  denied: boolean;
  checking: boolean;
  /** True on Android/iOS where a system dialog may be shown. */
  isMobile: boolean;
  requestPermission: () => Promise<void>;
}

/** Track microphone permission, requesting it lazily from the platform. */
export function useMicPermission(): UseMicPermission {
  const { t } = useTranslation();
  void t;
  const isMobile = getPlatform().os === 'android' || getPlatform().os === 'ios';
  const [granted, setGranted] = useState(true);
  const [denied, setDenied] = useState(false);
  const [checking, setChecking] = useState(true);

  useEffect(() => {
    let active = true;
    api
      .micPermissionStatus()
      .then((dto) => {
        if (!active) return;
        setGranted(dto.granted);
        setDenied(!dto.granted);
      })
      .catch(() => {
        if (!active) return;
        setGranted(true);
        setDenied(false);
      })
      .finally(() => {
        if (active) setChecking(false);
      });
    return () => {
      active = false;
    };
  }, []);

  const requestPermission = useCallback(async () => {
    setChecking(true);
    try {
      const g = await api.requestMicPermission();
      setGranted(g);
      setDenied(!g);
    } catch {
      setGranted(true);
      setDenied(false);
    } finally {
      setChecking(false);
    }
  }, []);

  return { granted, denied, checking, isMobile, requestPermission };
}

// ---------------------------------------------------------------------------
// Capture hook
// ---------------------------------------------------------------------------

export interface UseVoiceCapture {
  stage: VoiceStage;
  elapsed: number;
  error: string | null;
  stageLabel: string;
  permission: UseMicPermission;
  start: () => Promise<void>;
  stop: () => Promise<void>;
  cancel: () => Promise<void>;
  retry: () => Promise<void>;
  dismiss: () => void;
}

/** State machine driving a one-tap voice capture into `targetPath`. */
export function useVoiceCapture(
  targetPath: string,
  options: UseVoiceCaptureOptions = {},
): UseVoiceCapture {
  const { t } = useTranslation();
  const sttConfigured = useSettingsStore((s) => !!s.settings?.stt?.endpoint);
  const { diarize, summarize, identify, onInserted } = options;
  const [stage, setStage] = useState<VoiceStage>('idle');
  const [elapsed, setElapsed] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [stageName, setStageName] = useState('');
  const [recordingPath, setRecordingPath] = useState<string | null>(null);
  const timerRef = useRef<number | null>(null);
  // Pending auto-reset after a successful insert — returns the button to idle
  // so the one-tap flow is repeatable (the success indicator is transient).
  const resetTimerRef = useRef<number | null>(null);

  const permission = useMicPermission();

  useEffect(() => {
    let un: (() => void) | undefined;
    listen<{ stage: string }>('dictation:stage', (e) => setStageName(e.payload.stage)).then((f) => {
      un = f;
    });
    return () => un?.();
  }, []);

  useEffect(() => () => {
    if (timerRef.current !== null) window.clearInterval(timerRef.current);
    if (resetTimerRef.current !== null) window.clearTimeout(resetTimerRef.current);
  }, []);

  const clearTimer = () => {
    if (timerRef.current !== null) {
      window.clearInterval(timerRef.current);
      timerRef.current = null;
    }
  };

  /** Cancel a pending 'inserted' → 'idle' auto-reset (e.g. user started again). */
  const clearResetTimer = () => {
    if (resetTimerRef.current !== null) {
      window.clearTimeout(resetTimerRef.current);
      resetTimerRef.current = null;
    }
  };

  /** Show the transient success indicator, then return the button to idle. */
  const scheduleReset = useCallback(() => {
    clearResetTimer();
    resetTimerRef.current = window.setTimeout(() => {
      resetTimerRef.current = null;
      setStage('idle');
      setRecordingPath(null);
    }, 2500);
  }, []);

  const start = useCallback(async () => {
    if (!permission.granted) {
      await permission.requestPermission();
      if (!permission.granted) return;
    }
    clearResetTimer();
    setError(null);
    setRecordingPath(null);
    setElapsed(0);
    try {
      const dto = await api.dictationStart(targetPath);
      setRecordingPath(dto.recording_path);
      setStage('recording');
      timerRef.current = window.setInterval(() => setElapsed((e) => e + 1), 1000);
    } catch (e) {
      setError(String(e));
      setStage('error');
    }
  }, [targetPath, permission]);

  const stop = useCallback(async () => {
    clearTimer();
    setStage('transcribing');
    try {
      const dto = await api.dictationStop();
      const res = await finalizeVoiceMemo(dto.recording_path, dto.duration_secs, targetPath, {
        diarize,
        summarize,
        identify,
        sttConfigured,
        onStage: setStage,
        onInserted,
      });
      if (res.outcome === 'fallback') setError(res.error ?? null);
      setElapsed(0);
      if (res.outcome === 'inserted') scheduleReset();
    } catch (e) {
      setError(String(e));
      setStage('error');
    }
  }, [targetPath, sttConfigured, diarize, summarize, identify, onInserted, scheduleReset]);

  const cancel = useCallback(async () => {
    clearTimer();
    try {
      await api.dictationCancel();
    } catch {
      // nothing in flight — safe to ignore
    }
    setStage('idle');
    setRecordingPath(null);
    setElapsed(0);
    setError(null);
  }, []);

  const retry = useCallback(async () => {
    setError(null);
    if (!recordingPath) return;
    setStage('transcribing');
    try {
      const res = await api.dictationTranscribe(recordingPath, targetPath, {
        diarize,
        summarize,
        identify,
      });
      onInserted?.(res.inserted_block_ids);
      setStage('inserted');
      setElapsed(0);
      scheduleReset();
    } catch (e) {
      setError(String(e));
      setStage('fallback');
    }
  }, [recordingPath, targetPath, diarize, summarize, identify, onInserted, scheduleReset]);

  const dismiss = useCallback(() => {
    setError(null);
    setStage('idle');
  }, []);

  const stageLabel = VOICE_STAGE_LABEL_KEYS[stageName]
    ? t(VOICE_STAGE_LABEL_KEYS[stageName])
    : stage === 'transcribing'
      ? t('dictation.working')
      : '';

  return {
    stage,
    elapsed,
    error,
    stageLabel,
    permission,
    start,
    stop,
    cancel,
    retry,
    dismiss,
  };
}

/** Format seconds as `m:ss` for the recording timer. */
export function formatTimer(secs: number): string {
  const s = Math.max(0, Math.round(secs));
  const m = Math.floor(s / 60);
  return `${m}:${String(s % 60).padStart(2, '0')}`;
}
