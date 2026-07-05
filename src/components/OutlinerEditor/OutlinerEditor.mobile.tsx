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

import { useMemo, useState, useCallback } from 'react';
import { BlockNoteView } from '@blocknote/mantine';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import Alert from '@mui/material/Alert';
import CircularProgress from '@mui/material/CircularProgress';
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
import DiagramIcon from '@mui/icons-material/Schema';
import FlagIcon from '@mui/icons-material/Flag';
import ContentCutIcon from '@mui/icons-material/ContentCut';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import DeleteIcon from '@mui/icons-material/Delete';
import AddCircleIcon from '@mui/icons-material/AddCircle';
import '@blocknote/core/fonts/inter.css';
import '@blocknote/mantine/style.css';
import * as api from '../../lib/commands';
import LinkPreviewPopup from '../LinkPreviewPopup';
import AISlashMenu from '../AISlashMenu';
import AIFormattingToolbar from '../AIFormattingToolbar';
import MathEditorModal from '../MathEditorModal';
import MarkerBadge from '../MarkerBadge';
import { useLongPress } from '../../lib/hooks/useLongPress';
import { useEditorData } from './OutlinerEditor.shared';
import type { Props } from './OutlinerEditor.shared';

export default function OutlinerEditorMobile(props: Props) {
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

  // -----------------------------------------------------------------------
  // Insert menu state
  // -----------------------------------------------------------------------
  const [insertAnchor, setInsertAnchor] = useState<HTMLElement | null>(null);

  // -----------------------------------------------------------------------
  // Marker menu state
  // -----------------------------------------------------------------------
  const MARKER_OPTIONS = ['TODO', 'DOING', 'DONE', 'NOW', 'LATER', 'WAITING', 'CANCELLED'];
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
  // Insert actions
  // -----------------------------------------------------------------------
  const insertBlock = useCallback(
    (type: string, level?: number) => {
      setInsertAnchor(null);
      if (!editor) return;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const ed = editor as any;
      const block = ed.createBlock();
      if (type === 'heading') {
        block.type = 'heading';
        block.props = { level: level || 1 };
      } else if (type === 'mermaid') {
        block.type = 'mermaid';
        block.content = [{ type: 'text', text: '', styles: {} }];
      } else if (type === 'paragraph') {
        block.type = 'paragraph';
      }
      ed.insertBlocks([block], ed.document[ed.document.length - 1]?.id, 'after');
    },
    [editor],
  );

  const insertMath = useCallback(() => {
    setInsertAnchor(null);
    if (!editor) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const view = (editor as any)?.prosemirrorView;
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
      setMarkerAnchor(null);
      if (!editor) return;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const ed = editor as any;
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
  // Context menu actions
  // -----------------------------------------------------------------------
  const handleContextAction = useCallback(
    async (action: string) => {
      if (!contextMenu || !editor) return;
      const { blockId } = contextMenu;
      setContextMenu(null);
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const ed = editor as any;
      switch (action) {
        case 'delete':
          ed.removeBlocks([blockId]);
          break;
        case 'copy':
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
        case 'cut':
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
    },
    [contextMenu, editor],
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
      </BlockNoteView>
    ),
    [editor, pagePath, minHeight],
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
      }}
    >
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
        {editorView}

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
          <MenuItem onClick={() => insertBlock('paragraph')}>
            <ListItemIcon><TextFieldsIcon fontSize="small" /></ListItemIcon>
            <ListItemText>Paragraph</ListItemText>
          </MenuItem>
          <MenuItem onClick={() => insertBlock('heading', 1)}>
            <ListItemIcon><LooksOneIcon fontSize="small" /></ListItemIcon>
            <ListItemText>Heading 1</ListItemText>
          </MenuItem>
          <MenuItem onClick={() => insertBlock('heading', 2)}>
            <ListItemIcon><LooksTwoIcon fontSize="small" /></ListItemIcon>
            <ListItemText>Heading 2</ListItemText>
          </MenuItem>
          <MenuItem onClick={() => insertBlock('heading', 3)}>
            <ListItemIcon><Looks3Icon fontSize="small" /></ListItemIcon>
            <ListItemText>Heading 3</ListItemText>
          </MenuItem>
          <MenuItem onClick={() => insertBlock('mermaid')}>
            <ListItemIcon><DiagramIcon fontSize="small" /></ListItemIcon>
            <ListItemText>Mermaid diagram</ListItemText>
          </MenuItem>
          <MenuItem onClick={insertMath}>
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
            <MenuItem key={m} onClick={() => applyMarker(m)}>
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
          onSave={(latex) => {
            const pos = mathEdit.pos;
            setMathEdit(null);
            if (!latex.trim()) return;
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
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
    </Box>
  );
}
