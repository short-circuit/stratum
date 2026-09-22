// Recents store — the single source of truth for the sidebar "Recent" list.
//
// The canonical source of recent items is the page list the backend derives
// from the block store (`api.listPages`), whose ordering is `modified_at DESC`.
// This store consolidates the ordering/limit rules that previously lived ad hoc
// in the sidebar (`PageTree`): exclude the journal namespace, keep the canonical
// order. It exposes a single `refresh()` entry point that is:
//
//   - debounced (trailing 500ms) so bursts of events (autosave, page ops,
//     watcher notifications) collapse into at most one reload, and
//   - idempotent: it only publishes a new list when the derived recents have
//     actually changed, so unchanged data never triggers a re-render that would
//     disturb user scroll/selection.
//
// Consumers (event sources) call `useRecentsStore.getState().refresh()`; they do
// not need to manage ordering, limits, or dedup themselves.

import { create } from 'zustand';
import type { PageDto } from '../lib/types';
import * as api from '../lib/commands';

/** Trailing debounce window for `refresh()`. Rapid successive triggers within
 *  this window coalesce into a single reload. */
export const RECENTS_DEBOUNCE_MS = 500;

/** Namespace excluded from the recents list (unchanged from the previous
 *  sidebar behavior — journals are not surfaced under "Recent"). */
export const JOURNAL_NAMESPACE = 'journals/';

/** A page is a "recent" candidate iff it is not part of the journal namespace. */
export function isRecentPage(page: PageDto): boolean {
  return !page.path.startsWith(JOURNAL_NAMESPACE);
}

/** Derive the recents list from the canonical page list: drop journal pages,
 *  preserve the canonical `modified_at DESC` ordering. The input array is not
 *  mutated (a copy is sorted). */
export function deriveRecents(pages: PageDto[]): PageDto[] {
  return pages
    .filter(isRecentPage)
    .slice()
    .sort((a, b) => b.modified_at.localeCompare(a.modified_at));
}

/** Cheap identity test for the derived list, used to make the refresh write
 *  idempotent: two lists compare equal when they have the same pages in the
 *  same order with identical modification timestamps. */
export function sameRecents(a: PageDto[], b: PageDto[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    const x = a[i];
    const y = b[i];
    if (!x || !y) return false;
    if (x.path !== y.path || x.modified_at !== y.modified_at) return false;
  }
  return true;
}

export interface RecentsState {
  /** Recent pages (non-journal, `modified_at` DESC) derived from the canonical
   *  source. Empty until the first successful refresh. */
  recents: PageDto[];
  /** Timestamp of the last successful refresh, or null before the first one. */
  lastUpdated: number | null;
  /** True while a reload is in flight. */
  refreshing: boolean;
  /** Last refresh error message, if any. A failed refresh keeps the previous list. */
  error: string | null;
  /** Schedule (debounced, trailing) a reload of recent items from the canonical
   *  source. Repeated calls within `RECENTS_DEBOUNCE_MS` coalesce into one
   *  reload. Idempotent: state is only updated when the derived list changed. */
  refresh: () => void;
}

export const useRecentsStore = create<RecentsState>()((set, get) => {
  let timer: ReturnType<typeof setTimeout> | undefined;

  const doRefresh = async (): Promise<void> => {
    const prev = get().recents;
    set({ refreshing: true, error: null });
    try {
      const { pages } = await api.listPages();
      const next = deriveRecents(pages);
      // Idempotent write: only publish when the derived list actually changed,
      // so unchanged data keeps user scroll/selection and avoids needless
      // re-renders in wired-up consumers.
      if (!sameRecents(prev, next)) {
        set({ recents: next, lastUpdated: Date.now() });
      }
    } catch (e) {
      // Keep the previous list on failure; surface the error for diagnostics.
      set({ error: String(e), lastUpdated: Date.now() });
    } finally {
      set({ refreshing: false });
    }
  };

  return {
    recents: [],
    lastUpdated: null,
    refreshing: false,
    error: null,
    refresh: () => {
      if (timer) {
        clearTimeout(timer);
      }
      timer = setTimeout(() => {
        timer = undefined;
        void doRefresh();
      }, RECENTS_DEBOUNCE_MS);
    },
  };
});
