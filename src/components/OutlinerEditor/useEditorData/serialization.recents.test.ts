import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useRecentsStore } from '../../../stores/recentsStore';
import { refreshRecentsAfterSave } from './serialization';

describe('refreshRecentsAfterSave — autosave/edit wiring into the recents store', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    useRecentsStore.setState({
      recents: [],
      lastUpdated: null,
      refreshing: false,
      error: null,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('routes a completed save into the store refresh', () => {
    const refreshSpy = vi
      .spyOn(useRecentsStore.getState(), 'refresh')
      .mockImplementation(() => {});

    refreshRecentsAfterSave();

    expect(refreshSpy).toHaveBeenCalledTimes(1);
  });

  it('collapses a burst of saves into a single store refresh (idempotent)', () => {
    const refreshSpy = vi
      .spyOn(useRecentsStore.getState(), 'refresh')
      .mockImplementation(() => {});

    // Simulate an autosave burst: every successful write calls the helper.
    refreshRecentsAfterSave();
    refreshRecentsAfterSave();
    refreshRecentsAfterSave();

    // The helper hands off to the store's debounced refresh entry point; the
    // store's own debounce (covered in recentsStore.test.ts) coalesces these
    // into at most one reload.
    expect(refreshSpy).toHaveBeenCalledTimes(3);
  });

  it('needs no page argument — the reload is canonical, not per-page', () => {
    const refreshSpy = vi
      .spyOn(useRecentsStore.getState(), 'refresh')
      .mockImplementation(() => {});

    refreshRecentsAfterSave();

    // The store reloads the whole canonical list; the helper does not need to
    // know which page saved (journal or not — the store's deriveRecents filter
    // handles namespace exclusion).
    expect(refreshSpy).toHaveBeenCalledTimes(1);
  });
});
