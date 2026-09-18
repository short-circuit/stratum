/**
 * useEditorData hook and shared editor types for the Outliner editor.
 *
 * Extracted from OutlinerEditor.shared.tsx during the E6 sizing-gate refactor
 * (§2.2 of .sisyphus/refactoring-plan.md). Consumers import these from
 * OutlinerEditor.shared, which re-exports them.
 *
 * Encapsulates all editor state management: creation, block loading, auto-save
 * (debounced), math rendering, wiki-link preview popup, dead-link detection,
 * and hover/click event delegation.
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { useCreateBlockNote } from '@blocknote/react';
import { useNavigate } from 'react-router-dom';
import * as api from '../../lib/commands';
import {
  normalizeContent,
  isWikiLinkHref,
  extractWikiLinkTarget,
  isTagHref,
  extractTagTarget,
} from '../../lib/wikiLinks';
import { useCtrlHeld } from '../../lib/useCtrlHeld';
import { useMathInline, setupMathDblClick } from '../../lib/useMathInline';
import { dtoToBlockNote, blockNoteToDto } from './dtoConverters';
import { detectAndApplyMarkers } from './markerDetection';
import type { BlockMeta } from './dtoConverters';
import { useStore } from '../../stores/appStore';
import { schema } from './editorSchema';

// ---------------------------------------------------------------------------
// Shared Props & types
// ---------------------------------------------------------------------------

export interface Props {
  pagePath: string;
  autoFocus?: boolean;
  minHeight?: string;
}

export type MathEditState = { latex: string; pos: number } | null;
export type PreviewState = {
  content: string;
  pageTitle: string | null;
  pagePath: string;
  position: { x: number; y: number };
  loading: boolean;
} | null;
export type DeadLinkPopupState = {
  target: string;
  position: { x: number; y: number };
} | null;

// ---------------------------------------------------------------------------
// EditorData — returned by useEditorData()
// ---------------------------------------------------------------------------

export interface EditorData {
  editor: ReturnType<typeof useCreateBlockNote>;
  status: string;
  error: string | null;
  setStatus: React.Dispatch<React.SetStateAction<string>>;
  setError: React.Dispatch<React.SetStateAction<string | null>>;
  pageMarkers: string[];
  mathEdit: MathEditState;
  setMathEdit: React.Dispatch<React.SetStateAction<MathEditState>>;
  containerRef: React.RefObject<HTMLDivElement | null>;
  ctrlHeld: React.MutableRefObject<boolean>;
  preview: PreviewState;
  setPreview: React.Dispatch<React.SetStateAction<PreviewState>>;
  deadLinkPopup: DeadLinkPopupState;
  setDeadLinkPopup: React.Dispatch<React.SetStateAction<DeadLinkPopupState>>;
  markDeadLinks: (root: HTMLElement) => void;
  showPreview: (href: string, x: number, y: number) => void;
  dismissPreview: () => void;
  navigateRef: React.MutableRefObject<(path: string) => void>;
  pagePath: string;
  minHeight: string;
  persistBlocks: (blockNoteBlocks: any[]) => void;
  saving: boolean;
  lastSavedAt: number | null;
}

// ---------------------------------------------------------------------------
// useEditorData() — shared editor lifecycle hook
// ---------------------------------------------------------------------------

/**
 * Creates and manages a BlockNote editor instance for the given page.
 *
 * Handles:
 *  - Editor initialisation with custom schema and link behaviour
 *  - Loading block DTOs from the Rust backend and converting them to
 *    BlockNote's internal format (Step 1)
 *  - Debounced save on every document change (Step 2)
 *  - Auto-focus on mount when `autoFocus` is true
 *  - Inline KaTeX rendering via `useMathInline`
 *  - Double-click handler to open the math editor modal
 *  - Dead link detection (highlighting [[wiki-links]] that point to
 *    non-existent pages)
 *  - Wiki-link hover preview popup (Ctrl/Cmd + hover)
 *  - Dead-link popup (clicking an unresolved wiki-link)
 *
 * @returns All state, refs, and callbacks needed by a platform variant.
 */
export function useEditorData(
  pagePath: string,
  autoFocus?: boolean,
  minHeight = '400px',
): EditorData {
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const blockMetaRef = useRef<Map<string, BlockMeta>>(new Map());
  const isProcessingRef = useRef(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState('init');
  const [mathEdit, setMathEdit] = useState<MathEditState>(null);
  const [pageMarkers, setPageMarkers] = useState<string[]>([]);
  // Content snapshot taken right after the initial load, so auto-save can skip
  // writes when nothing changed. Prevents the ED-07 load-rewrite corruption:
  // a programmatic onChange (from replaceBlocks or an external sync) must not
  // re-serialize an untouched document back to disk.
  const loadedSnapshotRef = useRef<string | null>(null);
  const savePendingRef = useRef(false);
  const [saving, setSaving] = useState(false);
  const [lastSavedAt, setLastSavedAt] = useState<number | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const ctrlHeld = useCtrlHeld();
  const hoverTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [preview, setPreview] = useState<PreviewState>(null);
  const navigate = useNavigate();
  const navigateRef = useRef(navigate);
  useEffect(() => {
    navigateRef.current = navigate;
  }, [navigate]);

  const [deadLinkPopup, setDeadLinkPopup] = useState<DeadLinkPopupState>(null);

  const editor = useCreateBlockNote({
    schema,
    links: {
      isValidLink: (href: string) => {
        if (href.startsWith('stratum:') || href.startsWith('stratum-tag:')) return true;
        if (!href) return true;
        return /^(?:https?|ftp|ftps|mailto|tel|callto|sms|cid|xmpp):/i.test(href);
      },
      onClick: (event) => {
        const a = (event.target as HTMLElement).closest?.('a');
        if (!a) return false;
        const href = a.getAttribute('href');
        if (!href) return false;
        if (isTagHref(href)) {
          if (!event.ctrlKey && !event.metaKey) return true;
          const tagName = extractTagTarget(href);
          navigateRef.current('/search?q=' + encodeURIComponent('#' + tagName));
          return true;
        }
        if (isWikiLinkHref(href)) {
          const rect = a.getBoundingClientRect();
          const pos = { x: rect.left, y: rect.bottom };
          let target = extractWikiLinkTarget(href);
          target = target.replace(/[[\]]/g, '').trim().toLowerCase();
          api.resolveLinkTarget(target).then((resolved) => {
            if (resolved.page_path) {
              navigateRef.current(
                '/page/' + encodeURIComponent(resolved.page_path),
              );
            } else {
              setDeadLinkPopup({ target, position: pos });
            }
          });
          return true;
        }
        return false;
      },
    },
  });

  // -----------------------------------------------------------------------
  // Step 1: Load blocks from backend
  // -----------------------------------------------------------------------

  // Deterministic key of what would be written to disk for a given document.
  // Computed through the same conversion the save path uses, so a stale/no-op
  // onChange compares equal and is skipped instead of rewriting the file with a
  // lossy backend round-trip (the ED-07 corruption vector).
  const serializeKey = useCallback((blockNoteBlocks: any[]): string => {
    const dtos = blockNoteToDto(structuredClone(blockNoteBlocks), blockMetaRef.current);
    return JSON.stringify(
      dtos.map((d) => [
        d.id,
        d.content,
        d.marker,
        d.priority,
        d.parent_id,
        d.left_id,
        d.heading_level,
        d.collapsed,
        ...(d.properties || []).slice().sort((a: [string, string], b: [string, string]) => a[0].localeCompare(b[0])),
      ]),
    );
  }, []);

  useEffect(() => {
    api
      .getBlocks(pagePath)
      .then(({ blocks }) => {
        try {
          blockMetaRef.current.clear();
          let bnBlocks: any[] = [];
          for (const b of blocks) b.content = normalizeContent(b.content);
          bnBlocks = dtoToBlockNote(blocks, blockMetaRef.current);
          if (bnBlocks.length > 0) {
            // Suppress the onChange fired by replaceBlocks so the programmatic
            // load does not trigger an auto-save that would rewrite the file
            // before the user has typed anything (ED-07: autosave-on-load).
            isProcessingRef.current = true;
            try {
              editor.replaceBlocks(editor.document, bnBlocks);
            } finally {
              isProcessingRef.current = false;
            }
          }
          // Snapshot the post-load editor state (through the same conversion the
          // save path uses) so auto-save skip logic can compare exact equality.
          try {
            loadedSnapshotRef.current = serializeKey(editor.document);
          } catch {
            loadedSnapshotRef.current = null;
          }
          const markers = [
            ...new Set(
              blocks.filter((b) => b.marker).map((b) => b.marker!),
            ),
          ];
          setPageMarkers(markers);
          setStatus('ready');
        } catch (e) {
          console.error('[OutlinerEditor] replaceBlocks failed:', e);
          setError(String(e));
          setStatus('error');
        }
      })
      .catch((err) => {
        console.error('[OutlinerEditor] getBlocks failed:', err);
        setError(String(err));
        setStatus('error');
      });
  }, [pagePath, editor, serializeKey]);

  // -----------------------------------------------------------------------
  // Step 2: Debounced auto-save on document change with flush-on-unmount.
  //
  // The most recent blocks (and the page they belong to) are kept in a ref so a
  // flush can persist the latest edit with the correct pagePath even after the
  // component unmounts or navigates away. performSave consumes the ref, so a
  // pending timer and a flush can never double-save: only the first of a racing
  // timer/flush actually runs, and a clean-up on unmount / page change flushes
  // whatever is still pending.
  // -----------------------------------------------------------------------
  const pendingSaveRef = useRef<{ blocks: any[]; pagePath: string } | null>(null);

  const performSave = useCallback(async () => {
    const pending = pendingSaveRef.current;
    if (!pending) return;
    pendingSaveRef.current = null;
    if (saveTimer.current) {
      clearTimeout(saveTimer.current);
      saveTimer.current = null;
    }
    const { blocks, pagePath: ctxPath } = pending;
    (window as any).__saveDebug = { called: true, at: Date.now() };
    // Clone blocks before marker detection so the editor document is never
    // mutated if saveBlocks() fails — prevents marker keyword data loss.
    const work = structuredClone(blocks);
    detectAndApplyMarkers(work, ctxPath, blockMetaRef.current);
    // Compare the exact serialization that would be written against the snapshot
    // captured at load. If nothing changed, skip the write entirely — this is the
    // defense-in-depth ED-07 fix: even if a post-load onChange slips through, an
    // untouched document never rewrites the file (idle saves previously fused
    // blocks, dropped markers, and mangled properties via the lossy serialiser).
    const prospectiveKey = serializeKey(work);
    if (loadedSnapshotRef.current !== null && prospectiveKey === loadedSnapshotRef.current) {
      // DEB[/tmp]-probe: mark the no-op skip so we can detect it in the DOM.
      (window as any).__saveDebug = { skipped: true, at: Date.now() };
      return;
    }
    savePendingRef.current = true;
    setSaving(true);
    try {
      const dtos = blockNoteToDto(work, blockMetaRef.current);
      await api.saveBlocks(ctxPath, dtos);
      setLastSavedAt(Date.now());
      (window as any).__saveDebug = { saved: true, at: Date.now() };
      // The document just saved — refresh the baseline so an identical
      // round-trip that fires again (e.g. StrictMode remount) is still a no-op.
      loadedSnapshotRef.current = prospectiveKey;
    } catch (e) {
      console.error('[OutlinerEditor] save failed:', e);
    } finally {
      savePendingRef.current = false;
      setSaving(false);
    }
  }, [serializeKey]);

  const persistBlocks = useCallback(
    (blockNoteBlocks: any[]) => {
      pendingSaveRef.current = { blocks: blockNoteBlocks, pagePath };
      if (saveTimer.current) clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(() => {
        saveTimer.current = null;
        void performSave();
      }, 500);
    },
    [pagePath, performSave],
  );

  // Flush any pending autosave when the page changes or the component unmounts,
  // so typing-then-navigating within the debounce window is never lost. The save
  // uses the pagePath captured in pendingSaveRef, so it targets the correct page
  // even across a navigation. api.saveBlocks is module-level, so the async call
  // survives unmount.
  useEffect(() => {
    return () => {
      if (saveTimer.current) {
        clearTimeout(saveTimer.current);
        saveTimer.current = null;
      }
      if (pendingSaveRef.current) {
        void performSave();
      }
    };
  }, [pagePath, performSave]);

  useEffect(() => {
    if (!editor || status !== 'ready') return;
    return editor.onChange(() => {
      if (isProcessingRef.current) return;
      persistBlocks(editor.document);
    });
  }, [editor, persistBlocks, status]);

  // -----------------------------------------------------------------------
  // Auto-focus on mount
  // -----------------------------------------------------------------------
  useEffect(() => {
    if (!editor || status !== 'ready' || !autoFocus) return;
    requestAnimationFrame(() => {
      editor?.prosemirrorView?.focus();
    });
  }, [editor, status, autoFocus]);

  // -----------------------------------------------------------------------
  // Inline KaTeX rendering via ProseMirror decorations
  // -----------------------------------------------------------------------
  useMathInline(editor, status === 'ready');

  // Double-click on rendered math to open the editor modal
  useEffect(() => {
    if (status !== 'ready') return;
    return setupMathDblClick(
      containerRef.current,
      (latex: string, pos: number) => setMathEdit({ latex, pos }),
    );
  }, [status]);

  // -----------------------------------------------------------------------
  // Dead link detection — highlight [[wiki-links]] to non-existent pages
  // -----------------------------------------------------------------------
  const markDeadLinks = useCallback((root: HTMLElement) => {
    const anchors = root.querySelectorAll<HTMLAnchorElement>(
      'a[data-inline-content-type="link"][href^="stratum:"]',
    );
    if (!anchors.length) return;
    const { getState } = useStore;
    const slugs = new Set(
      getState()
        .pages.map((p) =>
          (p.slug || p.path.replace(/\.md$/i, '')).toLowerCase(),
        ),
    );
    for (const a of anchors) {
      const href = a.getAttribute('href');
      if (!href || href.startsWith('stratum-tag:')) continue;
      const target = extractWikiLinkTarget(href)
        .replace(/[[\]]/g, '')
        .trim()
        .toLowerCase();
      if (!slugs.has(target)) {
        a.style.color = '#d97706';
        a.style.textDecoration = 'underline dashed';
      } else {
        a.style.color = '';
        a.style.textDecoration = '';
      }
    }
  }, []);

  useEffect(() => {
    if (status !== 'ready') return;
    const el = containerRef.current;
    if (!el) return;
    const t = setTimeout(() => markDeadLinks(el), 1000);
    return () => clearTimeout(t);
  }, [status, markDeadLinks]);

  // -----------------------------------------------------------------------
  // Wiki-link preview popup (Ctrl + hover)
  // -----------------------------------------------------------------------
  const showPreview = useCallback(
    (href: string, x: number, y: number) => {
      if (!ctrlHeld.current) return;
      const target = extractWikiLinkTarget(href);
      if (hoverTimer.current) clearTimeout(hoverTimer.current);
      hoverTimer.current = setTimeout(async () => {
        if (!ctrlHeld.current) {
          setPreview(null);
          return;
        }
        setPreview({
          content: '',
          pageTitle: null,
          pagePath: '',
          position: { x, y },
          loading: true,
        });
        try {
          const resolved = await api.resolveLinkTarget(target);
          if (!resolved.page_path || !ctrlHeld.current) {
            setPreview(null);
            return;
          }
          const ctx = await api.getBacklinkContext(
            resolved.page_path,
            pagePath,
          );
          if (!ctrlHeld.current) {
            setPreview(null);
            return;
          }
          setPreview({
            content: ctx?.content || '(empty)',
            pageTitle:
              ctx?.page_title || resolved.title || resolved.slug || target,
            pagePath: resolved.page_path,
            position: { x, y },
            loading: false,
          });
        } catch {
          setPreview(null);
        }
      }, 200);
    },
    [ctrlHeld, pagePath],
  );

  const dismissPreview = useCallback(() => {
    if (hoverTimer.current) {
      clearTimeout(hoverTimer.current);
      hoverTimer.current = null;
    }
    setPreview(null);
  }, []);

  // -----------------------------------------------------------------------
  // Wiki-link hover/click event delegation
  // -----------------------------------------------------------------------
  useEffect(() => {
    const el = containerRef.current;
    if (!el || status !== 'ready') return;

    let currentHovered = '';

    const handleMouseOver = (e: MouseEvent) => {
      const a = (e.target as HTMLElement).closest?.('a');
      if (!a) return;
      const href = a.getAttribute('href');
      if (!href || !isWikiLinkHref(href)) return;
      if (href === currentHovered) return;
      currentHovered = href;
      const rect = a.getBoundingClientRect();
      showPreview(href, rect.left, rect.bottom + 4);
    };

    const handleMouseOut = (e: MouseEvent) => {
      const a = (e.target as HTMLElement).closest?.('a');
      if (!a) {
        currentHovered = '';
        dismissPreview();
        return;
      }
      const href = a.getAttribute('href');
      if (!href || !isWikiLinkHref(href)) {
        currentHovered = '';
        dismissPreview();
        return;
      }
      currentHovered = '';
      dismissPreview();
    };

    const handleLinkPrevent = (e: MouseEvent) => {
      const a = (e.target as HTMLElement).closest?.('a');
      if (!a) return;
      const href = a.getAttribute('href');
      if (!href || !isWikiLinkHref(href)) return;
      e.preventDefault();
    };

    el.addEventListener('mouseover', handleMouseOver);
    el.addEventListener('mouseout', handleMouseOut);
    el.addEventListener('mousedown', handleLinkPrevent, true);
    el.addEventListener('click', handleLinkPrevent, true);
    return () => {
      el.removeEventListener('mouseover', handleMouseOver);
      el.removeEventListener('mouseout', handleMouseOut);
      el.removeEventListener('mousedown', handleLinkPrevent, true);
      el.removeEventListener('click', handleLinkPrevent, true);
    };
  }, [status, showPreview, dismissPreview]);

  // Poll Ctrl/Meta held state and dismiss preview when released
  useEffect(() => {
    const check = () => {
      if (!ctrlHeld.current) dismissPreview();
    };
    const interval = setInterval(check, 100);
    return () => clearInterval(interval);
  }, [ctrlHeld, dismissPreview]);

  return {
    editor,
    status,
    error,
    setStatus,
    setError,
    pageMarkers,
    mathEdit,
    setMathEdit,
    containerRef,
    ctrlHeld,
    preview,
    setPreview,
    deadLinkPopup,
    setDeadLinkPopup,
    markDeadLinks,
    showPreview,
    dismissPreview,
    navigateRef,
    pagePath,
    minHeight,
    persistBlocks,
    saving,
    lastSavedAt,
  };
}
