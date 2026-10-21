/**
 * useEditorAutoSave — debounced auto-save with ED-07 no-op skip and
 * flush-on-unmount for the Outliner editor.
 *
 * Extracted verbatim from useEditorData/index.ts during the t_dc58c24c
 * sizing-gate decomposition. Owns the pending-save ref, the debounce timer, the
 * deterministic-serialization skip that prevents the ED-07 load-rewrite
 * corruption, and the flush-on-unmount safety net. No JSX.
 *
 * @module OutlinerEditor/useEditorData/save
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import type { MutableRefObject } from 'react';
import { blockNoteToDto, type BlockMeta } from '../dtoConverters';
import { detectAndApplyMarkers } from '../markerDetection';
import * as api from '../../../lib/commands';
import { computeSerializedKey, refreshRecentsAfterSave } from './serialization';
import type { EditorData } from './types';

interface UseEditorAutoSaveArgs {
  pagePath: string;
  editor: EditorData['editor'];
  blockMetaRef: MutableRefObject<Map<string, BlockMeta>>;
  loadedSnapshotRef: MutableRefObject<string | null>;
  status: string;
  isProcessingRef: MutableRefObject<boolean>;
  onPersistScheduled: () => void;
}

/**
 * Debounced auto-save on document change with flush-on-unmount.
 *
 * The most recent blocks (and the page they belong to) are kept in a ref so a
 * flush can persist the latest edit with the correct pagePath even after the
 * component unmounts or navigates away. performSave consumes the ref, so a
 * pending timer and a flush can never double-save: only the first of a racing
 * timer/flush actually runs, and a clean-up on unmount / page change flushes
 * whatever is still pending.
 */
export function useEditorAutoSave({
  pagePath,
  editor,
  blockMetaRef,
  loadedSnapshotRef,
  status,
  isProcessingRef,
  onPersistScheduled,
}: UseEditorAutoSaveArgs) {
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const savePendingRef = useRef(false);
  const [saving, setSaving] = useState(false);
  const [lastSavedAt, setLastSavedAt] = useState<number | null>(null);
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
    const prospectiveKey = computeSerializedKey(work, blockMetaRef.current);
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
      // The document just saved — the page's modified_at changed, so the
      // sidebar "Recent" list is stale. Schedule a (debounced, idempotent)
      // reload of the recents store: refreshRecentsAfterSave routes the save
      // into the store's refresh, which coalesces bursts of saves into a
      // single reload, so calling it here never causes a refresh storm.
      // Fires on every successful write — whether via the debounced autosave
      // timer or the flush-on-unmount path.
      refreshRecentsAfterSave();
      // The document just saved — refresh the baseline so an identical
      // round-trip that fires again (e.g. StrictMode remount) is still a no-op.
      loadedSnapshotRef.current = prospectiveKey;
    } catch (e) {
      console.error('[OutlinerEditor] save failed:', e);
    } finally {
      savePendingRef.current = false;
      setSaving(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- blockMetaRef & loadedSnapshotRef are stable refs drilled in from the parent composer.
  }, []);

  const persistBlocks = useCallback(
    (blockNoteBlocks: any[]) => {
      pendingSaveRef.current = { blocks: blockNoteBlocks, pagePath };
      // Refresh reminder chips alongside the pending save so the visible due
      // badge reflects any metadata mutation without waiting for the save.
      onPersistScheduled();
      if (saveTimer.current) clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(() => {
        saveTimer.current = null;
        void performSave();
      }, 500);
    },
    [pagePath, performSave, onPersistScheduled],
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
    // eslint-disable-next-line react-hooks/exhaustive-deps -- isProcessingRef is a stable ref drilled in from the parent composer.
  }, [editor, persistBlocks, status]);

  return { persistBlocks, saving, lastSavedAt };
}
