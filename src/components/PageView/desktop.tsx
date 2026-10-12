import { useState, useCallback } from 'react';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Chip from '@mui/material/Chip';
import Button from '@mui/material/Button';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import ListSubheader from '@mui/material/ListSubheader';
import Divider from '@mui/material/Divider';
import ArrowDropDownIcon from '@mui/icons-material/ArrowDropDown';
import IconButton from '@mui/material/IconButton';
import MicNoneIcon from '@mui/icons-material/MicNone';
import OutlinerEditor from '../OutlinerEditor';
import BacklinksPanel from '../BacklinksPanel';
import SuggestedConnectionsPanel from '../SuggestedConnectionsPanel';
import DictationPanel from '../DictationPanel';
import { usePageView } from './shared';
import * as api from '../../lib/commands';
import { useStore } from '../../stores/appStore';
import { useTranslation } from 'react-i18next';

export default function PageViewDesktop() {
  const { t } = useTranslation();
  const { pagePath, currentPage, editorKey, reindexing, handleReindex, handleDelete } = usePageView();
  const [noteMenuAnchor, setNoteMenuAnchor] = useState<HTMLElement | null>(null);
  const [dictationOpen, setDictationOpen] = useState(false);

  const handleNormalizeFile = useCallback(async () => {
    if (!currentPage) return;
    try {
      await api.normalizeFile(currentPage.path);
    } catch (e) {
      console.error(t('page.normalize_failed'), e);
      useStore.getState().showError(String(e));
    }
  }, [currentPage, t]);

  const closeMenu = () => setNoteMenuAnchor(null);

  if (!pagePath) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <Box sx={{ textAlign: 'center' }}>
          <Typography variant="h5" sx={{ fontWeight: 600, mb: 1 }}>{t('page.stratum_pkm')}</Typography>
          <Typography variant="body2" color="text.secondary">{t('page.select_a_page_from_the_sidebar_or_create_a_new_one')}</Typography>
        </Box>
      </Box>
    );
  }

  if (!currentPage) {
    return <Box sx={{ p: 2 }}><Typography variant="body2" color="text.secondary">{t('common.loading')}</Typography></Box>;
  }

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <Box sx={{ px: 3, py: 1.5, borderBottom: 1, borderColor: 'divider', display: 'flex', alignItems: 'center', gap: 1.5 }}>
        <Typography variant="h6" sx={{ fontWeight: 700, fontSize: '1.1rem' }}>
          {currentPage.title || currentPage.slug}
        </Typography>
        <Chip label={currentPage.path} size="small" variant="outlined" sx={{ fontSize: '0.7rem' }} />
        <Box sx={{ flex: 1 }} />
        <IconButton
          size="small"
          onClick={() => setDictationOpen(o => !o)}
          title={t('page.voice_memo_dictation')}
          color={dictationOpen ? 'primary' : 'default'}
        >
          <MicNoneIcon fontSize="small" />
        </IconButton>
        <Button
          size="small"
          variant="outlined"
          endIcon={<ArrowDropDownIcon />}
          onClick={e => setNoteMenuAnchor(e.currentTarget)}
          disabled={reindexing}
          sx={{ textTransform: 'none', fontSize: '0.75rem' }}
        >
          {reindexing ? '…' : t('page.note')}
        </Button>
        <Menu
          anchorEl={noteMenuAnchor}
          open={Boolean(noteMenuAnchor)}
          onClose={closeMenu}
        >
          <ListSubheader sx={{ lineHeight: '28px', fontSize: '0.7rem', color: 'text.disabled' }}>{t('page.actions')}</ListSubheader>
          <MenuItem onClick={() => { handleReindex(); closeMenu(); }} disabled={reindexing} dense>{t('page.reindex_note')}</MenuItem>
          <MenuItem onClick={() => { handleNormalizeFile(); closeMenu(); }} dense>{t('page.normalize_file')}</MenuItem>
          <Divider />
          <MenuItem onClick={() => { handleDelete(); closeMenu(); }} dense sx={{ color: 'error.main' }}>{t('page.delete_page')}</MenuItem>
        </Menu>
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
      <SuggestedConnectionsPanel pagePath={currentPage.path} />
    </Box>
  );
}
