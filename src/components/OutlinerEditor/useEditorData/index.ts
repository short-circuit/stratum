/**
 * useEditorData hook for the Outliner editor.
 *
 * Extracted from OutlinerEditor.shared.tsx during the E6 sizing-gate refactor.
 * Encapsulates editor state management: creation, block loading, debounced
 * auto-save, math rendering, wiki-link preview, dead-link detection, and
 * hover/click delegation.
 *
 * Directory module (E6.F1 sizing-gate followup) to stay under the 500-line
 * gate: types → ./types, serialization → ./serialization. Following the
 * t_dc58c24c sizing-gate decomposition the auto-save, load, and wiki-link
 * interaction concerns were further split into ./save, ./loadBlocks, and
 * ./wikiLinks; this file is now a thin composer over those internal hooks.
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { useCreateBlockNote } from '@blocknote/react';
import { useNavigate } from 'react-router-dom';
import i18n from '../../../i18n';
import { createAssetEditorOptions } from '../../../lib/assetUpload';
import { buildEditorLinks } from './editorLinks';
import { useCtrlHeld } from '../../../lib/useCtrlHeld';
import { useMathInline, setupMathDblClick } from '../../../lib/useMathInline';
import { applyReminderDecorations } from '../../../lib/useReminderDecorations';
import { schema } from '../editorSchema';
import type { BlockMeta } from '../dtoConverters';
import type {
  MathEditState,
  PreviewState,
  DeadLinkPopupState,
  EditorData,
} from './types';
import { useBlockLoading } from './loadBlocks';
import { useEditorAutoSave } from './save';
import { useWikiLinkInteractions } from './wikiLinks';

// Re-export the split-out types so existing consumers of the useEditorData
// module (OutlinerEditor.shared barrel, MobileEditorOverlays) keep resolving
// them under the same path.
export type {
  Props,
  MathEditState,
  PreviewState,
  DeadLinkPopupState,
  EditorData,
} from './types';

// ---------------------------------------------------------------------------
// useEditorData() — shared editor lifecycle hook
// ---------------------------------------------------------------------------

/**
 * Creates and manages a BlockNote editor instance for the given page.
 *
 * Handles:
 *  - Editor initialisation with custom schema and link behaviour
 *  - Loading block DTOs from the Rust backend and converting them to
 *    BlockNote's internal format (via useBlockLoading)
 *  - Debounced save on every document change (via useEditorAutoSave)
 *  - Auto-focus on mount when `autoFocus` is true
 *  - Inline KaTeX rendering via `useMathInline`
 *  - Double-click handler to open the math editor modal
 *  - Dead link detection (highlighting [[wiki-links]] that point to
 *    non-existent pages) (via useWikiLinkInteractions)
 *  - Wiki-link hover preview popup (Ctrl/Cmd + hover)
 *  - Dead-link popup (clicking an unresolved wiki-link)
 *
 * @returns All state, refs, and callbacks needed by a platform variant.
 */
export function useEditorData(
  pagePath: string,
  autoFocus?: boolean,
  minHeight = '400px',
  highlightBlockId?: string,
  focusBlockId?: string,
): EditorData {
  const blockMetaRef = useRef<Map<string, BlockMeta>>(new Map());
  const isProcessingRef = useRef(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState('init');
  // Bumped whenever block metadata (reminders/markers) is mutated, so reminder
  // decoration effects can re-apply even though blockMetaRef is a mutable ref.
  const [metaVersion, setMetaVersion] = useState(0);
  const [mathEdit, setMathEdit] = useState<MathEditState>(null);
  const [pageMarkers, setPageMarkers] = useState<string[]>([]);
  // Content snapshot taken right after the initial load, so auto-save can skip
  // writes when nothing changed. Prevents the ED-07 load-rewrite corruption:
  // a programmatic onChange (from replaceBlocks or an external sync) must not
  // re-serialize an untouched document back to disk.
  const loadedSnapshotRef = useRef<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const ctrlHeld = useCtrlHeld();
  const [preview, setPreview] = useState<PreviewState>(null);
  const navigate = useNavigate();
  const navigateRef = useRef(navigate);
  useEffect(() => {
    navigateRef.current = navigate;
  }, [navigate]);

  const [deadLinkPopup, setDeadLinkPopup] = useState<DeadLinkPopupState>(null);

  const editor = useCreateBlockNote({
    schema,
    links: buildEditorLinks({
      onNavigate: navigate,
      onDeadLink: (target: string, pos: { x: number; y: number }) =>
        setDeadLinkPopup({ target, position: pos }),
    }),
    ...createAssetEditorOptions(),
  });

  // -----------------------------------------------------------------------
  // Step 1: Load blocks from backend
  // -----------------------------------------------------------------------
  useBlockLoading({
    pagePath,
    editor,
    blockMetaRef,
    loadedSnapshotRef,
    isProcessingRef,
    setStatus,
    setError,
    setPageMarkers,
    onLoaded: () => setMetaVersion((v) => v + 1),
  });

  // -----------------------------------------------------------------------
  // Step 2: Debounced auto-save on document change with flush-on-unmount.
  // -----------------------------------------------------------------------
  const { persistBlocks, saving, lastSavedAt } = useEditorAutoSave({
    pagePath,
    editor,
    blockMetaRef,
    loadedSnapshotRef,
    status,
    isProcessingRef,
    onPersistScheduled: () => setMetaVersion((v) => v + 1),
  });

  // -----------------------------------------------------------------------
  // Auto-focus on mount
  // -----------------------------------------------------------------------
  useEffect(() => {
    if (!editor || status !== 'ready' || !autoFocus) return;
    requestAnimationFrame(() => {
      editor?.prosemirrorView?.focus();
    });
  }, [editor, status, autoFocus]);

  // Focus a specific block after the editor mounts (prose→edit double-click jump).
  // Deferred a frame so the editor is attached; wrapped in try/catch because the
  // target block may not exist (e.g. it was deleted before switching to edit).
  useEffect(() => {
    if (!editor || status !== 'ready' || !focusBlockId) return;
    const timer = setTimeout(() => {
      try {
        editor.setTextCursorPosition(focusBlockId, 'start');
        editor.focus();
      } catch {
        // Target block was removed before switching to edit — leave editor unfocused.
      }
    }, 0);
    return () => clearTimeout(timer);
  }, [editor, status, focusBlockId]);

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
  // Reminder decoration chips — re-applied after load and after any metadata
  // mutation. blockMetaRef is a mutable ref, so the effect is driven off the
  // metaVersion counter instead. applyReminderDecorations clears existing
  // chips first, so repeated application never duplicates; a second delayed
  // pass catches blocks that BlockNote commits asynchronously after load.
  // -----------------------------------------------------------------------
  useEffect(() => {
    if (status !== 'ready') return;
    const el = containerRef.current;
    if (!el) return;
    const translate = (k: string, opts?: Record<string, unknown>): string => i18n.t(k, opts as any) as string;
    const apply = () => applyReminderDecorations(el, blockMetaRef.current, translate);
    const t1 = window.setTimeout(apply, 50);
    const t2 = window.setTimeout(apply, 400);
    return () => {
      window.clearTimeout(t1);
      window.clearTimeout(t2);
    };
  }, [status, metaVersion, editor]);

  // Deep-link / highlight-on-open: when a highlightBlockId is provided and the
  // editor is ready, scroll to and briefly flash the target block once its DOM
  // node exists (BlockNote renders blocks asynchronously after replaceBlocks).
  const scrollToBlock = useCallback((blockId: string) => {
    const el = containerRef.current;
    if (!el) return;
    const blockEl = el.querySelector(`[data-block-id="${blockId}"]`);
    if (!blockEl) return;
    blockEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
    blockEl.classList.add('reminder-flash');
    window.setTimeout(() => blockEl.classList.remove('reminder-flash'), 2500);
  }, []);

  useEffect(() => {
    if (status !== 'ready' || !highlightBlockId) return;
    const id = highlightBlockId;
    let attempts = 0;
    const timer = window.setInterval(() => {
      attempts += 1;
      if (containerRef.current?.querySelector(`[data-block-id="${id}"]`)) {
        window.clearInterval(timer);
        scrollToBlock(id);
      } else if (attempts > 20) {
        window.clearInterval(timer);
      }
    }, 100);
    return () => window.clearInterval(timer);
  }, [status, highlightBlockId, scrollToBlock]);

  // -----------------------------------------------------------------------
  // Dead-link detection + wiki-link hover/click delegation
  // -----------------------------------------------------------------------
  const { markDeadLinks, showPreview, dismissPreview } = useWikiLinkInteractions({
    containerRef,
    status,
    pagePath,
    ctrlHeld,
    setPreview,
  });

  return {
    editor,
    status,
    error,
    setStatus,
    setError,
    pageMarkers,
    blockMetaRef,
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
    scrollToBlock,
  };
}
