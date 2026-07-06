import { useLocation, useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import ArrowBackIcon from '@mui/icons-material/ArrowBack';
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
    <div style={{ position: 'relative', height: '100vh', width: '100%', maxWidth: '100vw', backgroundColor: 'inherit', overflow: 'hidden' }}>
      <div style={{ position: 'absolute', top: 0, left: 0, right: 0, height: 48, display: 'flex', alignItems: 'center', paddingLeft: 8, paddingRight: 8, borderBottom: '1px solid', borderColor: 'divider', backgroundColor: 'inherit', zIndex: 1100 }}>
        {showBack && (
          <button onClick={() => navigate(-1)} style={{ marginRight: 4, background: 'none', border: 'none', cursor: 'pointer', padding: 4 }}>
            ←
          </button>
        )}
        <span style={{ fontWeight: 600, flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', fontSize: 16 }}>
          {title}
        </span>
      </div>

      {error && (
        <div style={{ position: 'absolute', top: 48, left: 0, right: 0, zIndex: 1090 }}>
          <div style={{ padding: '8px 16px', backgroundColor: '#fdeded', color: '#5f2120', borderRadius: 0, fontSize: 14 }}>{error}</div>
        </div>
      )}

      <div style={{ position: 'absolute', top: error ? 88 : 48, bottom: 56, left: 0, right: 0, overflow: 'auto' }}>
        {children}
      </div>

      <MobileNav />
    </div>
  );
}
