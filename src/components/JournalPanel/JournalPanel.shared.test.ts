import { renderHook } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as api from '../../lib/commands';
import { useStore } from '../../stores/appStore';
import { useJournalPanel, formatDate } from './JournalPanel.shared';

// Mock react-router-dom hooks used by useJournalPanel.
vi.mock('react-router-dom', () => ({
  useNavigate: () => vi.fn(),
  useSearchParams: () => [new URLSearchParams(), vi.fn()],
}));

// Mock the api module so we can assert on ensureTodayJournal.
vi.mock('../../lib/commands', () => ({
  ensureTodayJournal: vi.fn(),
  listPages: vi.fn(),
  getVaultInfo: vi.fn(),
}));

describe('useJournalPanel — fresh vault (empty page list)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset the zustand store to a fresh state (no pages).
    useStore.setState({
      pages: [],
      loading: false,
      error: null,
      vault: null,
      currentPage: null,
      themeConfig: { primaryHex: '#f97316', secondaryHex: '#6b7280', dark: true, fontSize: 16 },
    });
    (api.ensureTodayJournal as ReturnType<typeof vi.fn>).mockResolvedValue({
      path: `journals/${formatDate(new Date())}.md`,
      slug: formatDate(new Date()),
      title: formatDate(new Date()),
      block_count: 0,
      modified_at: Date.now(),
    });
  });

  it('calls ensureTodayJournal even when the page list is empty (fixes infinite spinner)', async () => {
    renderHook(() => useJournalPanel());

    // On mount the effect runs ensureJournal; with an empty page list the
    // regression skipped the call, leaving the spinner forever.
    expect(api.ensureTodayJournal).toHaveBeenCalled();
  });

  it('clears the loading flag once today journal is ensured', async () => {
    const apiMod = await import('../../lib/commands');
    const ensureSpy = apiMod.ensureTodayJournal as ReturnType<typeof vi.fn>;
    ensureSpy.mockResolvedValue({});

    const { result } = renderHook(() => useJournalPanel());
    // journalLoading should settle to false after resolution.
    await vi.waitFor(() => {
      expect(result.current.journalLoading).toBe(false);
    });
    expect(result.current.journalError).toBeNull();
  });
});
