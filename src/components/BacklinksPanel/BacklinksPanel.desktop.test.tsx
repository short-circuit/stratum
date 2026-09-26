import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, cleanup, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import BacklinksPanelDesktop from './BacklinksPanel.desktop';
import * as api from '../../lib/commands';
import type { BacklinkItem } from '../../lib/types';

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

async function renderPanel() {
  const utils = render(
    <MemoryRouter>
      <BacklinksPanelDesktop pagePath="pages/target.md" />
    </MemoryRouter>,
  );
  // Expand the accordion so the backlink rows are rendered.
  fireEvent.click(screen.getByRole('button', { name: /backlinks/i }));
  await waitFor(() => {
    expect(screen.getByText('pages/source.md')).toBeInTheDocument();
  });
  return utils;
}

function pressCtrl() {
  fireEvent.keyDown(window, { key: 'Control' });
}
function releaseCtrl() {
  fireEvent.keyUp(window, { key: 'Control' });
}

beforeEach(() => {
  // resetAllMocks (not clearAllMocks): also clears the mockResolvedValueOnce /
  // mockRejectedValueOnce queues so per-test backend stubs never leak across
  // tests in this file.
  vi.resetAllMocks();
  vi.mocked(api.getPageBacklinks).mockResolvedValue([backlink]);
});

afterEach(() => {
  cleanup();
});

describe('BacklinksPanelDesktop — ctrl+hover snippet popup', () => {
  it('fetches the snippet and shows the popup with highlighted anchor on ctrl+hover', async () => {
    vi.mocked(api.getBacklinkSnippet).mockResolvedValueOnce(snippet);
    await renderPanel();

    pressCtrl();
    const row = screen.getByText('pages/source.md').closest('.MuiListItemButton-root')!;
    fireEvent.mouseEnter(row);

    await waitFor(() => {
      expect(api.getBacklinkSnippet).toHaveBeenCalledWith(
        'pages/source.md',
        'anchor-block-uuid',
      );
    });

    // Popup shows the note title and the backlinked anchor highlighted.
    expect(screen.getByText('Source Note')).toBeInTheDocument();
    // The anchor block appears both as the row context and inside the popover;
    // scope to the popover paper only.
    const paper = screen.getByText('Source Note').closest('.MuiPopover-paper') as HTMLElement;
    const anchorEl = within(paper).getByText('See [[Target]] note');
    expect(anchorEl).toHaveStyle({ backgroundColor: 'action.selected' });

    releaseCtrl();
  });

  it('does NOT open the popup on hover without Ctrl', async () => {
    vi.mocked(api.getBacklinkSnippet).mockResolvedValueOnce(snippet);
    await renderPanel();

    const row = screen.getByText('pages/source.md').closest('.MuiListItemButton-root')!;
    fireEvent.mouseEnter(row);

    await new Promise((r) => setTimeout(r, 300));
    expect(api.getBacklinkSnippet).not.toHaveBeenCalled();
    expect(screen.queryByText('Source Note')).not.toBeInTheDocument();
  });

  it('shows an error state when the snippet 404s (missing note/anchor)', async () => {
    vi.mocked(api.getBacklinkSnippet).mockRejectedValueOnce(
      new Error('Block not found'),
    );
    await renderPanel();

    pressCtrl();
    const row = screen.getByText('pages/source.md').closest('.MuiListItemButton-root')!;
    fireEvent.mouseEnter(row);

    await waitFor(() => {
      expect(screen.getByText(/could not load/i)).toBeInTheDocument();
    });
    expect(document.querySelector('.MuiCircularProgress-root')).toBeNull();

    releaseCtrl();
  });

  it('dismisses the popup when Ctrl is released', async () => {
    vi.mocked(api.getBacklinkSnippet).mockResolvedValueOnce(snippet);
    await renderPanel();

    pressCtrl();
    const row = screen.getByText('pages/source.md').closest('.MuiListItemButton-root')!;
    fireEvent.mouseEnter(row);
    await waitFor(() => {
      expect(screen.getByText('Source Note')).toBeInTheDocument();
    });

    releaseCtrl();
    await waitFor(() => {
      expect(screen.queryByText('Source Note')).not.toBeInTheDocument();
    });
  });

  it('plain click does not open the popup (navigation path unchanged)', async () => {
    await renderPanel();

    const row = screen.getByText('pages/source.md').closest('.MuiListItemButton-root')!;
    fireEvent.click(row);

    expect(api.getBacklinkSnippet).not.toHaveBeenCalled();
    expect(screen.queryByText('Source Note')).not.toBeInTheDocument();
  });
});
