import { useLocation, useNavigate } from 'react-router-dom';
import MobileNav from './MobileNav';
import ErrorAlert from './ui/ErrorAlert';
import { useStore, type AppError } from '../stores/appStore';

interface MobileLayoutProps {
  error: AppError | null;
  persistentError: AppError | null;
  children: React.ReactNode;
}

export default function MobileLayout({ error, persistentError, children }: MobileLayoutProps) {
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
  else if (location.pathname.startsWith('/ask-notes')) title = 'Ask your notes';
  else if (location.pathname.startsWith('/templates')) title = 'Templates';
  else if (location.pathname.startsWith('/flashcards')) title = 'Flashcards';
  else if (location.pathname.startsWith('/whiteboards')) title = 'Whiteboards';
  else if (location.pathname.startsWith('/plugins')) title = 'Plugins';
  else if (location.pathname.startsWith('/settings')) title = 'Settings';

  // Persistent errors take visual precedence; transient errors are dismissed
  // automatically and only shift content while visible.
  const visibleError = persistentError ?? error;
  const visible = Boolean(visibleError);

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

      {visibleError && (
        <div style={{ position: 'absolute', top: 48, left: 0, right: 0, zIndex: 1090 }}>
          <ErrorAlert
            message={visibleError.message}
            onClose={() => useStore.getState().dismissError(visibleError.id)}
          />
        </div>
      )}

      <div style={{ position: 'absolute', top: visible ? 88 : 48, bottom: 56, left: 0, right: 0, overflow: 'hidden' }}>
        {children}
      </div>

      <MobileNav />
    </div>
  );
}
