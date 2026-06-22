import { useStore } from '../stores/appStore';
import { useNavigate } from 'react-router-dom';

export default function PagesHome() {
  const { pages, vault } = useStore();
  const navigate = useNavigate();

  return (
    <div className="p-6 max-w-2xl mx-auto">
      <h2 className="text-lg font-semibold mb-1">Pages</h2>
      {vault && (
        <p className="text-xs text-gray-500 mb-4">
          {vault.path} · {pages.length} page{pages.length !== 1 ? 's' : ''}
        </p>
      )}

      {pages.length > 0 ? (
        <div className="space-y-0.5">
          {pages.map(page => (
            <button
              key={page.path}
              onClick={() => navigate(`/page/${encodeURIComponent(page.path)}`)}
              className="w-full text-left flex items-center gap-3 px-3 py-2 rounded hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
            >
              <span className="text-sm flex-1 truncate">
                {page.title || page.slug}
              </span>
              <span className="text-xs text-gray-400">{page.path}</span>
              <span className="text-xs text-gray-400">{page.block_count} blocks</span>
            </button>
          ))}
        </div>
      ) : (
        <div className="text-center py-12 text-gray-400">
          <p className="text-sm">No pages yet.</p>
          <p className="text-xs mt-1">
            Create a page from the sidebar or open the journal to get started.
          </p>
        </div>
      )}
    </div>
  );
}
