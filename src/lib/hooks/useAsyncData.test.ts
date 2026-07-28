import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { useAsyncData } from './useAsyncData';

describe('useAsyncData', () => {
  it('starts in loading state', () => {
    const fetcher = vi.fn().mockResolvedValue('data');
    const { result } = renderHook(() => useAsyncData(fetcher));

    expect(result.current.loading).toBe(true);
    expect(result.current.data).toBeNull();
    expect(result.current.error).toBeNull();
  });

  it('returns data after successful fetch', async () => {
    const fetcher = vi.fn().mockResolvedValue('loaded');
    const { result } = renderHook(() => useAsyncData(fetcher));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.data).toBe('loaded');
    expect(result.current.error).toBeNull();
  });

  it('returns error after failed fetch', async () => {
    const fetcher = vi.fn().mockRejectedValue(new Error('Network error'));
    const { result } = renderHook(() => useAsyncData(fetcher));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.data).toBeNull();
    expect(result.current.error).toBe('Error: Network error');
  });

  it('refresh re-executes the fetcher', async () => {
    const fetcher = vi.fn()
      .mockResolvedValueOnce('first')
      .mockResolvedValueOnce('second');

    const { result } = renderHook(() => useAsyncData(fetcher));

    await waitFor(() => {
      expect(result.current.data).toBe('first');
    });

    await act(async () => {
      await result.current.refresh();
    });

    expect(fetcher).toHaveBeenCalledTimes(2);
    expect(result.current.data).toBe('second');
  });
});
