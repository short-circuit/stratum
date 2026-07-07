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
import OutlinerEditor from '../OutlinerEditor';
import BacklinksPanel from '../BacklinksPanel';
import SuggestedConnectionsPanel from '../SuggestedConnectionsPanel';
import { usePageView } from './shared';
import * as api from '../../lib/commands';
import { useStore } from '../../stores/appStore';

export default function PageViewDesktop() {
  const { pagePath, currentPage, editorKey, reindexing, handleReindex, handleDelete } = usePageView();
  const [noteMenuAnchor, setNoteMenuAnchor] = useState<HTMLElement | null>(null);

  const handleNormalizeFile = useCallback(async () => {
    if (!currentPage) return;
    try {
      await api.normalizeFile(currentPage.path);
      console.log('File normalized:', currentPage.path);
    } catch (e) {
      console.error('Normalize failed:', e);
      useStore.setState({ error: String(e) });
    }
  }, [currentPage]);

  const closeMenu = () => setNoteMenuAnchor(null);

  if (!pagePath) {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <Box sx={{ textAlign: 'center' }}>
          <Typography variant="h5" sx={{ fontWeight: 600, mb: 1 }}>Stratum PKM</Typography>
          <Typography variant="body2" color="text.secondary">Select a page from the sidebar or create a new one.</Typography>
        </Box>
      </Box>
    );
  }

  if (!currentPage) {
    return <Box sx={{ p: 2 }}><Typography variant="body2" color="text.secondary">Loading...</Typography></Box>;
  }

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <Box sx={{ px: 3, py: 1.5, borderBottom: 1, borderColor: 'divider', display: 'flex', alignItems: 'center', gap: 1.5 }}>
        <Typography variant="h6" sx={{ fontWeight: 700, fontSize: '1.1rem' }}>
          {currentPage.title || currentPage.slug}
        </Typography>
        <Chip label={currentPage.path} size="small" variant="outlined" sx={{ fontSize: '0.7rem' }} />
        <Box sx={{ flex: 1 }} />
        <Button
          size="small"
          variant="outlined"
          endIcon={<ArrowDropDownIcon />}
          onClick={e => setNoteMenuAnchor(e.currentTarget)}
          disabled={reindexing}
          sx={{ textTransform: 'none', fontSize: '0.75rem' }}
        >
          {reindexing ? '…' : 'Note'}
        </Button>
        <Menu
          anchorEl={noteMenuAnchor}
          open={Boolean(noteMenuAnchor)}
          onClose={closeMenu}
        >
          <ListSubheader sx={{ lineHeight: '28px', fontSize: '0.7rem', color: 'text.disabled' }}>Actions</ListSubheader>
          <MenuItem onClick={() => { handleReindex(); closeMenu(); }} disabled={reindexing} dense>Reindex Note</MenuItem>
          <MenuItem onClick={() => { handleNormalizeFile(); closeMenu(); }} dense>Normalize File</MenuItem>
          <Divider />
          <MenuItem onClick={() => { handleDelete(); closeMenu(); }} dense sx={{ color: 'error.main' }}>Delete Page</MenuItem>
        </Menu>
      </Box>

      <Box sx={{ flex: 1, overflow: 'auto' }}>
        <OutlinerEditor key={editorKey} pagePath={currentPage.path} />
      </Box>

      <BacklinksPanel pagePath={currentPage.path} />
      <SuggestedConnectionsPanel pagePath={currentPage.path} />
    </Box>
  );
}
