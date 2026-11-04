/**
 * Desktop variant of the OutlinerEditor.
 *
 * Provides the desktop-specific UI wrapper (MUI Box, container) and the
 * BlockNoteView rendering. All editor state management and shared logic
 * delegates to useEditorData() in the shared module.
 *
 * Decomposed during the t_dc58c24c sizing-gate refactor: the loading/error
 * states, save-status chip, marker-badge row, dead-link popup, and the
 * right-click block menu / reminder flow were extracted into shared
 * sub-components and hooks so this variant stays under the 400-line component
 * gate.
 *
 * @module OutlinerEditor/OutlinerEditor.desktop
 */

import { useMemo } from 'react';
import { BlockNoteView } from '@blocknote/mantine';
import Box from '@mui/material/Box';
import '@blocknote/core/fonts/inter.css';
import '@blocknote/mantine/style.css';
import LinkPreviewPopup from '../LinkPreviewPopup';
import AISlashMenu from '../AISlashMenu';
import AIFormattingToolbar from '../AIFormattingToolbar';
import MathEditorModal from '../MathEditorModal';
import MarkerSuggestMenu from './MarkerSuggestMenu';
import WikiLinkAutocomplete from './WikiLinkAutocomplete';
import ReminderDialog from '../ReminderDialog';
import { EditorLoadingState, EditorErrorState } from './EditorStates';
import { SaveStatusChip } from './SaveStatusChip';
import { MarkerBadgeRow } from './MarkerBadgeRow';
import { DeadLinkCreatePopup } from './DeadLinkCreatePopup';
import { DesktopBlockContextMenu } from './DesktopBlockContextMenu';
import { useDesktopBlockMenu } from './useDesktopBlockMenu';
import { useEditorData } from './OutlinerEditor.shared';
import type { Props } from './OutlinerEditor.shared';
import { useTranslation } from 'react-i18next';

export default function OutlinerEditorDesktop(props: Props) {
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

  const {
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
  } = useDesktopBlockMenu({ editor, blockMetaRef, persistBlocks, containerRef, status });

  const saveLabel = useMemo(() => {
    if (saving) return t('editor.saving');
    if (lastSavedAt) return `Saved ${new Date(lastSavedAt).toLocaleTimeString()}`;
    return null;
  }, [saving, lastSavedAt, t]);

  // Memoize the editor view so it doesn't re-render on popup state changes
  // (which would reset scroll position)
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

  if (status === 'init' || status === 'loading') {
    return <EditorLoadingState pagePath={pagePath} />;
  }

  if (error) {
    return (
      <EditorErrorState
        error={error}
        onRetry={() => {
          setError(null);
          setStatus('init');
        }}
        retryKey="editor.retry"
      />
    );
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%', position: 'relative' }}>
      {saveLabel && <SaveStatusChip label={saveLabel} saving={saving} />}
      {pageMarkers.length > 0 && <MarkerBadgeRow markers={pageMarkers} />}
      <div
        ref={containerRef}
        className="blocknote-editor-container"
        style={{ flex: 1, minHeight: 0 }}
        onContextMenu={onBlockContextMenu}
      >
        {editorView}

        <DesktopBlockContextMenu
          menu={blockMenu}
          onClose={closeBlockMenu}
          onReadAloud={(id) => void readBlockAloud(id)}
          onOpenReminder={openReminderForBlockId}
          onClearReminder={clearReminderForBlock}
        />

        <ReminderDialog
          open={dialogOpen}
          initialDue={dialogDue}
          onSave={handleReminderSave}
          onClose={closeDialog}
        />

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

        <DeadLinkCreatePopup
          popup={deadLinkPopup}
          onClose={() => setDeadLinkPopup(null)}
          navigateRef={navigateRef}
        />

        {mathEdit && (
          <MathEditorModal
            initialLatex={mathEdit.latex}
            onSave={(latex) => {
              const pos = mathEdit.pos;
              setMathEdit(null);
              if (!latex.trim()) return;
              const view = editor?.prosemirrorView;
              if (!view) return;
              const text = `$${latex}$`;
              const tr = view.state.tr.replaceWith(
                pos,
                pos + mathEdit.latex.length + 2,
                view.state.schema.text(text),
              );
              view.dispatch(tr);
              view.focus();
            }}
            onCancel={() => setMathEdit(null)}
          />
        )}
      </div>
    </Box>
  );
}
