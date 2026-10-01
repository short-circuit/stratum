import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as api from '../../lib/commands';
import { useSavedQueries, formatUpdatedAt } from './QueryPanel.shared';
import type { SavedQuery } from '../../lib/types';

vi.mock('../../lib/commands', () => ({
  listSavedQueries: vi.fn(),
  saveSavedQuery: vi.fn(),
  renameSavedQuery: vi.fn(),
  deleteSavedQuery: vi.fn(),
  runQuery: vi.fn(),
}));

const QUERIES: SavedQuery[] = [
  { name: 'All TODO', query: '{:query [:find ?b :where [?b :block/marker "TODO"]]}', updated_at: '2026-09-22T20:00:00Z' },
  { name: 'Blocked', query: '{:query [:find ?b :where [?b :block/marker "WAITING"]]}', updated_at: '2026-09-22T21:00:00Z' },
];

describe('useSavedQueries', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (api.listSavedQueries as ReturnType<typeof vi.fn>).mockResolvedValue(QUERIES);
    (api.saveSavedQuery as ReturnType<typeof vi.fn>).mockResolvedValue(QUERIES[0]);
    (api.renameSavedQuery as ReturnType<typeof vi.fn>).mockResolvedValue({ ...QUERIES[0], name: 'Renamed' });
    (api.deleteSavedQuery as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('loads the saved query list on mount', async () => {
    const { result } = renderHook(() => useSavedQueries('{:query []}'));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.saved).toEqual(QUERIES);
    expect(api.listSavedQueries).toHaveBeenCalledTimes(1);
  });

  it('saveCurrent persists the current editor query under the given name and refreshes', async () => {
    const { result } = renderHook(() => useSavedQueries('{:query [:find ?b]}'));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    (api.listSavedQueries as ReturnType<typeof vi.fn>).mockResolvedValueOnce([...QUERIES]);
    await act(async () => {
      await result.current.saveCurrent('New saved query');
    });

    expect(api.saveSavedQuery).toHaveBeenCalledWith('New saved query', '{:query [:find ?b]}');
    // Refresh happened after the save so the list reflects the new entry.
    expect(result.current.saved.length).toBeGreaterThanOrEqual(2);
  });

  it('deleteQuery removes the entry and refreshes', async () => {
    const { result } = renderHook(() => useSavedQueries('{:query []}'));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    (api.listSavedQueries as ReturnType<typeof vi.fn>).mockResolvedValueOnce([QUERIES[1]]);
    await act(async () => {
      await result.current.deleteQuery('All TODO');
    });

    expect(api.deleteSavedQuery).toHaveBeenCalledWith('All TODO');
    expect(result.current.saved).toEqual([QUERIES[1]]);
  });

  it('renameQuery renames the entry and refreshes', async () => {
    const { result } = renderHook(() => useSavedQueries('{:query []}'));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const renamed = [{ ...QUERIES[0], name: 'Renamed' }, QUERIES[1]];
    (api.listSavedQueries as ReturnType<typeof vi.fn>).mockResolvedValueOnce(renamed);
    await act(async () => {
      await result.current.renameQuery('All TODO', 'Renamed');
    });

    expect(api.renameSavedQuery).toHaveBeenCalledWith('All TODO', 'Renamed');
    expect(result.current.saved[0]?.name).toBe('Renamed');
  });

  it('surfaces list load failures through loadError', async () => {
    (api.listSavedQueries as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('disk error'));
    const { result } = renderHook(() => useSavedQueries('{:query []}'));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });
    expect(result.current.loadError).toBe('Error: disk error');
    expect(result.current.saved).toEqual([]);
  });

  it('surfaces action failures through actionError without refreshing', async () => {
    const { result } = renderHook(() => useSavedQueries('{:query []}'));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    (api.saveSavedQuery as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error('duplicate name'));
    await act(async () => {
      await result.current.saveCurrent('Duplicate');
    });

    expect(result.current.actionError).toBe('Error: duplicate name');
    // The list must not be re-fetched after a failed save.
    expect(api.listSavedQueries).toHaveBeenCalledTimes(1);
  });

  it('clears the action error', async () => {
    const { result } = renderHook(() => useSavedQueries('{:query []}'));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    (api.saveSavedQuery as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error('boom'));
    await act(async () => {
      await result.current.saveCurrent('X');
    });
    expect(result.current.actionError).toBe('Error: boom');

    act(() => {
      result.current.clearActionError();
    });
    expect(result.current.actionError).toBeNull();
  });
});

describe('formatUpdatedAt', () => {
  it('returns an empty string for empty input', () => {
    expect(formatUpdatedAt('')).toBe('');
  });

  it('passes through unparseable dates', () => {
    expect(formatUpdatedAt('not-a-date')).toBe('not-a-date');
  });

  it('formats a valid RFC 3339 timestamp', () => {
    const out = formatUpdatedAt('2026-09-22T20:00:00Z');
    expect(out).not.toBe('');
    expect(out).not.toContain('NaN');
  });
});
