import { create } from 'zustand';

/**
 * Modifier-click navigation state.
 *
 * When the user Ctrl/Cmd+clicks a backlink, the source note's editor scroll
 * offset is captured so the target note can be opened without silently
 * discarding where the user was working in the source note. Returning to the
 * source note restores that offset.
 *
 * The app has no multi-pane/tab UI, so "not losing current note state" is
 * implemented as a one-shot restore of the source editor scroll on the next
 * open of the source note. The record is intentionally in-memory
 * (session-scoped): a stale persisted restore would be applied on unrelated
 * later opens, which is more surprising than losing the pointer across a full
 * app reload.
 */

export interface EditorRestoreState {
  /** Vertical scroll offset of the source note's editor scroll container. */
  scrollTop: number;
}

interface NavigationState {
  /** The note the user modifier-clicked away from, and how to restore it. */
  from: { pagePath: string; restore: EditorRestoreState } | null;
  /** Record that the user left `pagePath` via a modifier-click navigation. */
  push: (pagePath: string, restore: EditorRestoreState) => void;
  /**
   * Fetch the restore record for `pagePath`, if any, and clear it.
   * The record is cleared on delivery so it is applied exactly once.
   */
  consume: (pagePath: string) => EditorRestoreState | null;
}

export const useNavigationStore = create<NavigationState>()((set, get) => ({
  from: null,

  push: (pagePath, restore) => {
    // A fresh modifier-click supersedes any pending record.
    set({ from: { pagePath, restore } });
  },

  consume: (pagePath) => {
    const { from } = get();
    if (!from || from.pagePath !== pagePath) return null;
    set({ from: null });
    return from.restore;
  },
}));
