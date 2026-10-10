import Box from '@mui/material/Box';
import List from '@mui/material/List';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemText from '@mui/material/ListItemText';
import Typography from '@mui/material/Typography';
import Chip from '@mui/material/Chip';
import { usePagesHomeData } from './PagesHome.shared';
import { useTranslation } from 'react-i18next';

export default function PagesHomeMobile() {
  const { t } = useTranslation();
  const { pages, vault, navigateToPage } = usePagesHomeData();

  return (
    <Box sx={{ p: 2 }}>
      <Typography variant="h5" sx={{ fontWeight: 600, mb: 0.5 }}>Pages</Typography>
      {vault && (
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 2 }}>
          {vault.path} · {pages.length} page{pages.length !== 1 ? 's' : ''}
        </Typography>
      )}

      {pages.length > 0 ? (
        <List disablePadding>
          {pages.map(page => (
            <ListItemButton
              key={page.path}
              onClick={() => navigateToPage(page.path)}
              sx={{
                borderRadius: 1,
                mb: 1,
                border: 1,
                borderColor: 'divider',
                width: '100%',
              }}
            >
              <ListItemText
                primary={
                  <Typography variant="body1" sx={{ fontWeight: 600 }}>
                    {page.title || page.slug}
                  </Typography>
                }
                secondary={page.path}
                slotProps={{
                  secondary: { variant: 'caption' },
                }}
              />
              <Chip label={`${page.block_count}b`} size="small" variant="outlined" sx={{ ml: 1, flexShrink: 0 }} />
            </ListItemButton>
          ))}
        </List>
      ) : (
        <Box sx={{ textAlign: 'center', py: 6 }}>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 0.5 }}>{t('pages_home.no_pages_yet')}</Typography>
          <Typography variant="caption" color="text.disabled">
            {t('pages_home.create_a_page_from_the_sidebar_or_open_the_journal_to_get_st')}
          </Typography>
        </Box>
      )}
    </Box>
  );
}
