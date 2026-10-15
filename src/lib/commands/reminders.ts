import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ReminderDto, ReminderEventPayload } from '../types';

/** List all blocks that carry a `due` property, ordered by due ascending. */
export async function listReminders(): Promise<ReminderDto[]> {
  return invoke('list_reminders');
}

/** Push a reminder's due time forward by `minutes` (10 or 60). */
export async function snoozeReminder(
  pagePath: string,
  blockId: string,
  minutes: number,
): Promise<void> {
  return invoke('snooze_reminder', { pagePath, blockId, minutes });
}

/** Remove the reminder from a block (clears its `due` property). */
export async function clearReminder(pagePath: string, blockId: string): Promise<void> {
  return invoke('clear_reminder', { pagePath, blockId });
}

/** Subscribe to in-app reminder toast events (fired when a reminder becomes due). */
export function onReminderDue(cb: (payload: ReminderEventPayload) => void): Promise<UnlistenFn> {
  return listen<ReminderEventPayload>('reminder:due', (e) => cb(e.payload));
}

/** Subscribe to "reminder opened" events (fired when the user taps a notification). */
export function onReminderOpen(cb: (payload: ReminderEventPayload) => void): Promise<UnlistenFn> {
  return listen<ReminderEventPayload>('reminder:open', (e) => cb(e.payload));
}
