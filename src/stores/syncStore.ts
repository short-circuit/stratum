import { create } from 'zustand';
import type { SyncStatusDto, CommitLogEntry } from '../lib/types';
import * as api from '../lib/commands';

export interface SyncState {
  syncStatus: SyncStatusDto | null;
  commits: CommitLogEntry[];
  syncing: boolean;
  lastSyncTime: string | null;

  fetchSyncStatus: () => Promise<void>;
  fetchCommitLog: () => Promise<void>;
  doSync: () => Promise<SyncStatusDto>;
  doSyncWithPassphrase: (passphrase: string) => Promise<SyncStatusDto>;
  setSyncing: (syncing: boolean) => void;
  setCommits: (commits: CommitLogEntry[]) => void;
}

export const useSyncStore = create<SyncState>((set) => ({
  syncStatus: null,
  commits: [],
  syncing: false,
  lastSyncTime: null,

  fetchSyncStatus: async () => {
    try {
      const status = await api.getSyncStatus();
      set({ syncStatus: status, lastSyncTime: status.last_sync_time });
    } catch {
      // silently fail — e.g. no vault yet
    }
  },

  fetchCommitLog: async () => {
    try {
      const commits = await api.getCommitLog();
      set({ commits });
    } catch {
      // silently fail
    }
  },

  doSync: async () => {
    set({ syncing: true });
    try {
      const result = await api.syncVault();
      set({ syncStatus: result, lastSyncTime: result.last_sync_time });
      return result;
    } finally {
      set({ syncing: false });
    }
  },

  doSyncWithPassphrase: async (passphrase: string) => {
    set({ syncing: true });
    try {
      const result = await api.syncVaultWithPassphrase(passphrase);
      set({ syncStatus: result, lastSyncTime: result.last_sync_time });
      return result;
    } finally {
      set({ syncing: false });
    }
  },

  setSyncing: (syncing: boolean) => set({ syncing }),

  setCommits: (commits: CommitLogEntry[]) => set({ commits }),
}));
