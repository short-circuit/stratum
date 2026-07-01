import { useEffect, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import * as api from '../lib/commands';
import type { BacklinkItem } from '../lib/types';
import { useCtrlHeld } from '../lib/useCtrlHeld';
import LinkPreviewPopup from './LinkPreviewPopup';

interface Props {
  pagePath: string;
}

export default function BacklinksPanel({ pagePath }: Props) {
  const [backlinks, setBacklinks] = useState<BacklinkItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [collapsed, setCollapsed] = useState(true);
  const ctrlHeld = useCtrlHeld();
  const hoverTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [preview, setPreview] = useState<{
    content: string;
    pageTitle: string | null;
    pagePath: string;
    position: { x: number; y: number };
    loading: boolean;
  } | null>(null);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setLoading(true);
    api.getPageBacklinks(pagePath).then(items => {
      setBacklinks(items);
      setLoading(false);
    }).catch(() => setLoading(false));
  }, [pagePath]);

  const linked = backlinks.filter(b => b.is_linked);
  const unlinked = backlinks.filter(b => !b.is_linked);

  const handleMouseEnter = (item: BacklinkItem, e: React.MouseEvent) => {
    hoverTimer.current = setTimeout(async () => {
      if (!ctrlHeld.current) return;
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
      setPreview({
        content: item.context,
        pageTitle: null,
        pagePath: item.source_page,
        position: { x: rect.left, y: rect.bottom + 4 },
        loading: true,
      });
      try {
        const page = await api.openPage(item.source_page);
        if (ctrlHeld.current) {
          setPreview(prev => prev ? { ...prev, pageTitle: page.title || null, loading: false } : null);
        }
      } catch { setPreview(null); }
    }, 200);
  };

  const handleMouseLeave = () => {
    if (hoverTimer.current) { clearTimeout(hoverTimer.current); hoverTimer.current = null; }
  };

  // Dismiss preview when Ctrl is released
  useEffect(() => {
    const handleKeyUp = (e: KeyboardEvent) => {
      if (e.key === 'Control' || e.key === 'Meta') setPreview(null);
    };
    window.addEventListener('keyup', handleKeyUp);
    return () => window.removeEventListener('keyup', handleKeyUp);
  }, []);

  return (
    <div className="border-t border-[var(--secondary-200)] dark:border-[var(--secondary-700)]">
      <button
        onClick={() => setCollapsed(!collapsed)}
        className="w-full flex items-center justify-between px-4 py-2 text-xs font-semibold text-[var(--secondary-500)] uppercase hover:bg-[var(--secondary-50)] dark:hover:bg-[var(--secondary-800)] transition-colors"
      >
        <span>Backlinks ({backlinks.length})</span>
        <span className={`transform transition-transform ${collapsed ? '' : 'rotate-90'}`}>&#9654;</span>
      </button>

      {!collapsed && (
        <div className="px-4 pb-3 max-h-64 overflow-auto">
          {loading && <p className="text-xs text-[var(--secondary-400)]">Loading...</p>}

          {linked.length > 0 && (
            <div className="mb-3">
              <h4 className="text-xs text-[var(--secondary-400)] mb-1">
                Linked References ({linked.length})
              </h4>
              <ul className="space-y-1">
                {linked.map((bl, i) => (
                  <li key={i}>
                    <Link
                      to={'/page/' + encodeURIComponent(bl.source_page)}
                      className="block text-xs p-1.5 rounded hover:bg-[var(--secondary-100)] dark:hover:bg-[var(--secondary-800)]"
                      onMouseEnter={(e) => handleMouseEnter(bl, e)}
                      onMouseLeave={handleMouseLeave}
                    >
                      <div className="text-[var(--secondary-500)]">{bl.source_page}</div>
                      <div className="text-[var(--secondary-700)] dark:text-[var(--secondary-300)] truncate">{bl.context}</div>
                    </Link>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {unlinked.length > 0 && (
            <div>
              <h4 className="text-xs text-[var(--secondary-400)] mb-1">
                Unlinked Mentions ({unlinked.length})
              </h4>
              <ul className="space-y-1">
                  {unlinked.map((bl, i) => (
                  <li key={i}>
                    <Link
                      to={'/page/' + encodeURIComponent(bl.source_page)}
                      className="block text-xs p-1.5 rounded hover:bg-[var(--secondary-100)] dark:hover:bg-[var(--secondary-800)]"
                      onMouseEnter={(e) => handleMouseEnter(bl, e)}
                      onMouseLeave={handleMouseLeave}
                    >
                      <div className="text-[var(--secondary-500)]">{bl.source_page}</div>
                      <div className="text-[var(--secondary-600)] dark:text-[var(--secondary-400)] truncate">{bl.context}</div>
                    </Link>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {!loading && backlinks.length === 0 && (
            <p className="text-xs text-[var(--secondary-400)]">No backlinks found.</p>
          )}
        </div>
      )}

      {preview && (
        <LinkPreviewPopup
          content={preview.content}
          pageTitle={preview.pageTitle}
          pagePath={preview.pagePath}
          position={preview.position}
          loading={preview.loading}
          onClose={() => setPreview(null)}
        />
      )}
    </div>
  );
}
