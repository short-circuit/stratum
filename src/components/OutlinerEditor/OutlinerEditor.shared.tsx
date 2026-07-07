/**
 * Shared logic and data hooks for the OutlinerEditor platform variants.
 *
 * Provides the BlockNote schema, shared Props interface, and the useEditorData()
 * hook that encapsulates all editor state management: creation, block loading,
 * auto-save (debounced), math rendering, wiki-link preview popup, dead-link
 * detection, and hover/click event delegation.
 *
 * @module OutlinerEditor/OutlinerEditor.shared
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { useCreateBlockNote } from '@blocknote/react';
import { BlockNoteSchema, defaultBlockSpecs } from '@blocknote/core';
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
import { createMermaidSpec } from '../MermaidBlock';
import { dtoToBlockNote, blockNoteToDto } from './dtoConverters';
import { detectAndApplyMarkers } from './markerDetection';
import type { BlockMeta } from './dtoConverters';
import { useStore } from '../../stores/appStore';

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

export const schema = BlockNoteSchema.create({
  blockSpecs: {
    ...defaultBlockSpecs,
    mermaid: createMermaidSpec(),
  },
});

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
  useEffect(() => {
    api
      .getBlocks(pagePath)
      .then(({ blocks }) => {
        try {
          blockMetaRef.current.clear();
          for (const b of blocks) b.content = normalizeContent(b.content);
          const bnBlocks = dtoToBlockNote(blocks, blockMetaRef.current);
          if (bnBlocks.length > 0) {
            editor.replaceBlocks(editor.document, bnBlocks);
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
  }, [pagePath, editor]);

  // -----------------------------------------------------------------------
  // Step 2: Debounced auto-save on document change
  // -----------------------------------------------------------------------
  const persistBlocks = useCallback(
    (blockNoteBlocks: any[]) => {
      if (saveTimer.current) clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(async () => {
        try {
          // Clone blocks before marker detection so the editor document is never
          // mutated if saveBlocks() fails — prevents marker keyword data loss.
          detectAndApplyMarkers(
            structuredClone(blockNoteBlocks),
            pagePath,
            blockMetaRef.current,
          );
          const dtos = blockNoteToDto(blockNoteBlocks, blockMetaRef.current);
          await api.saveBlocks(pagePath, dtos);
        } catch (e) {
          console.error('[OutlinerEditor] save failed:', e);
        }
      }, 500);
    },
    [pagePath],
  );

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
      (editor as any)?.prosemirrorView?.focus();
    });
  }, [editor, status, autoFocus]);

  // -----------------------------------------------------------------------
  // Inline KaTeX rendering via ProseMirror decorations
  // -----------------------------------------------------------------------
  useMathInline(editor as any, status === 'ready');

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
  };
}
