/**
 * Mobile-only floating UI for the Outliner editor.
 *
 * Extracted from OutlinerEditor.mobile.tsx during the E6 sizing-gate refactor
 * (§2.3 of .sisyphus/refactoring-plan.md): owns the long-press context menu, the
 * floating insert/marker FABs, all three menus, the wiki-link preview popup,
 * the dead-link "create page" popover, and the full-screen math editor modal.
 *
 * Decomposed further during the t_dc58c24c sizing-gate refactor: the long-press
 * context menu state/actions, the insert/marker/actions menus, and the dead-link
 * popover were extracted into hooks and sub-components so this file stays under
 * the 400-line component gate.
 *
 * The parent renders the marker-badge row and passes the memoised BlockNoteView
 * as `children`, which is rendered inside the long-press editor container.
 */

import { useState, useCallback } from 'react';
import type { ReactNode } from 'react';
import Fab from '@mui/material/Fab';
import AddIcon from '@mui/icons-material/Add';
import FlagIcon from '@mui/icons-material/Flag';
import LinkPreviewPopup from '../LinkPreviewPopup';
import MathEditorModal from '../MathEditorModal';
import { MobileInsertMenu } from './MobileInsertMenu';
import { MobileMarkerMenu } from './MobileMarkerMenu';
import { MobileBlockActionsMenu } from './MobileBlockActionsMenu';
import { DeadLinkCreatePopup } from './DeadLinkCreatePopup';
import { useMobileBlockContextMenu } from './useMobileBlockContextMenu';
import { useTranslation } from 'react-i18next';
import type {
  MathEditState,
  PreviewState,
  DeadLinkPopupState,
  EditorData,
} from './useEditorData';

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
  const { t } = useTranslation();
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
  const { contextMenu, longPressHandlers, handleContextAction, closeContextMenu } =
    useMobileBlockContextMenu({ editor });

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
          aria-label={t('editor.insert_block')}
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
          aria-label={t('editor.toggle_marker')}
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

        <MobileInsertMenu
          anchor={insertAnchor}
          onClose={() => setInsertAnchor(null)}
          onInsert={onInsert}
          onInsertMath={onInsertMath}
        />

        <MobileMarkerMenu
          anchor={markerAnchor}
          onClose={() => setMarkerAnchor(null)}
          onApplyMarker={onApplyMarker}
        />

        <MobileBlockActionsMenu
          menu={contextMenu}
          onClose={closeContextMenu}
          onAction={handleContextAction}
        />
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
      <DeadLinkCreatePopup
        popup={deadLinkPopup}
        onClose={() => setDeadLinkPopup(null)}
        navigateRef={navigateRef}
      />

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
