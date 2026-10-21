/**
 * useMobileBlockContextMenu — mobile long-press block context menu for the
 * Outliner editor.
 *
 * Extracted verbatim from MobileEditorOverlays.tsx during the t_dc58c24c
 * sizing-gate decomposition. Owns the long-press detection and the
 * delete / copy / cut / speak actions. No JSX.
 *
 * @module OutlinerEditor/useMobileBlockContextMenu
 */

import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useLongPress } from '../../lib/hooks/useLongPress';
import { speakText, textFromBlock } from '../../lib/audio';
import type { EditorData } from './useEditorData';

interface ContextMenuState {
  x: number;
  y: number;
  blockId: string;
}

/**
 * Manages the mobile long-press block context menu: the menu state, the
 * long-press handlers to attach to the editor container, and the action handler
 * that performs cut / copy / speak / delete on the target block.
 */
export function useMobileBlockContextMenu({ editor }: { editor: EditorData['editor'] }) {
  const { t } = useTranslation();
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);

  const longPressHandlers = useLongPress({
    onLongPress: (e) => {
      const target = e.target as HTMLElement;
      const blockEl = target.closest('[data-block-id]');
      if (blockEl) {
        const blockId = blockEl.getAttribute('data-block-id') || '';
        if ('touches' in e && e.touches.length > 0) {
          setContextMenu({ x: e.touches[0].clientX, y: e.touches[0].clientY, blockId });
        } else {
          const me = e as React.MouseEvent;
          setContextMenu({ x: me.clientX, y: me.clientY, blockId });
        }
      }
    },
    threshold: 600,
  });

  const handleContextAction = useCallback(
    async (action: string) => {
      if (!contextMenu || !editor) return;
      const { blockId } = contextMenu;
      setContextMenu(null);
      const ed = editor;
      switch (action) {
        case 'delete':
          ed.removeBlocks([blockId]);
          break;
        case 'copy': {
          // BlockNote doesn't expose native copy for a single block;
          // fall back to copying the text content
          const doc = ed.document;
          const block = doc.find((b: { id: string }) => b.id === blockId);
          if (block) {
            const text = block.content
              ?.map((c: { text?: string }) => c.text || '')
              .join('');
            if (text) await navigator.clipboard.writeText(text);
          }
          break;
        }
        case 'cut': {
          const doc2 = ed.document;
          const block2 = doc2.find((b: { id: string }) => b.id === blockId);
          if (block2) {
            const text = block2.content
              ?.map((c: { text?: string }) => c.text || '')
              .join('');
            if (text) await navigator.clipboard.writeText(text);
          }
          ed.removeBlocks([blockId]);
          break;
        }
        case 'speak': {
          const doc3 = ed.document;
          const block3 = doc3.find((b: { id: string }) => b.id === blockId);
          const text = block3
            ? textFromBlock(block3 as { id: string; content?: unknown })
            : '';
          if (!text.trim()) return;
          try {
            await speakText(text);
          } catch (e) {
            console.error('[TTS] read-aloud failed:', e);
            alert(`${t('aiformatting_toolbar.read_aloud_failed')} ${String(e)}`);
          }
          break;
        }
      }
    },
    [contextMenu, editor, t],
  );

  const closeContextMenu = () => setContextMenu(null);

  return { contextMenu, longPressHandlers, handleContextAction, closeContextMenu };
}
