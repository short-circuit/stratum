import { render, screen } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { MemoryRouter } from 'react-router-dom';

// The vite define for the app version is not provided by the vitest runtime;
// stub it so the sidebar footer renders.
vi.stubGlobal('__APP_VERSION__', 'test');

// Mock the recents store so the test controls the list the sidebar renders.
// We assert the sidebar consumes useRecentsStore (the canonical, debounced
// source), NOT appStore.pages.
const mockedRecents = vi.hoisted(() => ({
  recents: [] as Array<{ path: string; slug: string; title: string | null; block_count: number; modified_at: string }>,
}));

vi.mock('../../stores/recentsStore', () => ({
  useRecentsStore: (selector: (s: { recents: typeof mockedRecents.recents }) => unknown) =>
    selector({ recents: mockedRecents.recents }),
}));

// appStore provides nav + page ops; the recents list comes from the mocked store.
vi.mock('../../stores/appStore', () => ({
  useStore: (selector: (s: Record<string, unknown>) => unknown) =>
    selector({
      pages: [{ path: 'pages/stale.md', slug: 'stale', title: 'Stale', block_count: 0, modified_at: '2020-01-01T00:00:00Z' }],
      vault: { path: '/vault', block_count: 0, page_count: 1 },
      loadPages: () => Promise.resolve(),
      createPage: () => Promise.resolve(),
      deletePage: () => Promise.resolve(),
    }),
}));

// Light stubs for Tauri runtime + commands used by the sidebar.
vi.mock('../../lib/commands', () => ({
  exportHtml: vi.fn().mockResolvedValue({ pages_exported: 0 }),
}));

vi.mock('react-router-dom', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return {
    ...actual,
    useNavigate: () => vi.fn(),
  };
});

// The real Sidebar renders desktop-only via useResponsive; jsdom UA is a
// desktop browser so it renders, but avoid pulling the real platform module.
vi.mock('../../lib/hooks/useResponsive', () => ({
  useResponsive: () => ({ isMobile: false, isDesktop: true, width: 1200 }),
}));

import Sidebar from './index';

describe('Sidebar — recents store consumer wiring', () => {
  beforeEach(() => {
    mockedRecents.recents = [];
  });

  it('renders the recents-store list (auto-refresh reaches the UI)', () => {
    mockedRecents.recents = [
      { path: 'pages/fresh.md', slug: 'fresh', title: 'Fresh', block_count: 1, modified_at: '2026-09-22T10:00:00Z' },
    ];
    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>,
    );
    // The fresh store-backed page is shown.
    expect(screen.getByText('Fresh')).toBeInTheDocument();
    // The stale appStore-only page is NOT shown — sidebars consumes the store.
    expect(screen.queryByText('Stale')).not.toBeInTheDocument();
  });

  it('does not render stale appStore pages when the store list is empty', () => {
    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>,
    );
    expect(screen.getByText('Recent')).toBeInTheDocument();
    expect(screen.queryByText('Stale')).not.toBeInTheDocument();
    expect(screen.getByText('No pages yet.')).toBeInTheDocument();
  });
});
