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
 * @module OutlinerEditor/OutlinerEditor.mobile
 */

import { useMemo, useCallback } from 'react';
import { BlockNoteView } from '@blocknote/mantine';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import Alert from '@mui/material/Alert';
import CircularProgress from '@mui/material/CircularProgress';
import '@blocknote/core/fonts/inter.css';
import '@blocknote/mantine/style.css';
import AISlashMenu from '../AISlashMenu';
import AIFormattingToolbar from '../AIFormattingToolbar';
import MarkerBadge from '../MarkerBadge';
import MarkerSuggestMenu from './MarkerSuggestMenu';
import WikiLinkAutocomplete from './WikiLinkAutocomplete';
import { useEditorData } from './OutlinerEditor.shared';
import type { Props } from './OutlinerEditor.shared';
import MobileEditorOverlays from './MobileEditorOverlays';

export default function OutlinerEditorMobile(props: Props) {
  const { pagePath, autoFocus, minHeight = '400px' } = props;

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
  } = useEditorData(pagePath, autoFocus, minHeight);

  const saveLabel = useMemo(() => {
    if (saving) return 'Saving…';
    if (lastSavedAt) return `Saved ${new Date(lastSavedAt).toLocaleTimeString()}`;
    return null;
  }, [saving, lastSavedAt]);

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
    return (
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: 256,
        }}
      >
        <Box sx={{ textAlign: 'center' }}>
          <CircularProgress size={20} sx={{ mb: 1.5 }} />
          <Typography variant="body2" color="text.secondary">
            Loading editor...
          </Typography>
          <Typography
            variant="caption"
            color="text.disabled"
            sx={{ display: 'block', mt: 0.25 }}
          >
            {pagePath}
          </Typography>
        </Box>
      </Box>
    );
  }

  // -----------------------------------------------------------------------
  // Error state
  // -----------------------------------------------------------------------
  if (error) {
    return (
      <Alert
        severity="error"
        sx={{ m: 2 }}
        action={
          <Button
            size="small"
            color="inherit"
            onClick={() => {
              setError(null);
              setStatus('init');
            }}
          >
            Retry
          </Button>
        }
      >
        Editor error: {error}
      </Alert>
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
      {saveLabel && (
        <Box
          sx={{
            position: 'absolute',
            top: 8,
            right: 12,
            zIndex: 5,
            px: 1,
            py: 0.25,
            borderRadius: 1,
            bgcolor: 'background.paper',
            border: '1px solid',
            borderColor: 'divider',
            boxShadow: 1,
            pointerEvents: 'none',
          }}
        >
          <Typography variant="caption" color={saving ? 'text.secondary' : 'text.disabled'}>
            {saveLabel}
          </Typography>
        </Box>
      )}
      {/* Marker badges row */}
      {pageMarkers.length > 0 && (
        <Box
          sx={{
            px: 1,
            py: 0.5,
            display: 'flex',
            gap: 0.5,
            flexWrap: 'wrap',
            borderBottom: 1,
            borderColor: 'divider',
            bgcolor: 'action.hover',
          }}
        >
          {pageMarkers.map((m) => (
            <MarkerBadge key={m} marker={m} />
          ))}
        </Box>
      )}

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
    </Box>
  );
}
