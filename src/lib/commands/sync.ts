import { invoke } from '@tauri-apps/api/core';
import type { SyncStatusDto, CommitLogEntry } from '../types';

export async function getSyncStatus(): Promise<SyncStatusDto> {
  return invoke('get_sync_status');
}

export async function syncVault(): Promise<SyncStatusDto> {
  return invoke('sync_vault');
}

export async function syncVaultWithPassphrase(passphrase: string): Promise<SyncStatusDto> {
  return invoke('sync_vault_with_passphrase', { passphrase });
}

export async function startSyncScheduler(): Promise<void> {
  return invoke('start_sync_scheduler');
}

export async function stopSyncScheduler(): Promise<void> {
  return invoke('stop_sync_scheduler');
}

export async function getCommitLog(): Promise<CommitLogEntry[]> {
  return invoke('get_commit_log');
}

export async function resolveConflictFile(path: string): Promise<void> {
  return invoke('resolve_conflict_file', { path });
}

export async function abortMerge(): Promise<void> {
  return invoke('abort_merge');
}
