import { invoke } from '@tauri-apps/api/core';
import { getPlatform } from '../platform';
import type {
  AiAction,
  AiTransformResult,
  ResearchResult,
  RagQueryResultDto,
  KanbanBlockDto,
  KanbanDataDto,
  DictationStartDto,
  DictationStopDto,
  DictationOptsDto,
  DictationResultDto,
  MicPermissionDto,
  SpeakerDto,
  SpeakerAssignDto,
  SttTestDto,
  TtsResult,
} from '../types';

// --- AI ---

export async function aiTransformBlock(
  text: string,
  action: AiAction,
  pagePath?: string,
): Promise<AiTransformResult> {
  return invoke('ai_transform_block', { text, action, pagePath });
}

export async function aiResearch(query: string): Promise<ResearchResult> {
  return invoke('ai_research', { query });
}

export async function aiRagQuery(question: string): Promise<RagQueryResultDto> {
  return invoke('ai_rag_query', { question });
}

export async function aiInterlinkNotes(
  text: string,
  pagePath?: string,
): Promise<AiTransformResult> {
  return invoke('ai_interlink_notes', { text, pagePath });
}

export async function generateMermaid(prompt: string): Promise<string> {
  const result = await invoke<AiTransformResult>('generate_mermaid', { prompt });
  return result.content;
}

// --- Templates ---

export async function listTemplates(): Promise<{
  name: string; path: string; content: string; description?: string;
}[]> {
  return invoke('list_templates');
}

export async function saveTemplate(name: string, content: string): Promise<void> {
  return invoke('save_template', { name, content });
}

export async function applyTemplate(
  templateName: string,
  targetPage: string,
  variables: [string, string][],
): Promise<string> {
  return invoke('apply_template', { templateName, targetPage, variables });
}

export async function installStarterTemplates(names?: string[]): Promise<string[]> {
  return invoke('install_starter_templates', { names: names ?? null });
}

export async function installStarterSample(): Promise<string[]> {
  return invoke('install_starter_sample');
}

// --- Export ---

export async function exportHtml(outputDir: string): Promise<{
  output_dir: string; pages_exported: number; assets_copied: number;
}> {
  return invoke('export_html', { outputDir });
}

export async function exportJson(outputDir: string): Promise<{
  output_dir: string; pages_exported: number; assets_copied: number;
}> {
  return invoke('export_json', { outputDir });
}

// --- Flashcards ---

export async function generateFlashcards(): Promise<{
  id: string; front: string; back: string; page_path: string; ease_factor: number; interval_days: number; repetitions: number; next_review: string;
}[]> {
  return invoke('generate_flashcards');
}

export async function reviewCard(cardId: string, quality: number, pagePath: string): Promise<{
  id: string; front: string; back: string; page_path: string; ease_factor: number; interval_days: number; repetitions: number; next_review: string;
}> {
  return invoke('review_card', { cardId, quality, pagePath });
}

// --- Whiteboards ---

export async function listWhiteboards(): Promise<{
  name: string; path: string; content: string;
}[]> {
  return invoke('list_whiteboards');
}

export async function saveWhiteboard(name: string, content: string): Promise<void> {
  return invoke('save_whiteboard', { name, content });
}

export async function loadWhiteboard(name: string): Promise<string> {
  return invoke('load_whiteboard', { name });
}

export async function renameWhiteboard(oldName: string, newName: string): Promise<void> {
  return invoke('rename_whiteboard', { oldName, newName });
}

export async function deleteWhiteboard(name: string): Promise<void> {
  return invoke('delete_whiteboard', { name });
}

// --- Library ---

export async function saveLibrary(content: string): Promise<void> {
  return invoke('save_library', { content });
}

export async function loadLibrary(): Promise<string> {
  return invoke('load_library');
}

export async function loadExtraLibraries(): Promise<string> {
  return invoke('load_extra_libraries');
}

// --- Kanban ---

export async function getKanbanBlocks(): Promise<KanbanDataDto> {
  return invoke('get_kanban_blocks');
}

export async function createKanbanBlock(
  content: string,
  marker: string,
): Promise<KanbanBlockDto> {
  return invoke('create_kanban_block', { content, marker });
}

// --- Dictation (voice memos) ---

export async function dictationStart(pagePath: string): Promise<DictationStartDto> {
  return invoke('dictation_start', { pagePath });
}

export async function dictationStop(): Promise<DictationStopDto> {
  return invoke('dictation_stop');
}

export async function dictationCancel(): Promise<void> {
  return invoke('dictation_cancel');
}

export async function dictationTranscribe(
  recordingPath: string,
  pagePath: string,
  opts?: DictationOptsDto,
): Promise<DictationResultDto> {
  return invoke('dictation_transcribe', { recordingPath, pagePath, opts });
}

export async function speakerList(): Promise<SpeakerDto[]> {
  return invoke('speaker_list');
}

export async function speakerAssign(
  recordingPath: string,
  speakerId: string,
  name: string,
  enroll: boolean,
): Promise<SpeakerAssignDto> {
  return invoke('speaker_assign', { recordingPath, speakerId, name, enroll });
}

export async function speakerDelete(name: string): Promise<void> {
  return invoke('speaker_delete', { name });
}

export async function sttTestConnection(): Promise<SttTestDto> {
  return invoke('stt_test_connection');
}

// --- Voice capture (E9.F4) ---

/**
 * Report microphone permission status.
 *
 * Desktop/iOS always report granted. On Android the authoritative state lives
 * in the injected `StratumMic` JS bridge (see MainActivity.kt), so we overlay
 * the command result with the bridge's `isMicGranted()` when present.
 */
export async function micPermissionStatus(): Promise<MicPermissionDto> {
  const dto = await invoke<MicPermissionDto>('mic_permission_status');
  const os = getPlatform().os;
  const bridge = (window as any).StratumMic;
  if (os === 'android' && bridge && typeof bridge.isMicGranted === 'function') {
    return { ...dto, granted: !!bridge.isMicGranted() };
  }
  return dto;
}

/**
 * Request microphone permission, resolving to `granted` when the flow ends.
 *
 * Desktop and iOS return `true` immediately. On Android the system dialog is
 * launched via the `StratumMic` bridge; the Kotlin launcher writes the result
 * back by calling `window.__stratumMicPermissionResolve(granted)`, which
 * resolves the returned promise so the UI never hangs if the dialog is shown.
 */
export async function requestMicPermission(): Promise<boolean> {
  const os = getPlatform().os;
  if (os !== 'android' && os !== 'ios') return true;
  const bridge = (window as any).StratumMic;
  if (os === 'android' && bridge && typeof bridge.requestMicPermission === 'function') {
    return new Promise((resolve) => {
      (window as any).__stratumMicPermissionResolve = (g: boolean) => resolve(!!g);
      bridge.requestMicPermission();
    });
  }
  // iOS: AVAudioSession prompts on first stream; treat as granted.
  return true;
}

/** Insert a placeholder memo referencing a saved clip (transcription fallback). */
export async function dictationInsertClipRef(
  pagePath: string,
  clipRelPath: string,
  durationSecs: number,
): Promise<string[]> {
  return invoke('dictation_insert_clip_ref', {
    pagePath,
    clipRelPath,
    durationSecs,
  });
}

// --- Text-to-speech ---

export async function ttsGenerate(text: string): Promise<TtsResult> {
  return invoke('tts_synthesize', { text });
}

export async function ttsSpeak(text: string): Promise<TtsResult> {
  return invoke('tts_speak', { text });
}
