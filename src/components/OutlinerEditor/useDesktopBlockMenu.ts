/**
 * useDesktopBlockMenu — desktop right-click block menu + read-aloud + reminder
 * actions for the Outliner editor.
 *
 * Extracted verbatim from OutlinerEditor.desktop.tsx during the t_dc58c24c
 * sizing-gate decomposition. Composes useEditorReminders and adds the
 * right-click blockMenu state, the read-aloud action, and the reminder
 * open/clear helpers. No JSX.
 *
 * @module OutlinerEditor/useDesktopBlockMenu
 */

import { useState, useCallback } from 'react';
import type { MutableRefObject, RefObject } from 'react';
import { useTranslation } from 'react-i18next';
import { getBlockDue } from '../../lib/reminders/editorReminderHelpers';
import { speakText, textFromBlock } from '../../lib/audio';
import { useEditorReminders } from './useEditorReminders';
import type { DesktopBlockMenuState } from './DesktopBlockContextMenu';
import type { BlockMeta } from './dtoConverters';

interface UseDesktopBlockMenuArgs {
  editor: ReturnType<typeof import('@blocknote/react').useCreateBlockNote>;
  blockMetaRef: MutableRefObject<Map<string, BlockMeta>>;
  persistBlocks: (blocks: any[]) => void;
  containerRef: RefObject<HTMLDivElement | null>;
  status: string;
}

/**
 * Manages the desktop right-click block context menu and its actions (read
 * aloud, set / change / clear reminder). Also wires the shared reminder dialog
 * state via useEditorReminders.
 */
export function useDesktopBlockMenu({
  editor,
  blockMetaRef,
  persistBlocks,
  containerRef,
  status,
}: UseDesktopBlockMenuArgs) {
  const { t } = useTranslation();
  const {
    openReminderForBlock,
    applyDue,
    dialogOpen,
    dialogDue,
    handleReminderSave,
    closeDialog,
  } = useEditorReminders({ editor, blockMetaRef, persistBlocks, containerRef, status });

  const [blockMenu, setBlockMenu] = useState<DesktopBlockMenuState | null>(null);

  const onBlockContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    const target = e.target as HTMLElement;
    const blockEl = target.closest('[data-block-id]');
    if (!blockEl) return;
    const blockId = blockEl.getAttribute('data-block-id') || '';
    if (!blockId) return;
    const due = getBlockDue(blockId, blockMetaRef.current);
    setBlockMenu({ mouseX: e.clientX, mouseY: e.clientY, blockId, due });
  };

  const closeBlockMenu = () => setBlockMenu(null);

  // Reads the given block aloud. Accepts an explicit blockId because the
  // DesktopBlockContextMenu calls onReadAloud after onClose() (which nulls
  // blockMenu), so the id must not be sourced from the closed menu state.
  const readBlockAloud = async (blockId: string) => {
    if (!editor) return;
    const doc = editor.document;
    const block = doc.find((b: { id: string }) => b.id === blockId);
    const text = block ? textFromBlock(block as { id: string; content?: unknown }) : '';
    if (!text.trim()) return;
    try {
      await speakText(text);
    } catch (e) {
      console.error('[TTS] read-aloud failed:', e);
      alert(`${t('aiformatting_toolbar.read_aloud_failed')} ${String(e)}`);
    }
  };

  // Mirror of the MenuItem onClick at desktop 303–309 / 316–322: closes the
  // menu, then opens the reminder dialog for the block.
  const openReminderForBlockId = useCallback(
    (blockId: string) => {
      setBlockMenu(null);
      openReminderForBlock(blockId);
    },
    [openReminderForBlock],
  );

  // Mirror of the MenuItem onClick at desktop 327–333: closes the menu, then
  // clears the block's due property.
  const clearReminderForBlock = (blockId: string) => {
    setBlockMenu(null);
    applyDue(blockId, null);
  };

  return {
    blockMenu,
    onBlockContextMenu,
    closeBlockMenu,
    readBlockAloud,
    openReminderForBlockId,
    clearReminderForBlock,
    dialogOpen,
    dialogDue,
    handleReminderSave,
    closeDialog,
  };
}
