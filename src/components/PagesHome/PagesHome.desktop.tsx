import { useState } from 'react';
import Box from '@mui/material/Box';
import List from '@mui/material/List';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemText from '@mui/material/ListItemText';
import Typography from '@mui/material/Typography';
import Chip from '@mui/material/Chip';
import Button from '@mui/material/Button';
import UploadFileIcon from '@mui/icons-material/UploadFile';
import { usePagesHomeData } from './PagesHome.shared';
import ImportDialog from '../ImportDialog';
import { PageTreeList } from '../Sidebar/PageTreeGrouped';
import { useTranslation } from 'react-i18next';

export default function PagesHomeDesktop() {
  const { t } = useTranslation();
  const { pages, vault, pagesView, navigateToPage } = usePagesHomeData();
  const [importOpen, setImportOpen] = useState(false);

  return (
    <Box sx={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      <Box sx={{ flex: 1, overflow: 'auto', p: 3, maxWidth: 600, mx: 'auto' }}>
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 0.5 }}>
        <Typography variant="h5" sx={{ fontWeight: 600 }}>{t('pages_home.pages')}</Typography>
        <Button
          size="small"
          variant="outlined"
          onClick={() => setImportOpen(true)}
          startIcon={<UploadFileIcon />}
        >
          {t('importers.import_notes')}
        </Button>
      </Box>
      {vault && (
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 2 }}>
          {vault.path} · {pages.length} page{pages.length !== 1 ? 's' : ''}
        </Typography>
      )}

      {pages.length > 0 ? (
        pagesView === 'tree' ? (
          <PageTreeList pages={pages} onNavigate={navigateToPage} showBlockCount />
        ) : (
          <List disablePadding>
            {pages.map(page => (
              <ListItemButton
                key={page.path}
                onClick={() => navigateToPage(page.path)}
                sx={{ borderRadius: 1, mb: 0.25 }}
              >
                <ListItemText
                  primary={page.title || page.slug}
                  secondary={page.path}
                  slotProps={{
                    primary: { variant: 'body2', noWrap: true },
                    secondary: { variant: 'caption' },
                  }}
                />
                <Chip label={`${page.block_count}b`} size="small" variant="outlined" sx={{ ml: 1 }} />
              </ListItemButton>
            ))}
          </List>
        )
      ) : (
        <Box sx={{ textAlign: 'center', py: 6 }}>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 0.5 }}>{t('pages_home.no_pages_yet')}</Typography>
          <Typography variant="caption" color="text.disabled">
            {t('pages_home.create_a_page_from_the_sidebar_or_open_the_journal_to_get_st')}
          </Typography>
        </Box>
      )}
      </Box>
      <ImportDialog open={importOpen} onClose={() => setImportOpen(false)} />
    </Box>
  );
}
