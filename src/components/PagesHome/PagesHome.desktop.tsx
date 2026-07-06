import Box from '@mui/material/Box';
import List from '@mui/material/List';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemText from '@mui/material/ListItemText';
import Typography from '@mui/material/Typography';
import Chip from '@mui/material/Chip';
import { usePagesHomeData } from './PagesHome.shared';

export default function PagesHomeDesktop() {
  const { pages, vault, navigateToPage } = usePagesHomeData();

  return (
    <Box sx={{ p: 3, maxWidth: 600, mx: 'auto' }}>
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
      ) : (
        <Box sx={{ textAlign: 'center', py: 6 }}>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 0.5 }}>No pages yet.</Typography>
          <Typography variant="caption" color="text.disabled">
            Create a page from the sidebar or open the journal to get started.
          </Typography>
        </Box>
      )}
    </Box>
  );
}
