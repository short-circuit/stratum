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
  const navigate = useNavigate();
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    const handleClickOutside = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    };
    window.addEventListener('keydown', handleEsc);
    window.addEventListener('mousedown', handleClickOutside);
    return () => {
      window.removeEventListener('keydown', handleEsc);
      window.removeEventListener('mousedown', handleClickOutside);
    };
  }, [onClose]);

  const handleNavigate = () => {
    navigate(`/page/${encodeURIComponent(pagePath)}`);
    onClose();
  };

  return (
    <div
      ref={ref}
      onClick={handleNavigate}
      style={{
        position: 'fixed',
        left: Math.min(position.x, window.innerWidth - 320),
        top: Math.min(position.y, window.innerHeight - 160),
        zIndex: 9999,
        maxWidth: 300,
        cursor: 'pointer',
      }}
      className="rounded-lg border border-[var(--secondary-300)] dark:border-[var(--secondary-600)] bg-[var(--primary-50)] dark:bg-[var(--primary-900)] shadow-xl p-3 text-sm"
    >
      {loading ? (
        <div className="text-xs text-[var(--secondary-400)] animate-pulse">Loading...</div>
      ) : (
        <>
          <div className="font-semibold text-[var(--secondary-700)] dark:text-[var(--secondary-200)] truncate mb-1">
            {pageTitle || pagePath}
          </div>
          <div className="text-[var(--secondary-500)] dark:text-[var(--secondary-400)] line-clamp-3 break-words">
            {content}
          </div>
        </>
      )}
    </div>
  );
}
