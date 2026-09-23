import { create } from 'zustand';
import type { VaultInfo, PageDto } from '../lib/types';
import * as api from '../lib/commands';

export interface ThemeConfig {
  primaryHex: string;
  secondaryHex: string;
  dark: boolean;
  fontSize: number;
}

/**
 * Structured top-of-app error notification.
 *
 * Replaces the former bare `string | null` `error` field with an object that
 * carries an id (used to invalidate stale auto-dismiss timers) and a
 * `persistent` flag. Persistent errors live in a SEPARATE slot
 * (`persistentError`) so an unrelated transient error can never clear them:
 * the two slots are written and cleared independently.
 */
export interface AppError {
  /** Monotonic identity for this notification; used to invalidate stale timers. */
  id: number;
  /** Human-readable message rendered by the top-of-app surface. */
  message: string;
  /** Persistent errors never auto-dismiss; they remain until explicitly dismissed. */
  persistent: boolean;
}

/** Options accepted by {@link AppState.showError}. */
export interface ShowErrorOptions {
  /** When true the notification goes to the `persistentError` slot (no auto-dismiss). */
  persistent?: boolean;
  /** Auto-dismiss delay in ms for transient errors (default {@link TRANSIENT_ERROR_MS}). */
  autoDismissMs?: number;
}

/** Default timeout after which a transient error auto-dismisses. */
export const TRANSIENT_ERROR_MS = 6000;

export interface AppState {
  vault: VaultInfo | null;
  pages: PageDto[];
  currentPage: PageDto | null;
  loading: boolean;
  /** Transient error slot: auto-dismisses after {@link TRANSIENT_ERROR_MS}. */
  error: AppError | null;
  /** Persistent error slot: never auto-dismisses; cleared only by explicit dismissal. */
  persistentError: AppError | null;
  themeConfig: ThemeConfig;

  loadVault: () => Promise<void>;
  loadPages: () => Promise<void>;
  openPage: (path: string) => Promise<void>;
  createPage: (path: string, title?: string) => Promise<void>;
  deletePage: (path: string) => Promise<void>;
  pickVaultDirectory: () => Promise<void>;
  setThemeConfig: (config: ThemeConfig) => void;

  /**
   * Surface an error through the app-wide error slots.
   *
   * Transient errors (default) go to the `error` slot and schedule an
   * auto-dismiss timer keyed by the notification id so a stale timer can never
   * clear a newer notification that replaced it. `{ persistent: true }` errors
   * go to the `persistentError` slot with no timer; they remain visible until
   * explicitly dismissed via {@link dismissError}. A new persistent error
   * replaces a previous one.
   */
  showError: (message: string, opts?: ShowErrorOptions) => void;

  /**
   * Dismiss an error (explicit user action). Accepts an optional id: when
   * provided, a stale dismissal request for a notification that was already
   * replaced is a no-op. Clears any pending auto-dismiss timer for the removed
   * entry. Clearing the store's error state means the error cannot reappear
   * until a new `showError` call.
   */
  dismissError: (id?: number) => void;

  /**
   * Clear a transient error without disturbing a persistent one. Used on
   * successful retry / unrelated state updates so a persistent error stays
   * visible while stale transient errors are removed.
   */
  clearTransientError: () => void;
}

// Module-scoped notification plumbing. The store is a module singleton, so
// timer bookkeeping lives at module scope; every timer is keyed by the
// notification id so a stale timer can never clear a newer notification.
let errorSeq = 0;
const errorTimers = new Map<number, ReturnType<typeof setTimeout>>();

function clearErrorTimer(id: number): void {
  const timer = errorTimers.get(id);
  if (timer !== undefined) {
    clearTimeout(timer);
    errorTimers.delete(id);
  }
}

export const useStore = create<AppState>((set, get) => ({
  vault: null,
  pages: [],
  currentPage: null,
  loading: false,
  error: null,
  persistentError: null,
  themeConfig: { primaryHex: '#f97316', secondaryHex: '#6b7280', dark: true, fontSize: 16 },

  loadVault: async () => {
    try {
      set({ loading: true });
      get().clearTransientError();
      const vault = await api.getVaultInfo();
      set({ vault });
      get().clearTransientError();
    } catch (e) {
      get().showError(String(e));
    } finally {
      set({ loading: false });
    }
  },

  loadPages: async () => {
    try {
      set({ loading: true });
      const { pages } = await api.listPages();
      set({ pages });
      // A successful retry clears any transient error left by an earlier failure.
      get().clearTransientError();
    } catch (e) {
      get().showError(String(e));
    } finally {
      set({ loading: false });
    }
  },

  openPage: async (path: string) => {
    try {
      set({ loading: true });
      const page = await api.openPage(path);
      set({ currentPage: page });
      // Successful navigation clears a stale transient error.
      get().clearTransientError();
    } catch (e) {
      get().showError(String(e));
    } finally {
      set({ loading: false });
    }
  },

  createPage: async (path: string, title?: string) => {
    try {
      await api.createPage(path, title);
      await get().loadPages();
      get().clearTransientError();
    } catch (e) {
      get().showError(String(e));
    }
  },

  deletePage: async (path: string) => {
    try {
      await api.deletePage(path);
      set({ currentPage: null });
      await get().loadPages();
      get().clearTransientError();
    } catch (e) {
      get().showError(String(e));
    }
  },

  pickVaultDirectory: async () => {
    try {
      set({ loading: true });
      get().clearTransientError();
      const vault = await api.pickVaultDirectory();
      set({ vault });
      await get().loadPages();
      get().clearTransientError();
    } catch (e) {
      get().showError(String(e));
    } finally {
      set({ loading: false });
    }
  },

  setThemeConfig: (config: ThemeConfig) => {
    set({ themeConfig: config });
  },

  showError: (message, opts) => {
    const id = ++errorSeq;
    const persistent = opts?.persistent ?? false;
    const autoDismissMs = opts?.autoDismissMs ?? TRANSIENT_ERROR_MS;

    if (persistent) {
      // Persistent errors live in their own slot; replacing one cancels any
      // stale timer it may have had and never touches the transient slot.
      const prev = get().persistentError;
      if (prev) clearErrorTimer(prev.id);
      set({ persistentError: { id, message, persistent: true } });
      return;
    }

    // Replacing an existing transient notification must cancel its pending timer.
    const prev = get().error;
    if (prev) clearErrorTimer(prev.id);

    set({ error: { id, message, persistent: false } });

    const timer = setTimeout(() => {
      // Only clear if this exact notification is still the current one; a newer
      // notification (different id) that replaced it must not be cleared.
      if (get().error?.id === id) {
        set({ error: null });
      }
      errorTimers.delete(id);
    }, autoDismissMs);
    errorTimers.set(id, timer);
  },

  dismissError: (id) => {
    const transient = get().error;
    if (transient && (id === undefined || transient.id === id)) {
      clearErrorTimer(transient.id);
      set({ error: null });
      return;
    }
    const persistent = get().persistentError;
    if (persistent && (id === undefined || persistent.id === id)) {
      clearErrorTimer(persistent.id);
      set({ persistentError: null });
    }
  },

  clearTransientError: () => {
    const transient = get().error;
    if (transient) {
      clearErrorTimer(transient.id);
      set({ error: null });
    }
  },
}));
