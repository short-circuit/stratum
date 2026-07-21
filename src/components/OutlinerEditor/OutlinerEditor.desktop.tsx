/**
 * Desktop variant of the OutlinerEditor.
 *
 * Provides the desktop-specific UI wrapper (MUI Box, container)
 * and BlockNoteView rendering. All editor state management and
 * shared logic delegates to useEditorData() in the shared module.
 *
 * @module OutlinerEditor/OutlinerEditor.desktop
 */

import { useMemo } from 'react';
import { BlockNoteView } from '@blocknote/mantine';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import Alert from '@mui/material/Alert';
import CircularProgress from '@mui/material/CircularProgress';
import Popover from '@mui/material/Popover';
import IconButton from '@mui/material/IconButton';
import Tooltip from '@mui/material/Tooltip';
import AddCircleIcon from '@mui/icons-material/AddCircle';
import '@blocknote/core/fonts/inter.css';
import '@blocknote/mantine/style.css';
import * as api from '../../lib/commands';
import LinkPreviewPopup from '../LinkPreviewPopup';
import AISlashMenu from '../AISlashMenu';
import AIFormattingToolbar from '../AIFormattingToolbar';
import MathEditorModal from '../MathEditorModal';
import MarkerBadge from '../MarkerBadge';
import { useEditorData } from './OutlinerEditor.shared';
import type { Props } from './OutlinerEditor.shared';

export default function OutlinerEditorDesktop(props: Props) {
  const { pagePath, autoFocus, minHeight = '400px' } = props;

  const {
    editor,
    status,
    error,
    setStatus,
    setError,
    pageMarkers,
    mathEdit,
    setMathEdit,
    containerRef,
    preview,
    setPreview,
    deadLinkPopup,
    setDeadLinkPopup,
    navigateRef,
  } = useEditorData(pagePath, autoFocus, minHeight);

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
      </BlockNoteView>
    ),
    [editor, pagePath, minHeight],
  );

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

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {pageMarkers.length > 0 && (
        <Box
          sx={{
            px: 2,
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
      <div
        ref={containerRef}
        className="blocknote-editor-container"
        style={{ flex: 1, minHeight: 0 }}
      >
        {editorView}

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

        {mathEdit && (
          <MathEditorModal
            initialLatex={mathEdit.latex}
            onSave={(latex) => {
              const pos = mathEdit.pos;
              setMathEdit(null);
              if (!latex.trim()) return;
              const view = (
                editor as any
              )?.prosemirrorView as import('prosemirror-view').EditorView | undefined;
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
