import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import * as api from '../../lib/commands';
import { usePreview } from './BacklinksPanel.shared';
import type { BacklinkItem } from '../../lib/types';

vi.mock('../../lib/commands', () => ({
  getPageBacklinks: vi.fn(),
  getBacklinkSnippet: vi.fn(),
  openPage: vi.fn(),
}));

const mockSnippet = {
  note_id: 'pages/source.md',
  note_title: 'Source Note',
  anchor_id: 'anchor-uuid',
  anchor_content: 'The exact backlinked block',
  context: ['Before paragraph', 'The exact backlinked block', 'After paragraph'],
};

function item(over: Partial<BacklinkItem> = {}): BacklinkItem {
  return {
    source_id: 'anchor-uuid',
    source_page: 'pages/source.md',
    context: 'snippet context',
    is_linked: true,
    ...over,
  };
}

interface HarnessHandles {
  state: () => any;
  trigger: () => void;
  dismiss: () => void;
}

function renderHarness(showArg?: BacklinkItem) {
  let showRef: ((item: BacklinkItem, el?: HTMLElement | null) => void) | null = null;
  let dismissRef: (() => void) | null = null;
  let lastPreview: any = null;

  const setPreviewFromHook = (p: any) => { lastPreview = p; };

  function Harness() {
    const { preview, showPreview, dismissPreview } = usePreview();
    showRef = showPreview;
    dismissRef = dismissPreview;
    // Sync latest state to a closure so we can read it imperatively.
    setPreviewFromHook(preview);
    return (
      <div>
        <button data-testid="trigger" onClick={() => { if (showRef) showRef(showArg ?? item()); }}>
          trigger
        </button>
        <button data-testid="dismiss" onClick={() => { if (dismissRef) dismissRef(); }}>
          dismiss
        </button>
      </div>
    );
  }

  const utils = render(<Harness />);
  const state = () => {
    if (lastPreview === null) return null;
    return {
      n: lastPreview.noteId,
      t: lastPreview.noteTitle,
      l: lastPreview.loading,
      e: lastPreview.error,
      ctx: (lastPreview.context ?? []).length,
    };
  };
  return {
    state,
    trigger: () => fireEvent.click(utils.getByTestId('trigger')),
    dismiss: () => fireEvent.click(utils.getByTestId('dismiss')),
  } as HarnessHandles;
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe('usePreview (backlink snippet preview)', () => {
  it('showPreview fetches the snippet and transitions loading → ready', async () => {
    vi.mocked(api.getBacklinkSnippet).mockResolvedValue(mockSnippet);
    const h = renderHarness();

    h.trigger();
    expect(h.state()).toMatchObject({ n: 'pages/source.md', l: true, e: false });

    await waitFor(() => {
      expect(h.state()).toMatchObject({ l: false, t: 'Source Note' });
    });
    expect(api.getBacklinkSnippet).toHaveBeenCalledWith('pages/source.md', 'anchor-uuid');
    expect(h.state()).toMatchObject({ ctx: 3, e: false });
  });

  it('does not clobber a newer preview when a stale snippet resolves', async () => {
    let resolveSlow!: (v: typeof mockSnippet) => void;
    vi.mocked(api.getBacklinkSnippet)
      .mockReturnValueOnce(new Promise((r) => { resolveSlow = r; }))
      .mockResolvedValue(mockSnippet);

    // Harness that triggers with the given item each time.
    let currentArg: BacklinkItem = item();
    let showRef: ((it: BacklinkItem, el?: HTMLElement | null) => void) | null = null;
    let lastPreview: any = null;
    function Harness() {
      const { preview, showPreview } = usePreview();
      showRef = showPreview;
      lastPreview = preview;
      return (
        <div>
          <button data-testid="trigger" onClick={() => showRef?.(currentArg)}>
            trigger
          </button>
        </div>
      );
    }
    render(<Harness />);
    const state = () => {
      if (lastPreview === null) return null;
      return {
        n: lastPreview.noteId,
        t: lastPreview.noteTitle,
        l: lastPreview.loading,
        e: lastPreview.error,
      };
    };

    // Show A (slow promise pending).
    fireEvent.click(screen.getByTestId('trigger'));
    // Show B (fast resolve).
    currentArg = item({ source_page: 'pages/b.md' });
    fireEvent.click(screen.getByTestId('trigger'));
    await waitFor(() => {
      expect(state()).toMatchObject({ n: 'pages/b.md', l: false });
    });

    // Stale A resolves now — must not clobber B.
    resolveSlow({ ...mockSnippet, note_id: 'pages/source.md', note_title: 'Stale A' });
    await waitFor(() => {
      expect(state()).toMatchObject({ n: 'pages/b.md' });
    });
    expect(state()).not.toMatchObject({ n: 'pages/source.md' });
  });

  it('shows an error state when the snippet endpoint 404s (missing note/anchor)', async () => {
    vi.mocked(api.getBacklinkSnippet).mockRejectedValue(new Error('Note not found'));
    const h = renderHarness();

    h.trigger();
    await waitFor(() => {
      expect(h.state()).toMatchObject({ l: false, e: true, n: 'pages/source.md' });
    });
  });

  it('dismisses the preview when Ctrl is released', async () => {
    vi.mocked(api.getBacklinkSnippet).mockResolvedValue(mockSnippet);
    const h = renderHarness();

    h.trigger();
    await waitFor(() => {
      expect(h.state()).toMatchObject({ l: false, t: 'Source Note' });
    });

    fireEvent.keyUp(window, { key: 'Control' });
    await waitFor(() => {
      expect(h.state()).toBeNull();
    });
  });

  it('dismissPreview clears the preview', async () => {
    vi.mocked(api.getBacklinkSnippet).mockResolvedValue(mockSnippet);
    const h = renderHarness();

    h.trigger();
    await waitFor(() => {
      expect(h.state()).toMatchObject({ l: false });
    });
    h.dismiss();
    await waitFor(() => {
      expect(h.state()).toBeNull();
    });
  });
});
