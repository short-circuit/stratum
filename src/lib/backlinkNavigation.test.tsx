import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, fireEvent, act, cleanup } from '@testing-library/react';
import { useEffect } from 'react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import {
  useBacklinkNavigation,
  captureCurrentEditorScroll,
  useRestoreSourceScroll,
} from './backlinkNavigation';
import { useNavigationStore } from '../stores/navigationStore';

/**
 * QA coverage for Ctrl/Cmd+click backlink navigation.
 *
 * Guards the contract from task t_5a010d2b:
 *  - Ctrl+click on a backlink navigates to the target note in the editor,
 *    recording the source editor's scroll so it can be restored on return.
 *  - Plain click behavior is unchanged (direct routing, no scroll record).
 *  - Missing editor scroll containers degrade to a plain navigation (no crash).
 *  - Returning to a source page restores the recorded scroll one-shot.
 *
 * Navigation is observed through a real MemoryRouter (a live LocationReader),
 * so these tests exercise the actual useNavigate call, not a mock.
 */

/** Renders the real useBacklinkNavigation hook in a router context. */
function renderNavigationHarness() {
  let handler: ((path: string, e?: MouseEvent) => void) | null = null;
  let locationPath = '';

  function LocationReader() {
    const location = useLocation();
    useEffect(() => {
      locationPath = location.pathname;
    }, [location]);
    return null;
  }

  function Harness() {
    const navigateBacklink = useBacklinkNavigation();
    handler = navigateBacklink;
    return (
      <div>
        <button data-testid="trigger" onClick={(e) => handler?.('pages/target.md', e as unknown as MouseEvent)}>
          go
        </button>
      </div>
    );
  }

  const utils = render(
    <MemoryRouter initialEntries={['/page/pages/source.md']}>
      <Harness />
      <LocationReader />
    </MemoryRouter>,
  );

  const clickTrigger = (init?: MouseEventInit) => {
    fireEvent.click(utils.getByTestId('trigger'), init);
  };

  return {
    pathname: () => locationPath,
    clickTrigger,
    store: useNavigationStore,
  };
}

/** Builds a scrollable editor container stub in the document body. */
function stubEditorScrollContainer(scrollTop = 320): HTMLElement {
  const anchor = document.createElement('div');
  anchor.className = 'bn-editor';
  const scrollable = document.createElement('div');
  scrollable.className = 'scroll-host';
  Object.defineProperty(anchor, 'scrollHeight', { value: 1000 });
  Object.defineProperty(anchor, 'clientHeight', { value: 400 });
  Object.defineProperty(scrollable, 'scrollTop', { value: scrollTop, writable: true });
  Object.defineProperty(scrollable, 'scrollHeight', { value: 2000 });
  Object.defineProperty(scrollable, 'clientHeight', { value: 500 });
  scrollable.appendChild(anchor);
  document.body.appendChild(scrollable);

  vi.spyOn(window, 'getComputedStyle').mockImplementation((el) => {
    const base = { overflowY: 'visible' } as CSSStyleDeclaration;
    if (el.className === 'scroll-host') {
      (base as unknown as Record<string, string>).overflowY = 'auto';
    }
    return base;
  });

  return scrollable;
}

beforeEach(() => {
  useNavigationStore.setState({ from: null });
  document.body.innerHTML = '';
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe('useBacklinkNavigation — Ctrl/Cmd+click preserves source state', () => {
  it('Ctrl+click navigates to the target note and pushes a scroll record', () => {
    stubEditorScrollContainer(512);
    const h = renderNavigationHarness();

    h.clickTrigger({ ctrlKey: true });

    expect(decodeURIComponent(h.pathname())).toBe('/page/pages/target.md');
    // A scroll record for the source page is now pending one-shot restore.
    expect(h.store.getState().consume('pages/target.md')).toEqual({ scrollTop: 512 });
  });

  it('Meta+click behaves identically to Ctrl+click', () => {
    stubEditorScrollContainer(128);
    const h = renderNavigationHarness();

    h.clickTrigger({ metaKey: true });

    expect(decodeURIComponent(h.pathname())).toBe('/page/pages/target.md');
    expect(h.store.getState().consume('pages/target.md')).toEqual({ scrollTop: 128 });
  });

  it('plain click navigates without recording a scroll restore', () => {
    stubEditorScrollContainer(512);
    const h = renderNavigationHarness();

    h.clickTrigger({});

    expect(decodeURIComponent(h.pathname())).toBe('/page/pages/target.md');
    // No pending restore record: consume returns null.
    expect(h.store.getState().consume('pages/target.md')).toBeNull();
  });

  it('navigates without crashing when no editor scroll container exists', () => {
    // No .bn-editor / .ProseMirror element in the DOM.
    const h = renderNavigationHarness();

    expect(() => h.clickTrigger({ ctrlKey: true })).not.toThrow();
    expect(decodeURIComponent(h.pathname())).toBe('/page/pages/target.md');
    // No scroll could be captured, so nothing is recorded.
    expect(h.store.getState().consume('pages/target.md')).toBeNull();
  });
});

describe('useRestoreSourceScroll — returning restores the source scroll one-shot', () => {
  it('applies the recorded scroll to the restored page after two frames', async () => {
    vi.useFakeTimers();
    useNavigationStore.getState().push('pages/source.md', { scrollTop: 777 });
    const scrollable = stubEditorScrollContainer(0);

    let restore!: () => void;
    function Harness() {
      restore = useRestoreSourceScroll('pages/source.md');
      return null;
    }
    render(
      <MemoryRouter>
        <Harness />
      </MemoryRouter>,
    );

    act(() => {
      restore();
      // Flush the two requestAnimationFrame callbacks synchronously in jsdom.
      vi.advanceTimersByTime(100);
      vi.advanceTimersByTime(100);
    });

    expect(scrollable.scrollTop).toBe(777);
    // One-shot: a second restore for the same page is a no-op.
    act(() => restore());
    expect(scrollable.scrollTop).toBe(777);
  });

  it('is a silent no-op when no record exists for the page', () => {
    stubEditorScrollContainer(0);
    let restore!: () => void;
    function Harness() {
      restore = useRestoreSourceScroll('pages/other.md');
      return null;
    }
    render(
      <MemoryRouter>
        <Harness />
      </MemoryRouter>,
    );

    expect(() => act(() => restore())).not.toThrow();
  });

  it('does nothing when pagePath is null (landing route)', () => {
    stubEditorScrollContainer(10);
    let restore!: () => void;
    function Harness() {
      restore = useRestoreSourceScroll(null);
      return null;
    }
    render(
      <MemoryRouter>
        <Harness />
      </MemoryRouter>,
    );

    expect(() => act(() => restore())).not.toThrow();
  });
});

describe('captureCurrentEditorScroll', () => {
  it('returns the scrollTop of the found editor scroll container', () => {
    stubEditorScrollContainer(444);
    expect(captureCurrentEditorScroll()).toBe(444);
  });

  it('returns null when no editor anchor exists in the DOM', () => {
    document.body.innerHTML = '<div>no editor here</div>';
    expect(captureCurrentEditorScroll()).toBeNull();
  });
});
