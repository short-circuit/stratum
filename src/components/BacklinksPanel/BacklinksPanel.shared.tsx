//! Shared logic for BacklinksPanel — data fetching, filtering, preview state.

import { useEffect, useState, useCallback } from 'react';
import * as api from '../../lib/commands';
import type { BacklinkItem } from '../../lib/types';

export interface BacklinksPanelProps {
  pagePath: string;
}

export interface PreviewData {
  content: string;
  pageTitle: string | null;
  pagePath: string;
  anchorEl: HTMLElement | null;
  loading: boolean;
}

/**
 * Fetches backlinks for the given page and returns linked/unlinked subsets.
 */
export function useBacklinksData(pagePath: string) {
  const [backlinks, setBacklinks] = useState<BacklinkItem[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    if (!cancelled) setLoading(true);
    api.getPageBacklinks(pagePath)
      .then(items => { if (!cancelled) { setBacklinks(items); setLoading(false); } })
      .catch(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [pagePath]);

  const linked = backlinks.filter(b => b.is_linked);
  const unlinked = backlinks.filter(b => !b.is_linked);

  return { backlinks, loading, linked, unlinked };
}

/**
 * Manages preview popup/dialog state.
 * Shared by desktop (Popover) and mobile (Dialog) renderers.
 */
export function usePreview() {
  const [preview, setPreview] = useState<PreviewData | null>(null);

  const showPreview = useCallback(async (item: BacklinkItem, anchorEl?: HTMLElement | null) => {
    setPreview({
      content: item.context,
      pageTitle: null,
      pagePath: item.source_page,
      anchorEl: anchorEl ?? null,
      loading: true,
    });
    try {
      const page = await api.openPage(item.source_page);
      setPreview(prev =>
        prev ? { ...prev, pageTitle: page.title || null, loading: false } : null,
      );
    } catch {
      setPreview(null);
    }
  }, []);

  const dismissPreview = useCallback(() => setPreview(null), []);

  // Dismiss preview when Ctrl or Meta is released.
  useEffect(() => {
    const handleKeyUp = (e: KeyboardEvent) => {
      if (e.key === 'Control' || e.key === 'Meta') setPreview(null);
    };
    window.addEventListener('keyup', handleKeyUp);
    return () => window.removeEventListener('keyup', handleKeyUp);
  }, []);

  return { preview, showPreview, dismissPreview };
}
