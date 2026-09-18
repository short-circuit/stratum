import { useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useNavigationStore } from '../stores/navigationStore';

/**
 * Helpers for Ctrl/Cmd+click navigation from backlinks.
 *
 * Modifier-click navigation opens the target note in the editor while
 * recording the current note's editor scroll so the user can return to it.
 * Plain (no-modifier) navigation is left untouched and delegates to the
 * router as before.
 *
 * The scroll capture is defensive: it walks the page for the first
 * scrollable editor container (BlockNote editors do not expose a stable
 * scroll element) and best-effort records its offset. Selection restore is
 * deliberately not attempted — BlockNote's remount on page change makes a
 * stable selection pointer impractical, and scroll restoration is the part
 * that preserves "where I was working".
 */

const EDITOR_ANCHOR_SELECTOR =
  '.blocknote-editor-container, .bn-editor, .ProseMirror';

/**
 * Finds the element whose `scrollTop` controls the current note's scrolling.
 * BlockNote editors do not expose a stable scroll element — the scrollable
 * container is the nearest ancestor (e.g. the page view's `overflow:auto`
 * wrapper). Walks up from the first editor anchor found.
 */
function findEditorScrollContainer(): HTMLElement | null {
  const anchor = document.querySelector<HTMLElement>(EDITOR_ANCHOR_SELECTOR);
  if (!anchor) return null;
  let node: HTMLElement | null = anchor;
  while (node) {
    const style = window.getComputedStyle(node);
    const overflowY = style.overflowY;
    // Programmatically scrollable: explicit auto/scroll, or a clipped
    // container (hidden/clip) that actually scrolls. `overflow: visible`
    // ancestors with content overflow are NOT scrollable via scrollTop.
    const isScrollable =
      overflowY === 'auto' ||
      overflowY === 'scroll' ||
      (overflowY !== 'visible' && node.scrollHeight > node.clientHeight);
    if (isScrollable) return node;
    node = node.parentElement;
  }
  return null;
}

/**
 * Captures the current editor scroll offset. Returns null when no scrollable
 * editor container exists (nothing to preserve).
 */
export function captureCurrentEditorScroll(): number | null {
  const container = findEditorScrollContainer();
  if (!container) return null;
  return container.scrollTop;
}

/**
 * Restores a previously captured editor scroll onto the current page.
 * No-op when the record is absent or the page has already delivered it.
 * Delayed two frames to run after the re-mounted editor lays out.
 */
export function useRestoreSourceScroll(pagePath: string | null) {
  const consume = useNavigationStore((s) => s.consume);

  return useCallback(() => {
    if (!pagePath) return;
    const restore = consume(pagePath);
    if (!restore) return;
    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => {
        const container = findEditorScrollContainer();
        if (!container) return;
        container.scrollTop = restore.scrollTop;
      });
    });
  }, [pagePath, consume]);
}

/**
 * Builds a backlink navigation handler that preserves source scroll on
 * Ctrl/Cmd+click and behaves identically to a plain click otherwise.
 * Accepts mouse or touch events — the handler only reads the modifier flags.
 */
export function useBacklinkNavigation() {
  const navigate = useNavigate();
  const push = useNavigationStore((s) => s.push);

  return useCallback(
    (sourcePage: string, e?: { ctrlKey?: boolean; metaKey?: boolean }) => {
      if (e?.ctrlKey || e?.metaKey) {
        const scrollTop = captureCurrentEditorScroll();
        if (scrollTop !== null) push(sourcePage, { scrollTop });
      }
      navigate(`/page/${encodeURIComponent(sourcePage)}`);
    },
    [navigate, push],
  );
}
