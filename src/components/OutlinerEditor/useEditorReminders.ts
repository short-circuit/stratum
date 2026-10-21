/**
 * useEditorReminders — reminder dialog state and reminder-chip click delegation
 * for the Outliner editor.
 *
 * Extracted verbatim from OutlinerEditor.desktop.tsx / OutlinerEditor.mobile.tsx
 * during the t_dc58c24c sizing-gate decomposition. Owns the `due` property
 * mutation (applyDue), the dialog open/close state, and the delegated effect
 * that opens the dialog when a reminder chip is tapped. No JSX — so the 400-line
 * component gate does not apply, and the file stays well under 500.
 *
 * @module OutlinerEditor/useEditorReminders
 */

import { useState, useCallback, useEffect } from 'react';
import type { MutableRefObject, RefObject } from 'react';
import { getBlockDue, attachReminderChipClick } from '../../lib/reminders/editorReminderHelpers';
import type { BlockMeta } from './dtoConverters';

interface UseEditorRemindersArgs {
  editor: ReturnType<typeof import('@blocknote/react').useCreateBlockNote>;
  blockMetaRef: MutableRefObject<Map<string, BlockMeta>>;
  persistBlocks: (blocks: any[]) => void;
  containerRef: RefObject<HTMLDivElement | null>;
  status: string;
}

/**
 * Manages the block-reminder dialog: open/close state, the due value for the
 * block being edited, the `applyDue` metadata mutation, and the delegated click
 * handler that opens the dialog from a reminder chip.
 */
export function useEditorReminders({
  editor,
  blockMetaRef,
  persistBlocks,
  containerRef,
  status,
}: UseEditorRemindersArgs) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogBlockId, setDialogBlockId] = useState<string | null>(null);
  const [dialogDue, setDialogDue] = useState<string | null>(null);

  // Record the due property on a block's metadata and persist. Mirrors how
  // MarkerSuggestMenu mutates blockMetaRef then triggers a save.
  const applyDue = (blockId: string, isoUtc: string | null) => {
    const existing =
      blockMetaRef.current.get(blockId) ?? { marker: null, priority: null, properties: [] as [string, string][] };
    const properties = existing.properties.filter(([key]) => key !== 'due');
    const updated = { ...existing, properties: isoUtc ? [...properties, ['due', isoUtc] as [string, string]] : properties };
    blockMetaRef.current.set(blockId, updated);
    try {
      if (editor) persistBlocks(editor.document);
    } catch (e) {
      console.error('[OutlinerEditor] reminder save failed:', e);
    }
  };

  const openReminderForBlock = useCallback((blockId: string) => {
    setDialogBlockId(blockId);
    setDialogDue(getBlockDue(blockId, blockMetaRef.current));
    setDialogOpen(true);
  }, [blockMetaRef]);

  // Unifies the desktop (141–145) and mobile (135–149) handlers: close the
  // dialog, then apply the due value if a block was targeted. Semantics are
  // identical to both original variants.
  const handleReminderSave = (isoUtc: string | null) => {
    const blockId = dialogBlockId;
    closeDialog();
    if (blockId) applyDue(blockId, isoUtc);
  };

  const closeDialog = () => setDialogOpen(false);

  // Tapping a reminder chip opens the dialog for that block (delegated).
  useEffect(() => {
    const el = containerRef.current;
    if (!el || status !== 'ready') return;
    return attachReminderChipClick(el, openReminderForBlock);
  }, [containerRef, status, openReminderForBlock]);

  return {
    dialogOpen,
    dialogDue,
    openReminderForBlock,
    handleReminderSave,
    closeDialog,
    applyDue,
  };
}
