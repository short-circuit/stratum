import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import * as api from '../lib/commands';
import type { BacklinkItem } from '../lib/types';

interface Props {
  pagePath: string;
}

export default function BacklinksPanel({ pagePath }: Props) {
  const [backlinks, setBacklinks] = useState<BacklinkItem[]>([]);
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();

  useEffect(() => {
    setLoading(true);
    api.getPageBacklinks(pagePath).then(items => {
      setBacklinks(items);
      setLoading(false);
    }).catch(() => setLoading(false));
  }, [pagePath]);

  const linked = backlinks.filter(b => b.is_linked);
  const unlinked = backlinks.filter(b => !b.is_linked);

  return (
    <div className="border-t border-gray-200 dark:border-gray-700 p-4 max-h-64 overflow-auto">
      <h3 className="text-xs font-semibold text-gray-500 uppercase mb-2">Backlinks</h3>

      {loading && <p className="text-xs text-gray-400">Loading...</p>}

      {linked.length > 0 && (
        <div className="mb-3">
          <h4 className="text-xs text-gray-400 mb-1">
            Linked References ({linked.length})
          </h4>
          <ul className="space-y-1">
            {linked.map((bl, i) => (
              <li
                key={i}
                className="text-xs p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800 cursor-pointer"
                onClick={() => navigate(`/page/${encodeURIComponent(bl.source_page)}`)}
              >
                <div className="text-gray-500">{bl.source_page}</div>
                <div className="text-gray-700 dark:text-gray-300 truncate">{bl.context}</div>
              </li>
            ))}
          </ul>
        </div>
      )}

      {unlinked.length > 0 && (
        <div>
          <h4 className="text-xs text-gray-400 mb-1">
            Unlinked Mentions ({unlinked.length})
          </h4>
          <ul className="space-y-1">
            {unlinked.map((bl, i) => (
              <li
                key={i}
                className="text-xs p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800 cursor-pointer text-gray-500"
                onClick={() => navigate(`/page/${encodeURIComponent(bl.source_page)}`)}
              >
                <div className="truncate">{bl.context}</div>
              </li>
            ))}
          </ul>
        </div>
      )}

      {!loading && backlinks.length === 0 && (
        <p className="text-xs text-gray-400">No backlinks found.</p>
      )}
    </div>
  );
}
