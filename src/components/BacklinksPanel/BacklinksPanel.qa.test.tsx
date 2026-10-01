import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/react';
import { useEffect } from 'react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import BacklinksPanelDesktop from './BacklinksPanel.desktop';
import * as api from '../../lib/commands';
import { useNavigationStore } from '../../stores/navigationStore';
import type { BacklinkItem } from '../../lib/types';

/**
 * QA coverage for the merged backlinks panel (task t_5a010d2b).
 *
 * The panel combines two features that landed on separate branches and were
 * integrated here:
 *   1. Ctrl+hover on a backlink shows a snippet popup (E5.F2).
 *   2. Ctrl/Cmd+click on a backlink opens the target note while preserving the
 *      source editor scroll (E5.F4); plain click behavior is unchanged.
 *
 * These tests guard the integration contract at the component level:
 *  - Ctrl+hover shows the snippet popup AND ctrl+click still navigates (both
 *    features coexist on the same rows).
 *  - Plain click navigates without opening the popup (unchanged behavior).
 *  - Missing note/anchor (404) renders an error state and does NOT break the
 *    panel — later plain-click navigation and re-hover still work.
 *  - Keyboard (Escape) dismissal of the open popup is wired end-to-end.
 */

vi.mock('../../lib/commands', () => ({
  getPageBacklinks: vi.fn(),
  getBacklinkSnippet: vi.fn(),
  openPage: vi.fn(),
}));

const backlink: BacklinkItem = {
  source_id: 'anchor-block-uuid',
  source_page: 'pages/source.md',
  context: 'See [[Target]] note',
  is_linked: true,
};

const snippet = {
  note_id: 'pages/source.md',
  note_title: 'Source Note',
  anchor_id: 'anchor-block-uuid',
  anchor_content: 'See [[Target]] note',
  context: ['Context before', 'See [[Target]] note', 'Context after'],
};

let lastPath = '';

function LocationProbe() {
  const location = useLocation();
  useEffect(() => {
    lastPath = location.pathname;
  }, [location]);
  return null;
}

async function renderPanel(initialEntry = '/page/pages/target.md') {
  const utils = render(
    <MemoryRouter initialEntries={[initialEntry]}>
      <BacklinksPanelDesktop pagePath="pages/target.md" />
      <LocationProbe />
    </MemoryRouter>,
  );
  fireEvent.click(screen.getByRole('button', { name: /backlinks/i }));
  await waitFor(() => {
    expect(screen.getByText('pages/source.md')).toBeInTheDocument();
  });
  return utils;
}

function rowEl() {
  return screen.getByText('pages/source.md').closest('.MuiListItemButton-root') as HTMLElement;
}

function pressCtrl() {
  fireEvent.keyDown(window, { key: 'Control' });
}
function releaseCtrl() {
  fireEvent.keyUp(window, { key: 'Control' });
}

beforeEach(() => {
  vi.resetAllMocks();
  useNavigationStore.setState({ from: null });
  lastPath = '';
  vi.mocked(api.getPageBacklinks).mockResolvedValue([backlink]);
});

afterEach(() => {
  cleanup();
});

describe('BacklinksPanelDesktop — merged ctrl+hover + ctrl+click integration', () => {
  it('ctrl+click navigates to the target note AND records a source scroll restore', async () => {
    // Provide a scrollable editor container so a scroll record is captured.
    const anchor = document.createElement('div');
    anchor.className = 'bn-editor';
    const scrollHost = document.createElement('div');
    Object.defineProperty(scrollHost, 'scrollTop', { value: 321, writable: true });
    Object.defineProperty(scrollHost, 'scrollHeight', { value: 2000 });
    Object.defineProperty(scrollHost, 'clientHeight', { value: 500 });
    scrollHost.appendChild(anchor);
    document.body.appendChild(scrollHost);
    const realGetComputedStyle = window.getComputedStyle.bind(window);
    vi.spyOn(window, 'getComputedStyle').mockImplementation((el) => {
      const base = realGetComputedStyle(el);
      if (el === scrollHost) {
        (base as unknown as Record<string, string>).overflowY = 'auto';
      }
      return base;
    });

    await renderPanel();
    const row = rowEl();
    fireEvent.click(row, { ctrlKey: true });

    // Navigates to the target note in the editor.
    expect(decodeURIComponent(lastPath)).toBe('/page/pages/source.md');
    // Source scroll was recorded for the one-shot restore.
    expect(useNavigationStore.getState().consume('pages/source.md')).toEqual({ scrollTop: 321 });
  });

  it('plain click navigates without opening the snippet popup (unchanged)', async () => {
    vi.mocked(api.getBacklinkSnippet).mockResolvedValueOnce(snippet);
    await renderPanel();

    const row = rowEl();
    fireEvent.click(row);

    // Plain click navigates to the target.
    expect(decodeURIComponent(lastPath)).toBe('/page/pages/source.md');
    // No snippet fetch, no popup.
    expect(api.getBacklinkSnippet).not.toHaveBeenCalled();
    expect(screen.queryByText('Source Note')).not.toBeInTheDocument();
    // No scroll restore was recorded (plain click preserves nothing).
    expect(useNavigationStore.getState().consume('pages/source.md')).toBeNull();
  });

  it('ctrl+hover shows the snippet popup (feature 1) while ctrl+click navigation still works on the same row (feature 2)', async () => {
    vi.mocked(api.getBacklinkSnippet).mockResolvedValueOnce(snippet);
    await renderPanel();

    pressCtrl();
    const row = rowEl();
    fireEvent.mouseEnter(row);
    await waitFor(() => {
      expect(screen.getByText('Source Note')).toBeInTheDocument();
    });

    // Ctrl+click while the popup is open: navigates away and dismisses the popup.
    fireEvent.click(row, { ctrlKey: true });
    expect(decodeURIComponent(lastPath)).toBe('/page/pages/source.md');
    await waitFor(() => {
      expect(screen.queryByText('Source Note')).not.toBeInTheDocument();
    });
    releaseCtrl();
  });

  it('missing note/anchor (404) shows an error state and does not break the panel', async () => {
    vi.mocked(api.getBacklinkSnippet).mockRejectedValueOnce(new Error('Block not found'));
    await renderPanel();

    pressCtrl();
    fireEvent.mouseEnter(rowEl());
    await waitFor(() => {
      expect(screen.getByText(/could not load/i)).toBeInTheDocument();
    });
    releaseCtrl();

    // Panel remains usable: a later plain click navigates normally.
    fireEvent.click(rowEl());
    expect(decodeURIComponent(lastPath)).toBe('/page/pages/source.md');
  });

  it('Escape key dismisses the open snippet popup (keyboard dismissal)', async () => {
    vi.mocked(api.getBacklinkSnippet).mockResolvedValueOnce(snippet);
    await renderPanel();

    pressCtrl();
    fireEvent.mouseEnter(rowEl());
    await waitFor(() => {
      expect(screen.getByText('Source Note')).toBeInTheDocument();
    });

    // Escape on the modal presentation root dismisses the popover.
    const modalRoot = document.querySelector('[role="presentation"]');
    fireEvent.keyDown(modalRoot as Element, { key: 'Escape' });

    await waitFor(() => {
      expect(screen.queryByText('Source Note')).not.toBeInTheDocument();
    });
    releaseCtrl();
  });
});
