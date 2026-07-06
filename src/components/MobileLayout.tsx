import { useLocation, useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
import Alert from '@mui/material/Alert';
import MobileNav from './MobileNav';

interface MobileLayoutProps {
  error: string | null;
  children: React.ReactNode;
}

export default function MobileLayout({ error, children }: MobileLayoutProps) {
  const location = useLocation();
  const navigate = useNavigate();

  const isDetailPage = location.pathname.startsWith('/page/');
  const showBack = isDetailPage;

  let title = 'Stratum';
  if (location.pathname.startsWith('/journal')) title = 'Journal';
  else if (location.pathname.startsWith('/search')) title = 'Search';
  else if (location.pathname.startsWith('/graph')) title = 'Graph';
  else if (location.pathname === '/' || location.pathname.startsWith('/page/')) title = 'Pages';
  else if (location.pathname.startsWith('/kanban')) title = 'Kanban';
  else if (location.pathname.startsWith('/query')) title = 'Query';
  else if (location.pathname.startsWith('/templates')) title = 'Templates';
  else if (location.pathname.startsWith('/flashcards')) title = 'Flashcards';
  else if (location.pathname.startsWith('/whiteboards')) title = 'Whiteboards';
  else if (location.pathname.startsWith('/settings')) title = 'Settings';

  return (
    <Box sx={{ position: 'relative', height: '100vh', width: '100%', maxWidth: '100vw', bgcolor: 'background.default', overflow: 'hidden' }}>
      {/* Header — fixed at top */}
      <Box sx={{ position: 'absolute', top: 0, left: 0, right: 0, height: 48, display: 'flex', alignItems: 'center', px: 1, borderBottom: 1, borderColor: 'divider', bgcolor: 'background.paper', zIndex: 1100 }}>
        {showBack && (
          <IconButton size="small" onClick={() => navigate(-1)} sx={{ mr: 0.5 }}>
            <ArrowBackIcon fontSize="small" />
          </IconButton>
        )}
        <Typography variant="subtitle1" sx={{ fontWeight: 600, flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {title}
        </Typography>
      </Box>

      {/* Error banner — below header */}
      {error && (
        <Box sx={{ position: 'absolute', top: 48, left: 0, right: 0, zIndex: 1090 }}>
          <Alert severity="error" sx={{ borderRadius: 0 }}>{error}</Alert>
        </Box>
      )}

      {/* Main content — between header and nav, scrollable */}
      <Box sx={{ position: 'absolute', top: error ? 88 : 48, bottom: 56, left: 0, right: 0, overflow: 'auto', display: 'flex', flexDirection: 'column' }}>
        <Box sx={{ flex: 1, minHeight: '100%' }}>
          {children}
        </Box>
      </Box>

      {/* Bottom tab navigation — fixed at very bottom */}
      <MobileNav />
    </Box>
  );
}
