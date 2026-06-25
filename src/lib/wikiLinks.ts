const WIKI_LINK_RE = /\[\[([^\]]+?)(?:\|([^\]]*))?\]\]/g;

interface StyledText {
  type: 'text';
  text: string;
  styles: Record<string, boolean>;
}

interface LinkContent {
  type: 'link';
  content: StyledText[];
  href: string;
}

type InlineContent = StyledText | LinkContent;

export function parseWikiLinks(text: string): string | InlineContent[] {
  if (!text) return '';
  const result: InlineContent[] = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  WIKI_LINK_RE.lastIndex = 0;

  while ((match = WIKI_LINK_RE.exec(text)) !== null) {
    if (match.index > lastIndex) {
      result.push({
        type: 'text',
        text: text.slice(lastIndex, match.index),
        styles: {},
      });
    }

    const target = match[1].trim();
    const display = match[2]?.trim() || target;

    result.push({
      type: 'link',
      href: target,
      content: [{ type: 'text', text: display, styles: {} }],
    });

    lastIndex = match.index + match[0].length;
  }

  if (lastIndex < text.length) {
    result.push({
      type: 'text',
      text: text.slice(lastIndex),
      styles: {},
    });
  }

  if (result.length === 0) return '';
  return result;
}

export function isWikiLinkHref(href: string): boolean {
  return !href.includes('://') && !href.startsWith('#') && !href.startsWith('mailto:');
}
