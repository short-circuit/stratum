import { useEffect, useRef } from 'react';
import { useNavigate } from 'react-router-dom';

interface Props {
  content: string;
  pageTitle: string | null;
  pagePath: string;
  position: { x: number; y: number };
  loading: boolean;
  onClose: () => void;
}

export default function LinkPreviewPopup({ content, pageTitle, pagePath, position, loading, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const navigate = useNavigate();

  useEffect(() => {
    const handler = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose(); };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [onClose]);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    document.addEventListener('mousedown', handler, true);
    return () => document.removeEventListener('mousedown', handler, true);
  }, [onClose]);

  const handleClick = () => {
    navigate(`/page/${encodeURIComponent(pagePath)}`);
    onClose();
  };

  return (
    <div
      ref={ref}
      style={{ left: position.x, top: position.y, zIndex: 9999 }}
      className="fixed bg-white dark:bg-[var(--secondary-800)] border border-[var(--secondary-200)] dark:border-[var(--secondary-700)] rounded-lg shadow-xl p-3 max-w-xs"
    >
      {loading ? (
        <div className="text-xs text-[var(--secondary-400)]">Loading...</div>
      ) : (
        <>
          <div
            className="text-sm font-semibold text-blue-600 dark:text-blue-400 cursor-pointer hover:underline truncate mb-1"
            onClick={handleClick}
          >
            {pageTitle || pagePath}
          </div>
          <div className="text-xs text-[var(--secondary-600)] dark:text-[var(--secondary-300)] line-clamp-3">
            {content}
          </div>
          <div className="text-[10px] text-[var(--secondary-400)] mt-1">Ctrl+click to navigate</div>
        </>
      )}
    </div>
  );
}
