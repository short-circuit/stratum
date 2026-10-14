/**
 * Desktop variant of the OutlinerEditor.
 *
 * Provides the desktop-specific UI wrapper (MUI Box, container)
 * and BlockNoteView rendering. All editor state management and
 * shared logic delegates to useEditorData() in the shared module.
 *
 * @module OutlinerEditor/OutlinerEditor.desktop
 */

import { useEffect, useMemo, useState, useCallback } from 'react';
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
import AlarmIcon from '@mui/icons-material/Alarm';
import AlarmOffIcon from '@mui/icons-material/AlarmOff';
import '@blocknote/core/fonts/inter.css';
import '@blocknote/mantine/style.css';
import * as api from '../../lib/commands';
import { useRecentsStore } from '../../stores/recentsStore';
import { speakText, textFromBlock } from '../../lib/audio';
import { getBlockDue, attachReminderChipClick } from '../../lib/reminders/editorReminderHelpers';
import LinkPreviewPopup from '../LinkPreviewPopup';
import AISlashMenu from '../AISlashMenu';
import AIFormattingToolbar from '../AIFormattingToolbar';
import MathEditorModal from '../MathEditorModal';
import MarkerBadge from '../MarkerBadge';
import MarkerSuggestMenu from './MarkerSuggestMenu';
import WikiLinkAutocomplete from './WikiLinkAutocomplete';
import ReminderDialog from '../ReminderDialog';
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
  } = useEditorData(pagePath, autoFocus, minHeight, highlightBlockId);

  const saveLabel = useMemo(() => {
    if (saving) return t('editor.saving');
    if (lastSavedAt) return `Saved ${new Date(lastSavedAt).toLocaleTimeString()}`;
    return null;
  }, [saving, lastSavedAt, t]);

  // Right-click block context menu (desktop read-aloud entry point).
  const [blockMenu, setBlockMenu] = useState<{
    mouseX: number;
    mouseY: number;
    blockId: string;
    due: string | null;
  } | null>(null);

  const onBlockContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    const target = e.target as HTMLElement;
    const blockEl = target.closest('[data-block-id]');
    if (!blockEl) return;
    const blockId = blockEl.getAttribute('data-block-id') || '';
    if (!blockId) return;
    const due = getBlockDue(blockId, blockMetaRef.current);
    setBlockMenu({ mouseX: e.clientX, mouseY: e.clientY, blockId, due });
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
      alert(`${t('aiformatting_toolbar.read_aloud_failed')} ${String(e)}`);
    }
  };

  // -----------------------------------------------------------------------
  // Reminder integration: set / change / clear a block's due reminder.
  // -----------------------------------------------------------------------
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogBlockId, setDialogBlockId] = useState<string | null>(null);
  const [dialogDue, setDialogDue] = useState<string | null>(null);

  // Record the due property on a block's metadata and persist. Mirrors how
  // MarkerSuggestMenu mutates blockMetaRef then triggers a save.
  const applyDue = (blockId: string, isoUtc: string | null) => {
    const existing =
      blockMetaRef.current.get(blockId) ?? { marker: null, priority: null, properties: [] as [string, string][] };
    const properties = existing.properties.filter(([key]) => key !== 'due');
    const updated = { ...existing, properties: isoUtc ? [...properties, ['due', isoUtc] as [string, string]] : properties };
    blockMetaRef.current.set(blockId, updated);
    try {
      if (editor) persistBlocks(editor.document);
    } catch (e) {
      console.error('[OutlinerEditor] reminder save failed:', e);
    }
  };

  const openReminderForBlock = useCallback((blockId: string) => {
    setDialogBlockId(blockId);
    setDialogDue(getBlockDue(blockId, blockMetaRef.current));
    setDialogOpen(true);
  }, [blockMetaRef]);

  const handleReminderSave = (isoUtc: string | null) => {
    const blockId = dialogBlockId;
    setDialogOpen(false);
    if (blockId) applyDue(blockId, isoUtc);
  };

  const blockMenuDue = blockMenu?.due ?? null;

  // Tapping a reminder chip opens the dialog for that block (delegated).
  useEffect(() => {
    const el = containerRef.current;
    if (!el || status !== 'ready') return;
    return attachReminderChipClick(el, openReminderForBlock);
  }, [containerRef, status, openReminderForBlock]);

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
            {t('editor.loading_editor')}
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
            {t('editor.retry')}
          </Button>
        }
      >
        {t('editor.editor_error')} {error}
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
            <ListItemText>{t('editor.read_aloud')}</ListItemText>
          </MenuItem>
          {blockMenuDue === null ? (
            <MenuItem
              onClick={() => {
                if (!blockMenu) return;
                const blockId = blockMenu.blockId;
                setBlockMenu(null);
                openReminderForBlock(blockId);
              }}
            >
              <ListItemIcon><AlarmIcon fontSize="small" /></ListItemIcon>
              <ListItemText>{t('reminders.dialog.set_reminder')}</ListItemText>
            </MenuItem>
          ) : (
            <>
              <MenuItem
                onClick={() => {
                  if (!blockMenu) return;
                  const blockId = blockMenu.blockId;
                  setBlockMenu(null);
                  openReminderForBlock(blockId);
                }}
              >
                <ListItemIcon><AlarmIcon fontSize="small" /></ListItemIcon>
                <ListItemText>{t('reminders.dialog.change_reminder')}</ListItemText>
              </MenuItem>
              <MenuItem
                onClick={() => {
                  if (!blockMenu) return;
                  const blockId = blockMenu.blockId;
                  setBlockMenu(null);
                  applyDue(blockId, null);
                }}
              >
                <ListItemIcon><AlarmOffIcon fontSize="small" /></ListItemIcon>
                <ListItemText>{t('reminders.dialog.clear_reminder')}</ListItemText>
              </MenuItem>
            </>
          )}
        </Menu>

        <ReminderDialog
          open={dialogOpen}
          initialDue={dialogDue}
          onSave={handleReminderSave}
          onClose={() => setDialogOpen(false)}
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
          <Tooltip title={t('editor.create_page')}>
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
                  if (!String(e).includes(t('editor.already_exists'))) {
                    console.error(t('editor.failed_to_create_page'), e);
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
