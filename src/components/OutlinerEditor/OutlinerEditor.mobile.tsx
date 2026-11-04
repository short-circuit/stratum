/**
 * Mobile variant of the OutlinerEditor.
 *
 * Adapts the BlockNote editor for touch-first use:
 *  - Full-width layout (no side container margins)
 *  - Floating `+` Fab button triggers insert menu (replaces keyboard `/`)
 *  - Marker toggling via floating action button
 *  - Long-press context menu on blocks
 *  - Full-screen math editor modal
 *  - Same auto-save, preview, and dead-link popups as desktop
 *
 * Decomposed during the t_dc58c24c sizing-gate refactor: the loading/error
 * states, save-status chip, marker-badge row, and reminder dialog flow were
 * extracted into shared sub-components and hooks so this variant stays under
 * the 400-line component gate.
 *
 * @module OutlinerEditor/OutlinerEditor.mobile
 */

import { useMemo, useCallback } from 'react';
import { BlockNoteView } from '@blocknote/mantine';
import Box from '@mui/material/Box';
import '@blocknote/core/fonts/inter.css';
import '@blocknote/mantine/style.css';
import AISlashMenu from '../AISlashMenu';
import AIFormattingToolbar from '../AIFormattingToolbar';
import MarkerSuggestMenu from './MarkerSuggestMenu';
import WikiLinkAutocomplete from './WikiLinkAutocomplete';
import ReminderDialog from '../ReminderDialog';
import { EditorLoadingState, EditorErrorState } from './EditorStates';
import { SaveStatusChip } from './SaveStatusChip';
import { MarkerBadgeRow } from './MarkerBadgeRow';
import { useEditorReminders } from './useEditorReminders';
import { useEditorData } from './OutlinerEditor.shared';
import type { Props } from './OutlinerEditor.shared';
import MobileEditorOverlays from './MobileEditorOverlays';
import { useTranslation } from 'react-i18next';

export default function OutlinerEditorMobile(props: Props) {
  const { t } = useTranslation();
  const { pagePath, autoFocus, minHeight = '400px', highlightBlockId } = props;

  const {
    editor,
    status,
    error,
    setStatus,
    setError,
    pageMarkers,
    blockMetaRef,
    persistBlocks,
    mathEdit,
    setMathEdit,
    saving,
    lastSavedAt,
    containerRef,
    preview,
    setPreview,
    deadLinkPopup,
    setDeadLinkPopup,
    navigateRef,
  } = useEditorData(pagePath, autoFocus, minHeight, highlightBlockId, props.focusBlockId);

  const { dialogOpen, dialogDue, handleReminderSave, closeDialog } = useEditorReminders({
    editor,
    blockMetaRef,
    persistBlocks,
    containerRef,
    status,
  });

  const saveLabel = useMemo(() => {
    if (saving) return t('editor.saving');
    if (lastSavedAt) return `Saved ${new Date(lastSavedAt).toLocaleTimeString()}`;
    return null;
  }, [saving, lastSavedAt, t]);

  // -----------------------------------------------------------------------
  // Insert actions
  // -----------------------------------------------------------------------
  const insertBlock = useCallback(
    (type: string, level?: number) => {
      if (!editor) return;
      const ed = editor;
      const block = ed.createBlock();
      if (type === 'heading') {
        block.type = 'heading';
        block.props = { level: level || 1 };
      } else if (type === 'mermaid') {
        block.type = 'mermaid';
        block.content = [{ type: 'text', text: '', styles: {} }];
      } else if (type === 'codeBlock') {
        block.type = 'codeBlock';
        block.props = { language: 'text' };
        block.content = [{ type: 'text', text: '', styles: {} }];
      } else if (type === 'paragraph') {
        block.type = 'paragraph';
      }
      ed.insertBlocks([block], ed.document[ed.document.length - 1]?.id, 'after');
    },
    [editor],
  );

  const insertMath = useCallback(() => {
    if (!editor) return;
    const view = editor?.prosemirrorView;
    if (view) {
      const pos = view.state.selection.from;
      setMathEdit({ latex: '', pos });
    }
  }, [editor, setMathEdit]);

  // -----------------------------------------------------------------------
  // Marker actions
  // -----------------------------------------------------------------------
  const applyMarker = useCallback(
    (marker: string) => {
      if (!editor) return;
      const ed = editor;
      const sel = ed.getSelection();
      if (!sel) return;
      const block = sel[0];
      if (!block) return;
      ed.updateBlock(block.id, {
        type: 'checkListItem',
        props: { checked: marker === 'DONE' },
      });
      // The marker will be persisted on next onChange save
    },
    [editor],
  );

  // -----------------------------------------------------------------------
  // Memoised editor view
  // -----------------------------------------------------------------------
  const editorView = useMemo(
    () => (
      <BlockNoteView
        editor={editor}
        theme={
          document.documentElement.classList.contains('dark')
            ? 'dark'
            : 'light'
        }
        style={{ minHeight, height: '100%' }}
        slashMenu={false}
        formattingToolbar={false}
        linkToolbar={false}
      >
        <AISlashMenu pagePath={pagePath} />
        <AIFormattingToolbar />
        <MarkerSuggestMenu
          blockMetaRef={blockMetaRef}
          onSelect={() => {
            try {
              if (editor) persistBlocks(editor.document);
            } catch (e) {
              console.error('[OutlinerEditor] marker save failed:', e);
            }
          }}
        />
        <WikiLinkAutocomplete pagePath={pagePath} />
      </BlockNoteView>
    ),
    [editor, pagePath, minHeight, blockMetaRef, persistBlocks],
  );

  // -----------------------------------------------------------------------
  // Loading state
  // -----------------------------------------------------------------------
  if (status === 'init' || status === 'loading') {
    return <EditorLoadingState pagePath={pagePath} />;
  }

  // -----------------------------------------------------------------------
  // Error state
  // -----------------------------------------------------------------------
  if (error) {
    return (
      <EditorErrorState
        error={error}
        onRetry={() => {
          setError(null);
          setStatus('init');
        }}
        retryKey="common.retry"
      />
    );
  }

  // -----------------------------------------------------------------------
  // Ready — full mobile UI
  // -----------------------------------------------------------------------
  return (
    <Box
      sx={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        // Full-width: no side margins on mobile
        mx: 0,
        width: '100%',
        position: 'relative',
      }}
    >
      {saveLabel && <SaveStatusChip label={saveLabel} saving={saving} />}
      {pageMarkers.length > 0 && <MarkerBadgeRow markers={pageMarkers} horizontalPadding={1} />}

      {/* Editor view + all floating mobile UI */}
      <MobileEditorOverlays
        editor={editor}
        containerRef={containerRef}
        mathEdit={mathEdit}
        setMathEdit={setMathEdit}
        preview={preview}
        setPreview={setPreview}
        deadLinkPopup={deadLinkPopup}
        setDeadLinkPopup={setDeadLinkPopup}
        navigateRef={navigateRef}
        onInsert={insertBlock}
        onInsertMath={insertMath}
        onApplyMarker={applyMarker}
      >
        {editorView}
      </MobileEditorOverlays>

      <ReminderDialog
        open={dialogOpen}
        initialDue={dialogDue}
        onSave={handleReminderSave}
        onClose={closeDialog}
      />
    </Box>
  );
}
