import { useLocation, useNavigate } from 'react-router-dom';
import MobileNav from './MobileNav';

interface MobileLayoutProps {
  error: string | null;
  children: React.ReactNode;
}

// System-bar top inset, injected as a CSS custom property by the Android
// MainActivity (and by the index.html touch-device fallback). Resolves to 0
// on desktop and on devices without a status bar, so this is a no-op outside
// Android/iOS. Must be used via inline styles because its children are
// absolutely positioned — a padding-based container class would not offset
// them.
const SAFE_AREA_TOP = 'var(--safe-area-top, 0px)';

const TOP_BAR_HEIGHT = 48;
const BOTTOM_NAV_HEIGHT = 56;

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
  else if (location.pathname.startsWith('/ask-notes')) title = 'Ask your notes';
  else if (location.pathname.startsWith('/templates')) title = 'Templates';
  else if (location.pathname.startsWith('/flashcards')) title = 'Flashcards';
  else if (location.pathname.startsWith('/whiteboards')) title = 'Whiteboards';
  else if (location.pathname.startsWith('/plugins')) title = 'Plugins';
  else if (location.pathname.startsWith('/settings')) title = 'Settings';

  // Content area begins below the top bar (offset by the safe-area inset)
  // plus the error banner when one is shown.
  const contentTop = TOP_BAR_HEIGHT + (error ? 40 : 0);

  return (
    <div style={{ position: 'relative', height: '100vh', width: '100%', maxWidth: '100vw', backgroundColor: 'inherit', overflow: 'hidden' }}>
      <div style={{ position: 'absolute', top: SAFE_AREA_TOP, left: 0, right: 0, height: TOP_BAR_HEIGHT, display: 'flex', alignItems: 'center', paddingLeft: 8, paddingRight: 8, borderBottom: '1px solid', borderColor: 'divider', backgroundColor: 'inherit', zIndex: 1100 }}>
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
        <div style={{ position: 'absolute', top: `calc(${SAFE_AREA_TOP} + ${TOP_BAR_HEIGHT}px)`, left: 0, right: 0, zIndex: 1090 }}>
          <div style={{ padding: '8px 16px', backgroundColor: '#fdeded', color: '#5f2120', borderRadius: 0, fontSize: 14 }}>{error}</div>
        </div>
      )}

      <div style={{ position: 'absolute', top: `calc(${SAFE_AREA_TOP} + ${contentTop}px)`, bottom: BOTTOM_NAV_HEIGHT, left: 0, right: 0, overflow: 'hidden' }}>
        {children}
      </div>

      <MobileNav />
    </div>
  );
}
