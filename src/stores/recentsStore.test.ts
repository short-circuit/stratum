import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as api from '../lib/commands';
import {
  useRecentsStore,
  deriveRecents,
  sameRecents,
  isRecentPage,
  RECENTS_DEBOUNCE_MS,
} from './recentsStore';
import type { PageDto } from '../lib/types';

// Mock the api module so refresh() does not round-trip through Tauri IPC.
vi.mock('../lib/commands', () => ({
  listPages: vi.fn(),
}));

const mkPage = (path: string, modified_at: string): PageDto => ({
  path,
  slug: path.replace(/\.md$/, ''),
  title: null,
  block_count: 0,
  modified_at,
});

const A = mkPage('pages/a.md', '2026-09-22T10:00:00Z');
const B = mkPage('pages/b.md', '2026-09-22T11:00:00Z');
const J = mkPage('journals/2026-09-22.md', '2026-09-22T12:00:00Z');

describe('recentsStore — deriveRecents', () => {
  it('preserves canonical ordering and drops journal pages', () => {
    const derived = deriveRecents([A, J, B]);
    expect(derived.map(p => p.path)).toEqual(['pages/b.md', 'pages/a.md']);
  });

  it('filters out journal pages regardless of input order', () => {
    const derived = deriveRecents([J, A]);
    expect(derived.map(p => p.path)).toEqual(['pages/a.md']);
  });

  it('returns an empty list for no / journal-only input', () => {
    expect(deriveRecents([])).toEqual([]);
    expect(deriveRecents([J])).toEqual([]);
  });

  it('is stable on already-sorted canonical input', () => {
    const derived = deriveRecents([B, A]);
    expect(derived.map(p => p.path)).toEqual(['pages/b.md', 'pages/a.md']);
  });
});

describe('isRecentPage', () => {
  it('treats journal entries as non-recent', () => {
    expect(isRecentPage(J)).toBe(false);
    expect(isRecentPage(A)).toBe(true);
  });
});

describe('sameRecents', () => {
  it('is reflexive on identical lists', () => {
    expect(sameRecents([A, B], [A, B])).toBe(true);
  });

  it('is false when order changes', () => {
    expect(sameRecents([A, B], [B, A])).toBe(false);
  });

  it('is false when a page is added/removed or content changes', () => {
    expect(sameRecents([A], [A, B])).toBe(false);
    expect(sameRecents([A as PageDto], [B])).toBe(false);
    expect(sameRecents([A], [mkPage('pages/a.md', '2026-09-22T09:00:00Z')])).toBe(false);
  });
});

describe('useRecentsStore — refresh()', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    useRecentsStore.setState({ recents: [], lastUpdated: null, refreshing: false, error: null });
  });

  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  it('exposes refresh() and reloads from the canonical source', async () => {
    (api.listPages as ReturnType<typeof vi.fn>).mockResolvedValue({ pages: [A, B, J] });

    useRecentsStore.getState().refresh();

    // Debounce window: nothing happens until it elapses.
    expect(api.listPages).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(RECENTS_DEBOUNCE_MS + 10);

    expect(api.listPages).toHaveBeenCalledTimes(1);
    const state = useRecentsStore.getState();
    // Journal filtered out, order preserved.
    expect(state.recents.map(p => p.path)).toEqual(['pages/b.md', 'pages/a.md']);
    expect(state.error).toBeNull();
  });

  it('is idempotent — same data does not re-publish the list', async () => {
    (api.listPages as ReturnType<typeof vi.fn>).mockResolvedValue({ pages: [A, B] });

    useRecentsStore.setState({
      recents: deriveRecents([A, B]),
      lastUpdated: 123,
      refreshing: false,
      error: null,
    });
    const previousRecents = useRecentsStore.getState().recents;
    useRecentsStore.getState().refresh();
    await vi.advanceTimersByTimeAsync(RECENTS_DEBOUNCE_MS + 10);

    // listPages ran, but the recents array must NOT be re-published: a fresh
    // derived array would signal "data changed" to consumers and could disturb
    // scroll/selection. Assert the exact same reference is kept.
    expect(api.listPages).toHaveBeenCalledTimes(1);
    const after = useRecentsStore.getState();
    expect(after.recents).toBe(previousRecents);
    expect(after.lastUpdated).toBe(123);
  });

  it('debounces rapid successive triggers into a single reload', async () => {
    (api.listPages as ReturnType<typeof vi.fn>).mockResolvedValue({ pages: [A, B] });

    const refresh = useRecentsStore.getState().refresh;
    refresh();
    await vi.advanceTimersByTimeAsync(100);
    refresh();
    await vi.advanceTimersByTimeAsync(100);
    refresh();
    await vi.advanceTimersByTimeAsync(RECENTS_DEBOUNCE_MS + 10);

    expect(api.listPages).toHaveBeenCalledTimes(1);
  });

  it('does not reset the list on failure', async () => {
    (api.listPages as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error('boom'));

    useRecentsStore.getState().refresh();
    await vi.advanceTimersByTimeAsync(RECENTS_DEBOUNCE_MS + 10);

    const state = useRecentsStore.getState();
    expect(state.recents).toEqual([]);
    expect(state.error).toBeTruthy();
    expect(state.refreshing).toBe(false);
  });
});
