//! Shared logic for BacklinksPanel — data fetching, filtering, preview state.
//!
//! The preview popup (Ctrl+hover on desktop, long-press on mobile) fetches the
//! exact backlinked snippet via the `get_backlink_snippet` Tauri command
//! (`note_id` = `BacklinkItem.source_page`, `backlink_ref` =
//! `BacklinkItem.source_id`, the anchor block). Missing notes/anchors surface
//! as an error state in the popup rather than a silent no-op.

import { useEffect, useState, useCallback } from 'react';
import * as api from '../../lib/commands';
import type { BacklinkItem } from '../../lib/types';

export interface BacklinksPanelProps {
  pagePath: string;
}

export interface PreviewData {
  /** Vault-relative path of the note containing the anchor (the backlink source). */
  noteId: string;
  noteTitle: string;
  anchorContent: string;
  context: string[];
  anchorEl: HTMLElement | null;
  loading: boolean;
  error: boolean;
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
 * Manages the backlink-snippet preview popup/dialog state.
 *
 * `showPreview` immediately renders the popup in its loading state, then
 * fetches the snippet. 404-style responses (missing note / missing anchor)
 * leave the popup in an error state; the caller's normal close paths (Escape,
 * Ctrl release, leave, backdrop) dismiss it.
 *
 * Shared by desktop (Popover) and mobile (Dialog) renderers.
 */
export function usePreview() {
  const [preview, setPreview] = useState<PreviewData | null>(null);

  const showPreview = useCallback((item: BacklinkItem, anchorEl?: HTMLElement | null) => {
    setPreview({
      noteId: item.source_page,
      noteTitle: '',
      anchorContent: '',
      context: [],
      anchorEl: anchorEl ?? null,
      loading: true,
      error: false,
    });
    // Fetch the snippet in the background; resolution order guards against
    // stale responses if the user hovers two backlinks quickly.
    api.getBacklinkSnippet(item.source_page, item.source_id)
      .then(snippet => {
        setPreview(prev => {
          if (!prev || prev.noteId !== item.source_page) return prev;
          return {
            ...prev,
            noteTitle: snippet.note_title,
            anchorContent: snippet.anchor_content || '',
            context: snippet.context ?? [],
            loading: false,
            error: false,
          };
        });
      })
      .catch(() => {
        setPreview(prev => {
          if (!prev || prev.noteId !== item.source_page) return prev;
          return { ...prev, loading: false, error: true };
        });
      });
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
