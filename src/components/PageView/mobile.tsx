import { useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
import RefreshIcon from '@mui/icons-material/Refresh';
import DeleteIcon from '@mui/icons-material/Delete';
import MicNoneIcon from '@mui/icons-material/MicNone';
import AttachFileIcon from '@mui/icons-material/AttachFile';
import OutlinerEditor from '../OutlinerEditor';
import BacklinksPanel from '../BacklinksPanel';
import DictationPanel from '../DictationPanel';
import AttachmentPanel, { FILE_INPUT_ID } from '../AttachmentPanel';
import { usePageView } from './shared';
import { useStore } from '../../stores/appStore';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

export default function PageViewMobile() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { pagePath, currentPage, editorKey, reindexing, handleReindex, handleDelete } = usePageView();
  const [dictationOpen, setDictationOpen] = useState(false);

  // Reminder highlight-on-open: capture a pending highlight for THIS page once,
  // pass it to the editor, then immediately consume it from the global store so
  // it is not replayed on a later navigation (see desktop.tsx for rationale).
  const highlightBlock = useStore((s) => s.highlightBlock);
  const [highlightBlockId, setHighlightBlockId] = useState<string | undefined>(undefined);
  const highlightConsumedRef = useRef(false);
  useEffect(() => {
    if (highlightConsumedRef.current) return;
    if (!currentPage || !highlightBlock) return;
    if (highlightBlock.pagePath !== currentPage.path) return;
    highlightConsumedRef.current = true;
    setHighlightBlockId(highlightBlock.blockId);
    useStore.getState().consumeHighlightBlock();
  }, [currentPage, highlightBlock]);

  if (!pagePath) {
    return null;
  }

  if (!currentPage) {
    return <Box sx={{ p: 2 }}><Typography variant="body2" color="text.secondary">{t('common.loading')}</Typography></Box>;
  }

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column', bgcolor: 'background.default' }}>
      <Box
        sx={{
          px: 1.5,
          py: 1,
          borderBottom: 1,
          borderColor: 'divider',
          display: 'flex',
          alignItems: 'center',
          gap: 1,
          minHeight: 48,
        }}
      >
        <IconButton edge="start" onClick={() => navigate(-1)} size="small">
          <ArrowBackIcon />
        </IconButton>

        <Typography
          variant="subtitle1"
          sx={{ fontWeight: 600, flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}
        >
          {currentPage.title || currentPage.slug}
        </Typography>

        <IconButton size="small" onClick={() => setDictationOpen(o => !o)} title={t('page.voice_memo')}>
          <MicNoneIcon fontSize="small" />
        </IconButton>

        <IconButton
          size="small"
          onClick={() => document.getElementById(FILE_INPUT_ID)?.click()}
          title={t('asset.attach')}
        >
          <AttachFileIcon fontSize="small" />
        </IconButton>

        <IconButton size="small" onClick={handleReindex} disabled={reindexing} title={t('page.reindex')}>
          <RefreshIcon fontSize="small" />
        </IconButton>

        <IconButton size="small" onClick={handleDelete} color="error" title={t('sidebar.delete_page')}>
          <DeleteIcon fontSize="small" />
        </IconButton>
      </Box>

      {dictationOpen && (
        <DictationPanel
          pagePath={currentPage.path}
          onInserted={() => {
            setDictationOpen(false);
            void useStore.getState().openPage(currentPage.path);
          }}
        />
      )}

      <Box sx={{ flex: 1, overflow: 'auto' }}>
        <OutlinerEditor key={editorKey} pagePath={currentPage.path} highlightBlockId={highlightBlockId} />
      </Box>

      <AttachmentPanel pagePath={currentPage.path} />
      <BacklinksPanel pagePath={currentPage.path} />
    </Box>
  );
}
