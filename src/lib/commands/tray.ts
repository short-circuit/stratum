import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { TrayStatusDto } from '../types';

/** Read current tray status from the Rust backend (desktop; disabled on mobile). */
export async function getTrayStatus(): Promise<TrayStatusDto> {
  return invoke('get_tray_status');
}

/** Persist a quick-capture note as a block into today's journal or the inbox page. */
export async function quickCaptureSave(text: string): Promise<{ page_path: string; block_id: string }> {
  return invoke('quick_capture_save', { text });
}

/** Subscribe to the tray/global-shortcut "open quick capture" event. */
export async function onQuickCaptureOpen(cb: (voice: boolean) => void): Promise<() => void> {
  return listen('quick-capture:open', (e) => cb(Boolean((e.payload as { voice?: unknown } | undefined)?.voice)));
}

/** Subscribe to the tray "Quit" event (frontend saves library best-effort first). */
export async function onAppQuit(cb: () => void): Promise<() => void> {
  return listen('app:quit', cb);
}
