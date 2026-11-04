/**
 * useWikiLinkInteractions — dead-link detection, Ctrl-hover preview, and
 * hover/click event delegation for the Outliner editor.
 *
 * Extracted verbatim from useEditorData/index.ts during the t_dc58c24c
 * sizing-gate decomposition. Owns the marker-highlighting of unresolved
 * [[wiki-links]], the deferred Ctrl/Cmd-hover preview popup, and the delegated
 * mouse handlers that prevent browser navigation and drive the preview. No JSX.
 *
 * @module OutlinerEditor/useEditorData/wikiLinks
 */

import { useCallback, useEffect, useRef } from 'react';
import type { Dispatch, MutableRefObject, RefObject, SetStateAction } from 'react';
import i18n from '../../../i18n';
import * as api from '../../../lib/commands';
import { isWikiLinkHref, extractWikiLinkTarget } from '../../../lib/wikiLinks';
import { useStore } from '../../../stores/appStore';
import type { PreviewState } from './types';

interface UseWikiLinkInteractionsArgs {
  containerRef: RefObject<HTMLDivElement | null>;
  status: string;
  pagePath: string;
  ctrlHeld: MutableRefObject<boolean>;
  setPreview: Dispatch<SetStateAction<PreviewState>>;
}

/**
 * Dead-link detection — highlight [[wiki-links]] to non-existent pages, plus
 * the Ctrl/Cmd-hover preview popup and the delegated hover/click handlers.
 */
export function useWikiLinkInteractions({
  containerRef,
  status,
  pagePath,
  ctrlHeld,
  setPreview,
}: UseWikiLinkInteractionsArgs) {
  const hoverTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const markDeadLinks = useCallback((root: HTMLElement) => {
    const anchors = root.querySelectorAll<HTMLAnchorElement>(
      'a[data-inline-content-type="link"][href^="stratum:"]',
    );
    if (!anchors.length) return;
    const { getState } = useStore;
    const slugs = new Set(
      getState()
        .pages.map((p) =>
          (p.slug || p.path.replace(/\.md$/i, '')).toLowerCase(),
        ),
    );
    for (const a of anchors) {
      const href = a.getAttribute('href');
      if (!href || href.startsWith('stratum-tag:')) continue;
      const target = extractWikiLinkTarget(href)
        .replace(/[[\]]/g, '')
        .trim()
        .toLowerCase();
      if (!slugs.has(target)) {
        a.style.color = '#d97706';
        a.style.textDecoration = 'underline dashed';
      } else {
        a.style.color = '';
        a.style.textDecoration = '';
      }
    }
  }, []);

  useEffect(() => {
    if (status !== 'ready') return;
    const el = containerRef.current;
    if (!el) return;
    const t = setTimeout(() => markDeadLinks(el), 1000);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- containerRef is a stable ref drilled in from the parent composer.
  }, [status, markDeadLinks]);

  const showPreview = useCallback(
    (href: string, x: number, y: number) => {
      if (!ctrlHeld.current) return;
      const target = extractWikiLinkTarget(href);
      if (hoverTimer.current) clearTimeout(hoverTimer.current);
      hoverTimer.current = setTimeout(async () => {
        if (!ctrlHeld.current) {
          setPreview(null);
          return;
        }
        setPreview({
          content: '',
          pageTitle: null,
          pagePath: '',
          position: { x, y },
          loading: true,
        });
        try {
          const resolved = await api.resolveLinkTarget(target);
          if (!resolved.page_path || !ctrlHeld.current) {
            setPreview(null);
            return;
          }
          const ctx = await api.getBacklinkContext(
            resolved.page_path,
            pagePath,
          );
          if (!ctrlHeld.current) {
            setPreview(null);
            return;
          }
          setPreview({
            content: ctx?.content || i18n.t('editor._empty'),
            pageTitle:
              ctx?.page_title || resolved.title || resolved.slug || target,
            pagePath: resolved.page_path,
            position: { x, y },
            loading: false,
          });
        } catch {
          setPreview(null);
        }
      }, 200);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps -- setPreview is a parent-owned state setter (stable by definition).
    [ctrlHeld, pagePath],
  );

  const dismissPreview = useCallback(() => {
    if (hoverTimer.current) {
      clearTimeout(hoverTimer.current);
      hoverTimer.current = null;
    }
    setPreview(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- setPreview is a parent-owned state setter (stable by definition).
  }, []);

  // Wiki-link hover/click event delegation
  useEffect(() => {
    const el = containerRef.current;
    if (!el || status !== 'ready') return;

    let currentHovered = '';

    const handleMouseOver = (e: MouseEvent) => {
      const a = (e.target as HTMLElement).closest?.('a');
      if (!a) return;
      const href = a.getAttribute('href');
      if (!href || !isWikiLinkHref(href)) return;
      if (href === currentHovered) return;
      currentHovered = href;
      const rect = a.getBoundingClientRect();
      showPreview(href, rect.left, rect.bottom + 4);
    };

    const handleMouseOut = (e: MouseEvent) => {
      const a = (e.target as HTMLElement).closest?.('a');
      if (!a) {
        currentHovered = '';
        dismissPreview();
        return;
      }
      const href = a.getAttribute('href');
      if (!href || !isWikiLinkHref(href)) {
        currentHovered = '';
        dismissPreview();
        return;
      }
      currentHovered = '';
      dismissPreview();
    };

    const handleLinkPrevent = (e: MouseEvent) => {
      const a = (e.target as HTMLElement).closest?.('a');
      if (!a) return;
      const href = a.getAttribute('href');
      if (!href || !isWikiLinkHref(href)) return;
      e.preventDefault();
    };

    el.addEventListener('mouseover', handleMouseOver);
    el.addEventListener('mouseout', handleMouseOut);
    el.addEventListener('mousedown', handleLinkPrevent, true);
    el.addEventListener('click', handleLinkPrevent, true);
    return () => {
      el.removeEventListener('mouseover', handleMouseOver);
      el.removeEventListener('mouseout', handleMouseOut);
      el.removeEventListener('mousedown', handleLinkPrevent, true);
      el.removeEventListener('click', handleLinkPrevent, true);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- containerRef is a stable ref drilled in from the parent composer.
  }, [status, showPreview, dismissPreview]);

  // Poll Ctrl/Meta held state and dismiss preview when released
  useEffect(() => {
    const check = () => {
      if (!ctrlHeld.current) dismissPreview();
    };
    const interval = setInterval(check, 100);
    return () => clearInterval(interval);
  }, [ctrlHeld, dismissPreview]);

  return { markDeadLinks, showPreview, dismissPreview };
}
