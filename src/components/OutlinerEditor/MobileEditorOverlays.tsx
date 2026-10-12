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
import IconButton from '@mui/material/IconButton';
import Menu from '@mui/material/Menu';
import Divider from '@mui/material/Divider';
import AddIcon from '@mui/icons-material/Add';
import FlagIcon from '@mui/icons-material/Flag';
import MoreHorizIcon from '@mui/icons-material/MoreHoriz';
import LinkPreviewPopup from '../LinkPreviewPopup';
import MathEditorModal from '../MathEditorModal';
import { MobileInsertMenu } from './MobileInsertMenu';
import { MobileMarkerMenu } from './MobileMarkerMenu';
import { MobileBlockActionsMenu } from './MobileBlockActionsMenu';
import { DeadLinkCreatePopup } from './DeadLinkCreatePopup';
import { useMobileBlockContextMenu } from './useMobileBlockContextMenu';
import { InsertMenuItems, MarkerMenuItems } from './editorActionMenuItems';
import { MARKER_OPTIONS } from './markerOptions';
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
  /**
   * "Row mode" for journal-list rows: the editor is rendered inside a
   * content-sized row (not a full-page viewport), so the two bottom-anchored
   * FABs would overflow the container and overlap the row above / the text.
   * When true, this component renders a SINGLE inconspicuous action IconButton
   * pinned inside a reserved top-right region (container min-height + content
   * right padding) instead, opening one combined Insert + Marker menu.
   * All other capabilities (long-press menu, preview, dead link, math modal)
   * are unaffected. Default false (full-page editor keeps the FABs).
   */
  rowActions?: boolean;
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
  rowActions = false,
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
  // Row-mode combined actions menu state (journal list rows)
  // -----------------------------------------------------------------------
  const [rowMenuAnchor, setRowMenuAnchor] = useState<HTMLElement | null>(null);

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
      {/* Editor container — uses long-press for context menu.
          Row mode: the container reserves a top-right action region
          (min-height + right padding) so the single action button can NEVER
          overlap the row content or overflow into adjacent rows. */}
      <div
        ref={containerRef}
        className="blocknote-editor-container"
        style={{
          flex: 1,
          minHeight: rowActions ? 64 : 0,
          paddingRight: rowActions ? 56 : 0,
          position: 'relative',
        }}
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

        {rowActions ? (
          /* Single inconspicuous row action button (journal list rows).
             40px touch target; the container's reserved region keeps it from
             overlapping text or neighbouring rows. Opens ONE combined menu
             (insert + markers) — replaces the two FABs whose bottom-anchored
             absolute positioning overflowed the content-sized row (bug
             t_1100de53). */
          <IconButton
            aria-label={t('editor.row_actions')}
            onClick={(e) => setRowMenuAnchor(e.currentTarget)}
            sx={{
              position: 'absolute',
              top: 8,
              right: 8,
              width: 40,
              height: 40,
              zIndex: 1200,
              color: 'text.secondary',
            }}
          >
            <MoreHorizIcon />
          </IconButton>
        ) : (
          <>
            {/* Floating + insert button (full-page editor only) */}
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
          </>
        )}

        {rowActions && (
          <Menu
            open={Boolean(rowMenuAnchor)}
            anchorEl={rowMenuAnchor}
            onClose={() => setRowMenuAnchor(null)}
            anchorOrigin={{ vertical: 'top', horizontal: 'left' }}
            transformOrigin={{ vertical: 'bottom', horizontal: 'left' }}
            slotProps={{ paper: { sx: { minWidth: 180, maxHeight: '60vh', overflow: 'auto' } } }}
          >
            <InsertMenuItems onInsert={onInsert} onInsertMath={onInsertMath} />
            <Divider />
            <MarkerMenuItems markers={MARKER_OPTIONS} onApplyMarker={onApplyMarker} />
          </Menu>
        )}

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
