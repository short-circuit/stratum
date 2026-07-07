/**
 * Shared logic hook for PageView.
 * Extracts pagePath from URL params, manages editorKey/reset,
 * and exposes reindex/delete handlers used by both desktop and mobile variants.
 */

import { useEffect, useState, useCallback } from 'react';
import { useParams } from 'react-router-dom';
import { useStore, type AppState } from '../../stores/appStore';
import * as api from '../../lib/commands';

export interface PageViewState {
  /** Decoded page path from URL params, or null on the landing route */
  pagePath: string | null;
  /** Current page data from the store (null while loading or on landing) */
  currentPage: AppState['currentPage'];
  /** Incrementing key to force OutlinerEditor remount on page change */
  editorKey: number;
  /** True while a reindex operation is in flight */
  reindexing: boolean;
  /** Trigger a full reindex of the current page */
  handleReindex: () => Promise<void>;
  /** Delete the current page after user confirmation */
  handleDelete: () => Promise<void>;
}

/**
 * Hook that centralises the common PageView orchestration:
 * URL param → store.openPage → editor key bump → reindex / delete.
 */
export function usePageView(): PageViewState {
  const { pagePath: rawPath } = useParams<{ pagePath: string }>();
  const { currentPage, openPage, deletePage } = useStore();
  const [editorKey, setEditorKey] = useState(0);
  const [reindexing, setReindexing] = useState(false);

  const pagePath = rawPath ? decodeURIComponent(rawPath) : null;

  useEffect(() => {
    if (pagePath) {
      openPage(pagePath);
      // Bump key so OutlinerEditor remounts with fresh content
      setEditorKey(k => k + 1);
    }
  }, [pagePath, openPage]);

  const handleReindex = useCallback(async () => {
    if (!currentPage) return;
    setReindexing(true);
    try {
      await api.reindexPage(currentPage.path);
      setEditorKey(k => k + 1);
    } catch (e) {
      console.error('Reindex failed:', e);
      useStore.setState({ error: String(e) });
    } finally {
      setReindexing(false);
    }
  }, [currentPage]);

  const handleDelete = useCallback(async () => {
    if (!currentPage) return;
    if (!confirm(`Delete "${currentPage.title || currentPage.slug}"?`)) return;
    try {
      await deletePage(currentPage.path);
    } catch (e) {
      console.error('Delete failed:', e);
    }
  }, [currentPage, deletePage]);

  return {
    pagePath,
    currentPage,
    editorKey,
    reindexing,
    handleReindex,
    handleDelete,
  };
}
