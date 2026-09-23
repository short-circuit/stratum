import { renderHook } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as api from '../../lib/commands';
import { useStore } from '../../stores/appStore';
import { useJournalPanel, formatDate, addDays, isJournalDateKey } from './JournalPanel.shared';

// Mock react-router-dom hooks used by useJournalPanel.
let mockNavigate: ReturnType<typeof vi.fn>;
vi.mock('react-router-dom', () => ({
  useNavigate: () => mockNavigate,
  useSearchParams: () => [new URLSearchParams(), vi.fn()],
}));

// Mock the api module so we can assert on ensureTodayJournal.
vi.mock('../../lib/commands', () => ({
  ensureTodayJournal: vi.fn(),
  createPage: vi.fn(),
  listPages: vi.fn(),
  getVaultInfo: vi.fn(),
}));

describe('addDays (Prev/Next day navigation helper)', () => {
  it('increments a date by one day across month boundaries', () => {
    expect(addDays('2026-02-28', 1)).toBe('2026-03-01');
    expect(addDays('2026-12-31', 1)).toBe('2027-01-01');
    expect(addDays('2026-03-01', -1)).toBe('2026-02-28');
  });

  it('preserves padding for single-digit months and days', () => {
    expect(addDays('2026-01-09', -1)).toBe('2026-01-08');
    expect(addDays('2026-10-01', -1)).toBe('2026-09-30');
  });
});

describe('isJournalDateKey', () => {
  it('accepts valid YYYY-MM-DD keys and rejects everything else', () => {
    expect(isJournalDateKey('2026-09-23')).toBe(true);
    expect(isJournalDateKey(null)).toBe(false);
    expect(isJournalDateKey('2026-9-23')).toBe(false);
    expect(isJournalDateKey('2026/09/23')).toBe(false);
    expect(isJournalDateKey('not-a-date')).toBe(false);
    expect(isJournalDateKey('2026-13-40')).toBe(true); // format-level, not calendar-validity
  });
});

describe('useJournalPanel — fresh vault (empty page list)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockNavigate = vi.fn();
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

  it('navigates to the next day from today via createNewDay(1)', async () => {
    const apiMod = await import('../../lib/commands');
    (apiMod.createPage as ReturnType<typeof vi.fn>).mockResolvedValue({});
    const { result } = renderHook(() => useJournalPanel());
    // Wait for the idle ensure flow to settle before asserting navigation.
    await vi.waitFor(() => expect(result.current.journalLoading).toBe(false));

    result.current.createNewDay(1);
    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    await vi.waitFor(() =>
      expect(mockNavigate).toHaveBeenCalledWith(`/journal?date=${formatDate(tomorrow)}`),
    );
  });

  it('navigates to the previous day from today via createNewDay(-1)', async () => {
    const apiMod = await import('../../lib/commands');
    (apiMod.createPage as ReturnType<typeof vi.fn>).mockResolvedValue({});
    const { result } = renderHook(() => useJournalPanel());
    await vi.waitFor(() => expect(result.current.journalLoading).toBe(false));

    result.current.createNewDay(-1);
    const yesterday = new Date();
    yesterday.setDate(yesterday.getDate() - 1);
    await vi.waitFor(() =>
      expect(mockNavigate).toHaveBeenCalledWith(`/journal?date=${formatDate(yesterday)}`),
    );
  });
});
