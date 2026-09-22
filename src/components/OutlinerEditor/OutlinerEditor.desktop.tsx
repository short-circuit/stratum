/**
 * Desktop variant of the OutlinerEditor.
 *
 * Provides the desktop-specific UI wrapper (MUI Box, container)
 * and BlockNoteView rendering. All editor state management and
 * shared logic delegates to useEditorData() in the shared module.
 *
 * @module OutlinerEditor/OutlinerEditor.desktop
 */

import { useMemo, useState } from 'react';
import { BlockNoteView } from '@blocknote/mantine';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import Alert from '@mui/material/Alert';
import CircularProgress from '@mui/material/CircularProgress';
import Popover from '@mui/material/Popover';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import IconButton from '@mui/material/IconButton';
import Tooltip from '@mui/material/Tooltip';
import VolumeUpIcon from '@mui/icons-material/VolumeUp';
import AddCircleIcon from '@mui/icons-material/AddCircle';
import '@blocknote/core/fonts/inter.css';
import '@blocknote/mantine/style.css';
import * as api from '../../lib/commands';
import { useRecentsStore } from '../../stores/recentsStore';
import { speakText, textFromBlock } from '../../lib/audio';
import LinkPreviewPopup from '../LinkPreviewPopup';
import AISlashMenu from '../AISlashMenu';
import AIFormattingToolbar from '../AIFormattingToolbar';
import MathEditorModal from '../MathEditorModal';
import MarkerBadge from '../MarkerBadge';
import MarkerSuggestMenu from './MarkerSuggestMenu';
import WikiLinkAutocomplete from './WikiLinkAutocomplete';
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

  // Right-click block context menu (desktop read-aloud entry point).
  const [blockMenu, setBlockMenu] = useState<{
    mouseX: number;
    mouseY: number;
    blockId: string;
  } | null>(null);

  const onBlockContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    const target = e.target as HTMLElement;
    const blockEl = target.closest('[data-block-id]');
    if (!blockEl) return;
    const blockId = blockEl.getAttribute('data-block-id') || '';
    if (!blockId) return;
    setBlockMenu({ mouseX: e.clientX, mouseY: e.clientY, blockId });
  };

  const readBlockAloud = async () => {
    if (!blockMenu || !editor) return;
    const { blockId } = blockMenu;
    setBlockMenu(null);
    const doc = editor.document;
    const block = doc.find((b: { id: string }) => b.id === blockId);
    const text = block ? textFromBlock(block as { id: string; content?: unknown }) : '';
    if (!text.trim()) return;
    try {
      await speakText(text);
    } catch (e) {
      console.error('[TTS] read-aloud failed:', e);
      alert(`Read-aloud failed: ${String(e)}`);
    }
  };

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
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%', position: 'relative' }}>
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
        onContextMenu={onBlockContextMenu}
      >
        {editorView}

        <Menu
          open={Boolean(blockMenu)}
          onClose={() => setBlockMenu(null)}
          anchorReference="anchorPosition"
          anchorPosition={
            blockMenu ? { left: blockMenu.mouseX, top: blockMenu.mouseY } : undefined
          }
        >
          <MenuItem onClick={readBlockAloud}>
            <ListItemIcon><VolumeUpIcon fontSize="small" /></ListItemIcon>
            <ListItemText>Read aloud</ListItemText>
          </MenuItem>
        </Menu>

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
