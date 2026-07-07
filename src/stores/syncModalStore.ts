import { create } from 'zustand';
import * as api from '../lib/commands';

interface SyncModalState {
  conflictModalOpen: boolean;
  conflictFiles: string[];
  passphraseModalOpen: boolean;

  showConflictModal: (files: string[]) => void;
  hideConflictModal: () => void;
  showPassphraseModal: () => void;
  hidePassphraseModal: () => void;

  /** Resolve a single conflicted file via the backend, then remove it from the list. */
  resolveConflict: (file: string) => Promise<void>;
  /** Resolve all currently listed conflicted files. */
  resolveAllConflicts: () => Promise<void>;
  /** Abort the in-progress merge and close the modal. */
  abortMerge: () => Promise<void>;
  /**
   * Submit the SSH passphrase to the backend, run the sync, and chain
   * into the conflict flow if needed.
   */
  submitPassphrase: (passphrase: string) => Promise<void>;
}

export const useSyncModalStore = create<SyncModalState>((set, get) => ({
  conflictModalOpen: false,
  conflictFiles: [],
  passphraseModalOpen: false,

  showConflictModal: (files: string[]) => {
    set({ conflictModalOpen: true, conflictFiles: files });
  },

  hideConflictModal: () => {
    set({ conflictModalOpen: false, conflictFiles: [] });
  },

  showPassphraseModal: () => {
    set({ passphraseModalOpen: true });
  },

  hidePassphraseModal: () => {
    set({ passphraseModalOpen: false });
  },

  resolveConflict: async (file: string) => {
    await api.resolveConflictFile(file);
    const files = get().conflictFiles.filter((f) => f !== file);
    if (files.length === 0) {
      set({ conflictModalOpen: false, conflictFiles: [] });
    } else {
      set({ conflictFiles: files });
    }
  },

  resolveAllConflicts: async () => {
    const files = get().conflictFiles;
    for (const f of files) {
      try {
        await api.resolveConflictFile(f);
      } catch {
        /* skip individual failures */
      }
    }
    set({ conflictModalOpen: false, conflictFiles: [] });
  },

  abortMerge: async () => {
    await api.abortMerge();
    set({ conflictModalOpen: false, conflictFiles: [] });
  },

  submitPassphrase: async (passphrase: string) => {
    try {
      const result = await api.syncVaultWithPassphrase(passphrase);
      get().hidePassphraseModal();
      if (result.status === 'conflicts') {
        get().showConflictModal(result.conflicts);
      }
    } catch (e) {
      get().hidePassphraseModal();
      throw e;
    }
  },
}));
