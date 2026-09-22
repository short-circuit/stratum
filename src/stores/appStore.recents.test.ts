import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as api from '../lib/commands';
import { useStore } from './appStore';
import { useRecentsStore } from './recentsStore';
import type { PageDto } from '../lib/types';

// Mock the Tauri command surface so the store actions can run in jsdom without
// a runtime. The recentsStore consumes the same barrel; a single mock covers
// both consumers.
vi.mock('../lib/commands', () => ({
  listPages: vi.fn(),
  openPage: vi.fn(),
  createPage: vi.fn(),
  deletePage: vi.fn(),
}));

const PAGE: PageDto = {
  path: 'pages/a.md',
  slug: 'a',
  title: null,
  block_count: 0,
  modified_at: '2026-09-22T10:00:00Z',
};

describe('appStore — recents refresh wiring (page ops)', () => {
  let refreshSpy: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.clearAllMocks();
    // Replace the store's refresh with a no-op spy so the wiring is asserted
    // directly without depending on the debounce timer.
    refreshSpy = vi
      .spyOn(useRecentsStore.getState(), 'refresh')
      .mockImplementation(() => {});

    useStore.setState({
      vault: { path: '/vault', block_count: 0, page_count: 0 },
      pages: [PAGE],
      currentPage: null,
      loading: false,
      error: null,
      themeConfig: { primaryHex: '#f97316', secondaryHex: '#6b7280', dark: true, fontSize: 16 },
    });

    (api.listPages as ReturnType<typeof vi.fn>).mockResolvedValue({ pages: [PAGE] });
  });

  afterEach(() => {
    refreshSpy.mockRestore();
  });

  it('createPage triggers a recents refresh after the page is created', async () => {
    (api.createPage as ReturnType<typeof vi.fn>).mockResolvedValue(PAGE);

    await useStore.getState().createPage('pages/a.md');

    expect(api.createPage).toHaveBeenCalledWith('pages/a.md', undefined);
    expect(refreshSpy).toHaveBeenCalledTimes(1);
  });

  it('deletePage triggers a recents refresh after the page is deleted', async () => {
    (api.deletePage as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);

    await useStore.getState().deletePage('pages/a.md');

    expect(api.deletePage).toHaveBeenCalledWith('pages/a.md');
    expect(refreshSpy).toHaveBeenCalledTimes(1);
  });

  it('openPage triggers a recents refresh after the page is opened', async () => {
    (api.openPage as ReturnType<typeof vi.fn>).mockResolvedValue(PAGE);

    await useStore.getState().openPage('pages/a.md');

    expect(api.openPage).toHaveBeenCalledWith('pages/a.md');
    expect(refreshSpy).toHaveBeenCalledTimes(1);
  });

  it('does not refresh when createPage fails (no stale-list churn on error)', async () => {
    (api.createPage as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('boom'));

    await useStore.getState().createPage('pages/a.md');

    expect(refreshSpy).not.toHaveBeenCalled();
  });
});
