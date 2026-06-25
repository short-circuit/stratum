import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import * as api from '../lib/commands';
import type { BacklinkItem } from '../lib/types';
import { useCtrlHeld } from '../lib/useCtrlHeld';
import LinkPreviewPopup from './LinkPreviewPopup';

interface Props {
  pagePath: string;
}

interface PreviewState {
  content: string;
  pageTitle: string | null;
  pagePath: string;
  position: { x: number; y: number };
  loading: boolean;
}

export default function BacklinksPanel({ pagePath }: Props) {
  const [backlinks, setBacklinks] = useState<BacklinkItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [collapsed, setCollapsed] = useState(true);
  const [preview, setPreview] = useState<PreviewState | null>(null);
  const hoverTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const navigate = useNavigate();
  const { ctrlHeld } = useCtrlHeld();

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setLoading(true);
    api.getPageBacklinks(pagePath).then(items => {
      setBacklinks(items);
      setLoading(false);
    }).catch(() => setLoading(false));
  }, [pagePath]);

  const handleMouseEnter = (bl: BacklinkItem, e: React.MouseEvent) => {
    if (!ctrlHeld.current) return;

    if (hoverTimer.current) clearTimeout(hoverTimer.current);
    hoverTimer.current = setTimeout(async () => {
      if (!ctrlHeld.current) return;

      setPreview({
        content: '',
        pageTitle: null,
        pagePath: '',
        position: { x: e.clientX + 10, y: e.clientY + 10 },
        loading: true,
      });

      try {
        const targetSlug = bl.source_page
          .replace(/\.md$/, '')
          .split('/')
          .pop() || bl.source_page;
        const resolved = await api.resolveLinkTarget(targetSlug);
        if (!resolved.page_path) {
          setPreview(null);
          return;
        }
        if (!ctrlHeld.current) { setPreview(null); return; }

        // For backlinks, the current page is the target, and source_page is the linking page
        // But we want context from the linking page (source_page) about the current page
        // Actually for backlinks: we're hovering over a backlink item showing source_page
        // The preview should show the context from source_page that links to current page
        // Let me re-read: the backlink already shows context in the panel.
        // For the popup, we fetch the actual block content from the source page
        // showing what links back to the current page.
        // Since backlinks already have `bl.context`, we can just use that for the content
        // and show the source page title.
        const ctx = await api.getBacklinkContext(bl.source_page, pagePath);
        if (!ctrlHeld.current) { setPreview(null); return; }

        setPreview({
          content: ctx?.content || bl.context || '(empty)',
          pageTitle: ctx?.page_title || resolved.title || targetSlug,
          pagePath: bl.source_page,
          position: { x: e.clientX + 10, y: e.clientY + 10 },
          loading: false,
        });
      } catch {
        setPreview(null);
      }
    }, 200);
  };

  const handleMouseLeave = () => {
    if (hoverTimer.current) {
      clearTimeout(hoverTimer.current);
      hoverTimer.current = null;
    }
    setPreview(null);
  };

  const linked = backlinks.filter(b => b.is_linked);
  const unlinked = backlinks.filter(b => !b.is_linked);

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
                  <li
                    key={i}
                    className="text-xs p-1.5 rounded hover:bg-[var(--secondary-100)] dark:hover:bg-[var(--secondary-800)] cursor-pointer"
                    onClick={() => navigate(`/page/${encodeURIComponent(bl.source_page)}`)}
                    onMouseEnter={(e) => handleMouseEnter(bl, e)}
                    onMouseLeave={handleMouseLeave}
                  >
                    <div className="text-[var(--secondary-500)]">{bl.source_page}</div>
                    <div className="text-[var(--secondary-700)] dark:text-[var(--secondary-300)] truncate">{bl.context}</div>
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
                  <li
                    key={i}
                    className="text-xs p-1.5 rounded hover:bg-[var(--secondary-100)] dark:hover:bg-[var(--secondary-800)] cursor-pointer text-[var(--secondary-500)]"
                    onClick={() => navigate(`/page/${encodeURIComponent(bl.source_page)}`)}
                    onMouseEnter={(e) => handleMouseEnter(bl, e)}
                    onMouseLeave={handleMouseLeave}
                  >
                    <div className="truncate">{bl.context}</div>
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
