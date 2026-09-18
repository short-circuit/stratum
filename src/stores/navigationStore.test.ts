import { beforeEach, describe, expect, it } from 'vitest';
import { useNavigationStore } from './navigationStore';

describe('useNavigationStore (backlink navigation state)', () => {
  beforeEach(() => {
    // Reset store state between tests.
    useNavigationStore.getState().consume('__reset');
  });

  it('records and consumes a modifier-click navigation record', () => {
    const push = useNavigationStore.getState().push;
    push('/notes/source.md', { scrollTop: 420 });

    const consume = useNavigationStore.getState().consume;
    expect(consume('/notes/source.md')).toEqual({ scrollTop: 420 });
  });

  it('consumes are one-shot — the record is cleared after delivery', () => {
    useNavigationStore.getState().push('/notes/a.md', { scrollTop: 10 });
    const consume = useNavigationStore.getState().consume;

    expect(consume('/notes/a.md')).toEqual({ scrollTop: 10 });
    // Second consume for the same page yields nothing.
    expect(consume('/notes/a.md')).toBeNull();
  });

  it('returns null for a page that was never modifier-clicked from', () => {
    useNavigationStore.getState().push('/notes/from.md', { scrollTop: 5 });
    expect(useNavigationStore.getState().consume('/notes/other.md')).toBeNull();
  });

  it('a fresh modifier-click supersedes a pending record', () => {
    useNavigationStore.getState().push('/notes/first.md', { scrollTop: 1 });
    useNavigationStore.getState().push('/notes/second.md', { scrollTop: 2 });

    const consume = useNavigationStore.getState().consume;
    // Only the most recent record survives.
    expect(consume('/notes/first.md')).toBeNull();
    expect(consume('/notes/second.md')).toEqual({ scrollTop: 2 });
  });
});
