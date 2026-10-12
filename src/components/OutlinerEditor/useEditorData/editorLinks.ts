import {
  isWikiLinkHref,
  extractWikiLinkTarget,
  isTagHref,
  extractTagTarget,
  isAssetHref,
} from '../../../lib/wikiLinks';
import * as api from '../../../lib/commands';

interface EditorLinksHandlers {
  onNavigate: (path: string) => void;
  onDeadLink: (target: string, pos: { x: number; y: number }) => void;
}

export function buildEditorLinks(handlers: EditorLinksHandlers) {
  return {
    isValidLink: (href: string) => {
      if (isAssetHref(href)) return true;
      if (href.startsWith('stratum:') || href.startsWith('stratum-tag:')) return true;
      if (!href) return true;
      return /^(?:https?|ftp|ftps|mailto|tel|callto|sms|cid|xmpp):/i.test(href);
    },
    onClick: (event: { target: EventTarget | null; ctrlKey: boolean; metaKey: boolean }) => {
      const a = (event.target as HTMLElement).closest?.('a');
      if (!a) return false;
      const href = a.getAttribute('href');
      if (!href) return false;
      if (isAssetHref(href)) return true;
      if (isTagHref(href)) {
        if (!event.ctrlKey && !event.metaKey) return true;
        const tagName = extractTagTarget(href);
        handlers.onNavigate('/search?q=' + encodeURIComponent('#' + tagName));
        return true;
      }
      if (isWikiLinkHref(href)) {
        const rect = a.getBoundingClientRect();
        const pos = { x: rect.left, y: rect.bottom };
        let target = extractWikiLinkTarget(href);
        target = target.replace(/[[\]]/g, '').trim().toLowerCase();
        api.resolveLinkTarget(target).then(resolved => {
          if (resolved.page_path) {
            handlers.onNavigate('/page/' + encodeURIComponent(resolved.page_path));
          } else {
            handlers.onDeadLink(target, pos);
          }
        });
        return true;
      }
      return false;
    },
  };
}
