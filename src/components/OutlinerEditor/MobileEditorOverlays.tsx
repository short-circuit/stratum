/**
 * Mobile-only floating UI for the Outliner editor.
 *
 * Extracted from OutlinerEditor.mobile.tsx during the E6 sizing-gate refactor
 * (§2.3 of .sisyphus/refactoring-plan.md): owns the long-press context menu, the
 * floating insert/marker FABs, all three menus, the wiki-link preview popup,
 * the dead-link "create page" popover, and the full-screen math editor modal.
 *
 * The parent renders the marker-badge row and passes the memoised BlockNoteView
 * as `children`, which is rendered inside the long-press editor container.
 */

import { useState, useCallback } from 'react';
import type { ReactNode } from 'react';
import Fab from '@mui/material/Fab';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import Popover from '@mui/material/Popover';
import IconButton from '@mui/material/IconButton';
import Tooltip from '@mui/material/Tooltip';
import AddIcon from '@mui/icons-material/Add';
import TextFieldsIcon from '@mui/icons-material/TextFields';
import LooksOneIcon from '@mui/icons-material/LooksOne';
import LooksTwoIcon from '@mui/icons-material/LooksTwo';
import Looks3Icon from '@mui/icons-material/Looks3';
import FunctionsIcon from '@mui/icons-material/Functions';
import CodeIcon from '@mui/icons-material/Code';
import DiagramIcon from '@mui/icons-material/Schema';
import FlagIcon from '@mui/icons-material/Flag';
import ContentCutIcon from '@mui/icons-material/ContentCut';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import DeleteIcon from '@mui/icons-material/Delete';
import AddCircleIcon from '@mui/icons-material/AddCircle';
import VolumeUpIcon from '@mui/icons-material/VolumeUp';
import * as api from '../../lib/commands';
import { useRecentsStore } from '../../stores/recentsStore';
import { speakText, textFromBlock } from '../../lib/audio';
import LinkPreviewPopup from '../LinkPreviewPopup';
import MathEditorModal from '../MathEditorModal';
import MarkerBadge from '../MarkerBadge';
import { useLongPress } from '../../lib/hooks/useLongPress';
import type {
  MathEditState,
  PreviewState,
  DeadLinkPopupState,
  EditorData,
} from './useEditorData';

const MARKER_OPTIONS = ['TODO', 'DOING', 'DONE', 'NOW', 'LATER', 'WAITING', 'CANCELLED'];

export interface MobileEditorOverlaysProps {
  editor: EditorData['editor'];
  containerRef: React.RefObject<HTMLDivElement | null>;
  children: ReactNode;
  mathEdit: MathEditState;
  setMathEdit: React.Dispatch<React.SetStateAction<MathEditState>>;
  preview: PreviewState;
  setPreview: React.Dispatch<React.SetStateAction<PreviewState>>;
  deadLinkPopup: DeadLinkPopupState;
  setDeadLinkPopup: React.Dispatch<React.SetStateAction<DeadLinkPopupState>>;
  navigateRef: EditorData['navigateRef'];
  onInsert: (type: string, level?: number) => void;
  onInsertMath: () => void;
  onApplyMarker: (marker: string) => void;
}

export default function MobileEditorOverlays({
  editor,
  containerRef,
  children,
  mathEdit,
  setMathEdit,
  preview,
  setPreview,
  deadLinkPopup,
  setDeadLinkPopup,
  navigateRef,
  onInsert,
  onInsertMath,
  onApplyMarker,
}: MobileEditorOverlaysProps) {
  // -----------------------------------------------------------------------
  // Insert menu state
  // -----------------------------------------------------------------------
  const [insertAnchor, setInsertAnchor] = useState<HTMLElement | null>(null);

  // -----------------------------------------------------------------------
  // Marker menu state
  // -----------------------------------------------------------------------
  const [markerAnchor, setMarkerAnchor] = useState<HTMLElement | null>(null);

  // -----------------------------------------------------------------------
  // Long-press context menu
  // -----------------------------------------------------------------------
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    blockId: string;
  } | null>(null);

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

  // -----------------------------------------------------------------------
  // Context menu actions
  // -----------------------------------------------------------------------
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
            alert(`Read-aloud failed: ${String(e)}`);
          }
          break;
        }
      }
    },
    [contextMenu, editor],
  );

  const onCloseMath = useCallback(() => setMathEdit(null), [setMathEdit]);

  const onSaveMath = useCallback(
    (latex: string) => {
      const pos = mathEdit?.pos ?? 0;
      setMathEdit(null);
      if (!latex.trim()) return;
      const view = editor?.prosemirrorView;
      if (!view) return;
      const text = `$${latex}$`;
      const tr = view.state.tr.replaceWith(
        pos,
        pos + (mathEdit?.latex.length ?? 0) + 2,
        view.state.schema.text(text),
      );
      view.dispatch(tr);
      view.focus();
    },
    [editor, mathEdit, setMathEdit],
  );

  return (
    <>
      {/* Editor container — uses long-press for context menu */}
      <div
        ref={containerRef}
        className="blocknote-editor-container"
        style={{ flex: 1, minHeight: 0, position: 'relative' }}
        // Mobile touch events for long-press on blocks
        onTouchStart={longPressHandlers.onTouchStart}
        onTouchEnd={longPressHandlers.onTouchEnd}
        onTouchMove={longPressHandlers.onTouchMove}
        onMouseDown={longPressHandlers.onMouseDown}
        onMouseUp={longPressHandlers.onMouseUp}
        onMouseMove={longPressHandlers.onMouseMove}
        onMouseLeave={longPressHandlers.onMouseLeave}
      >
        {children}

        {/* Floating + insert button */}
        <Fab
          color="primary"
          size="medium"
          aria-label="Insert block"
          onClick={(e) => setInsertAnchor(e.currentTarget)}
          sx={{
            position: 'absolute',
            bottom: 20,
            right: 20,
            zIndex: 1200,
            boxShadow: 4,
          }}
        >
          <AddIcon />
        </Fab>

        {/* Floating marker toggle */}
        <Fab
          color="secondary"
          size="small"
          aria-label="Toggle marker"
          onClick={(e) => setMarkerAnchor(e.currentTarget)}
          sx={{
            position: 'absolute',
            bottom: 80,
            right: 20,
            zIndex: 1200,
            boxShadow: 4,
          }}
        >
          <FlagIcon fontSize="small" />
        </Fab>

        {/* Insert menu */}
        <Menu
          open={Boolean(insertAnchor)}
          anchorEl={insertAnchor}
          onClose={() => setInsertAnchor(null)}
          anchorOrigin={{ vertical: 'top', horizontal: 'left' }}
          transformOrigin={{ vertical: 'bottom', horizontal: 'left' }}
          slotProps={{ paper: { sx: { minWidth: 180 } } }}
        >
          <MenuItem onClick={() => onInsert('paragraph')}>
            <ListItemIcon><TextFieldsIcon fontSize="small" /></ListItemIcon>
            <ListItemText>Paragraph</ListItemText>
          </MenuItem>
          <MenuItem onClick={() => onInsert('heading', 1)}>
            <ListItemIcon><LooksOneIcon fontSize="small" /></ListItemIcon>
            <ListItemText>Heading 1</ListItemText>
          </MenuItem>
          <MenuItem onClick={() => onInsert('heading', 2)}>
            <ListItemIcon><LooksTwoIcon fontSize="small" /></ListItemIcon>
            <ListItemText>Heading 2</ListItemText>
          </MenuItem>
          <MenuItem onClick={() => onInsert('heading', 3)}>
            <ListItemIcon><Looks3Icon fontSize="small" /></ListItemIcon>
            <ListItemText>Heading 3</ListItemText>
          </MenuItem>
          <MenuItem onClick={() => onInsert('codeBlock')}>
            <ListItemIcon><CodeIcon fontSize="small" /></ListItemIcon>
            <ListItemText>Code block</ListItemText>
          </MenuItem>
          <MenuItem onClick={() => onInsert('mermaid')}>
            <ListItemIcon><DiagramIcon fontSize="small" /></ListItemIcon>
            <ListItemText>Mermaid diagram</ListItemText>
          </MenuItem>
          <MenuItem onClick={onInsertMath}>
            <ListItemIcon><FunctionsIcon fontSize="small" /></ListItemIcon>
            <ListItemText>Math equation</ListItemText>
          </MenuItem>
        </Menu>

        {/* Marker menu */}
        <Menu
          open={Boolean(markerAnchor)}
          anchorEl={markerAnchor}
          onClose={() => setMarkerAnchor(null)}
          anchorOrigin={{ vertical: 'top', horizontal: 'left' }}
          transformOrigin={{ vertical: 'bottom', horizontal: 'left' }}
        >
          {MARKER_OPTIONS.map((m) => (
            <MenuItem key={m} onClick={() => onApplyMarker(m)}>
              <MarkerBadge marker={m} />
            </MenuItem>
          ))}
        </Menu>

        {/* Long-press context menu */}
        <Menu
          open={Boolean(contextMenu)}
          onClose={() => setContextMenu(null)}
          anchorReference="anchorPosition"
          anchorPosition={
            contextMenu
              ? { left: contextMenu.x, top: contextMenu.y }
              : undefined
          }
        >
          <MenuItem onClick={() => handleContextAction('cut')}>
            <ListItemIcon><ContentCutIcon fontSize="small" /></ListItemIcon>
            <ListItemText>Cut</ListItemText>
          </MenuItem>
          <MenuItem onClick={() => handleContextAction('copy')}>
            <ListItemIcon><ContentCopyIcon fontSize="small" /></ListItemIcon>
            <ListItemText>Copy</ListItemText>
          </MenuItem>
          <MenuItem onClick={() => handleContextAction('speak')}>
            <ListItemIcon><VolumeUpIcon fontSize="small" /></ListItemIcon>
            <ListItemText>Read aloud</ListItemText>
          </MenuItem>
          <MenuItem onClick={() => handleContextAction('delete')}>
            <ListItemIcon><DeleteIcon fontSize="small" /></ListItemIcon>
            <ListItemText>Delete</ListItemText>
          </MenuItem>
        </Menu>
      </div>

      {/* Wiki-link preview popup */}
      {preview && (
        <LinkPreviewPopup
          content={preview.content}
          pageTitle={preview.pageTitle}
          pagePath={preview.pagePath}
          position={preview.position}
          loading={preview.loading}
          onClose={() => setPreview(null)}
        />
      )}

      {/* Dead link popup */}
      <Popover
        open={Boolean(deadLinkPopup)}
        anchorReference="anchorPosition"
        anchorPosition={
          deadLinkPopup
            ? {
                left: deadLinkPopup.position.x,
                top: deadLinkPopup.position.y,
              }
            : undefined
        }
        onClose={() => setDeadLinkPopup(null)}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
        transformOrigin={{ vertical: 'top', horizontal: 'left' }}
        disableScrollLock
        slotProps={{ paper: { sx: { p: 0.5 } } }}
      >
        <Tooltip title="Create page">
          <IconButton
            size="small"
            color="primary"
            onClick={async () => {
              if (!deadLinkPopup) return;
              const slug = deadLinkPopup.target;
              try {
                await api.createPage(slug);
                useRecentsStore.getState().refresh();
              } catch (e) {
                if (!String(e).includes('already exists')) {
                  console.error('Failed to create page:', e);
                  setDeadLinkPopup(null);
                  return;
                }
              }
              navigateRef.current('/page/' + encodeURIComponent(slug));
              setDeadLinkPopup(null);
            }}
          >
            <AddCircleIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      </Popover>

      {/* Full-screen math editor modal */}
      {mathEdit && (
        <MathEditorModal
          initialLatex={mathEdit.latex}
          onSave={onSaveMath}
          onCancel={onCloseMath}
        />
      )}
    </>
  );
}
