import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import * as api from '../lib/commands';
import { useStore, TRANSIENT_ERROR_MS } from './appStore';

// Mock the Tauri command surface so the store actions can be exercised in
// jsdom without a runtime, mirroring useSettingsPage.test.ts.
vi.mock('../lib/commands', () => ({
  getVaultInfo: vi.fn(),
  listPages: vi.fn(),
  openPage: vi.fn(),
  createPage: vi.fn(),
  deletePage: vi.fn(),
  pickVaultDirectory: vi.fn(),
}));

const VAULT = { path: '/vault', block_count: 0, page_count: 1 };
const PAGE = { path: 'note.md', slug: 'note', title: 'Note', block_count: 0, modified_at: '2026-01-01' };

describe('appStore error state machine (dismissible / auto-dismiss / persistent)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    useStore.setState({
      vault: null,
      pages: [],
      currentPage: null,
      loading: false,
      error: null,
      persistentError: null,
      themeConfig: { primaryHex: '#f97316', secondaryHex: '#6b7280', dark: true, fontSize: 16 },
    });
    (api.getVaultInfo as ReturnType<typeof vi.fn>).mockResolvedValue(VAULT);
    (api.listPages as ReturnType<typeof vi.fn>).mockResolvedValue({ pages: [PAGE] });
    (api.openPage as ReturnType<typeof vi.fn>).mockResolvedValue(PAGE);
    (api.createPage as ReturnType<typeof vi.fn>).mockResolvedValue(PAGE);
    (api.deletePage as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);
    (api.pickVaultDirectory as ReturnType<typeof vi.fn>).mockResolvedValue(VAULT);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('showError surfaces a transient error in the transient slot with an id', () => {
    useStore.getState().showError('boom');
    const err = useStore.getState().error;
    expect(err).not.toBeNull();
    expect(err!.message).toBe('boom');
    expect(err!.persistent).toBe(false);
    expect(typeof err!.id).toBe('number');
  });

  it('transient errors auto-dismiss after the configured timeout', () => {
    useStore.getState().showError('transient');
    expect(useStore.getState().error?.message).toBe('transient');

    vi.advanceTimersByTime(TRANSIENT_ERROR_MS);
    expect(useStore.getState().error).toBeNull();
  });

  it('a newer transient error replaces the old one and only the newest auto-dismisses', () => {
    useStore.getState().showError('first');
    const firstId = useStore.getState().error!.id;
    // Half the window elapses for the first error...
    vi.advanceTimersByTime(TRANSIENT_ERROR_MS / 2);
    // ...then a newer error replaces it.
    useStore.getState().showError('second');
    const secondId = useStore.getState().error!.id;
    expect(secondId).not.toBe(firstId);

    // The stale timer for the first error must not clear the newer one.
    vi.advanceTimersByTime(TRANSIENT_ERROR_MS / 2);
    expect(useStore.getState().error?.message).toBe('second');

    // The newer error's own timer clears it.
    vi.advanceTimersByTime(TRANSIENT_ERROR_MS);
    expect(useStore.getState().error).toBeNull();
  });

  it('explicit dismiss removes the error and clears the underlying state', () => {
    useStore.getState().showError('dismiss me');
    const id = useStore.getState().error!.id;

    useStore.getState().dismissError(id);
    expect(useStore.getState().error).toBeNull();

    // State stays cleared even after the (now-invalidated) auto-dismiss window.
    vi.advanceTimersByTime(TRANSIENT_ERROR_MS);
    expect(useStore.getState().error).toBeNull();
  });

  it('a stale dismissal request for a replaced error is a no-op', () => {
    useStore.getState().showError('old');
    const oldId = useStore.getState().error!.id;
    useStore.getState().showError('new');

    useStore.getState().dismissError(oldId);
    expect(useStore.getState().error?.message).toBe('new');
  });

  it('explicit dismiss leaves unrelated store state intact', () => {
    useStore.setState({ pages: [PAGE], currentPage: PAGE, vault: VAULT });
    useStore.getState().showError('transient');

    useStore.getState().dismissError(useStore.getState().error!.id);

    expect(useStore.getState().error).toBeNull();
    expect(useStore.getState().pages).toEqual([PAGE]);
    expect(useStore.getState().currentPage?.path).toBe('note.md');
    expect(useStore.getState().vault?.path).toBe('/vault');
  });

  it('dismissError without an id dismisses whichever error is showing', () => {
    useStore.getState().showError('current');
    useStore.getState().dismissError();
    expect(useStore.getState().error).toBeNull();
  });

  it('persistent errors land in the persistent slot and never auto-dismiss', () => {
    useStore.getState().showError('sync conflict', { persistent: true });
    expect(useStore.getState().persistentError?.message).toBe('sync conflict');
    expect(useStore.getState().persistentError?.persistent).toBe(true);
    expect(useStore.getState().error).toBeNull();

    // Far beyond any transient timeout: still visible.
    vi.advanceTimersByTime(TRANSIENT_ERROR_MS * 10);
    expect(useStore.getState().persistentError).not.toBeNull();
  });

  it('persistent errors are manually dismissible', () => {
    useStore.getState().showError('keep until dismissed', { persistent: true });
    const id = useStore.getState().persistentError!.id;

    useStore.getState().dismissError(id);
    expect(useStore.getState().persistentError).toBeNull();
  });

  it('clearTransientError removes a transient error but leaves a persistent one untouched', () => {
    useStore.getState().showError('sync conflict', { persistent: true });
    useStore.getState().showError('transient noise');
    expect(useStore.getState().error).not.toBeNull();
    expect(useStore.getState().persistentError).not.toBeNull();

    useStore.getState().clearTransientError();
    expect(useStore.getState().error).toBeNull();
    expect(useStore.getState().persistentError?.message).toBe('sync conflict');
  });

  it('a successful retry clears a transient error from the same action', async () => {
    (api.openPage as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error('gone'));
    await useStore.getState().openPage('note.md');
    expect(useStore.getState().error).not.toBeNull();

    // Retry now succeeds — the stale transient error must clear.
    await useStore.getState().openPage('note.md');
    expect(useStore.getState().error).toBeNull();
    expect(useStore.getState().currentPage?.path).toBe('note.md');
  });

  it('a failing action surfaces the error', async () => {
    (api.openPage as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error('Open failed'));
    await useStore.getState().openPage('note.md');
    expect(useStore.getState().error?.message).toContain('Open failed');
  });

  it('a successful retry through loadPages clears a transient error and populates pages', async () => {
    // Regression for the pages-list failing-then-retrying flow: the stale
    // banner must clear on the successful retry, not survive until restart.
    (api.listPages as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error('list failed'));
    await useStore.getState().loadPages();
    expect(useStore.getState().error).not.toBeNull();

    // Retry now succeeds: stale transient error clears, pages fill in.
    await useStore.getState().loadPages();
    expect(useStore.getState().error).toBeNull();
    expect(useStore.getState().pages).toEqual([PAGE]);
  });

  it('a successful loadPages retry never disturbs a persistent error', async () => {
    // A persistent error (e.g. sync conflict) must survive unrelated transient
    // traffic. Here the transient error comes from a real failing retry that
    // then succeeds via the action path — not from calling the internal helper.
    useStore.getState().showError('sync conflict', { persistent: true });
    (api.listPages as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error('list failed'));
    await useStore.getState().loadPages();
    expect(useStore.getState().error).not.toBeNull();

    await useStore.getState().loadPages();
    expect(useStore.getState().error).toBeNull();
    expect(useStore.getState().persistentError?.message).toBe('sync conflict');
  });

  it('explicit dismiss of a transient error cancels its pending auto-dismiss timer', () => {
    // Once the user dismisses, the in-flight timer must not resurrect the
    // banner later (the "stays until restart" bug is the inverse: a timer that
    // never fires; this guards the timer being wrongly left armed).
    useStore.getState().showError('dismiss me');
    const id = useStore.getState().error!.id;

    useStore.getState().dismissError(id);
    expect(useStore.getState().error).toBeNull();

    // The original auto-dismiss window elapses with no resurrection.
    vi.advanceTimersByTime(TRANSIENT_ERROR_MS * 2);
    expect(useStore.getState().error).toBeNull();
  });
});
