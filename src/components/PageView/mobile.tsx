import { useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
import RefreshIcon from '@mui/icons-material/Refresh';
import DeleteIcon from '@mui/icons-material/Delete';
import MicNoneIcon from '@mui/icons-material/MicNone';
import OutlinerEditor from '../OutlinerEditor';
import BacklinksPanel from '../BacklinksPanel';
import DictationPanel from '../DictationPanel';
import { usePageView } from './shared';
import { useStore } from '../../stores/appStore';
import { useState } from 'react';

export default function PageViewMobile() {
  const navigate = useNavigate();
  const { pagePath, currentPage, editorKey, reindexing, handleReindex, handleDelete } = usePageView();
  const [dictationOpen, setDictationOpen] = useState(false);

  if (!pagePath) {
    return null;
  }

  if (!currentPage) {
    return <Box sx={{ p: 2 }}><Typography variant="body2" color="text.secondary">Loading...</Typography></Box>;
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

        <IconButton size="small" onClick={() => setDictationOpen(o => !o)} title="Voice memo">
          <MicNoneIcon fontSize="small" />
        </IconButton>

        <IconButton size="small" onClick={handleReindex} disabled={reindexing} title="Reindex">
          <RefreshIcon fontSize="small" />
        </IconButton>

        <IconButton size="small" onClick={handleDelete} color="error" title="Delete page">
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
        <OutlinerEditor key={editorKey} pagePath={currentPage.path} />
      </Box>

      <BacklinksPanel pagePath={currentPage.path} />
    </Box>
  );
}
