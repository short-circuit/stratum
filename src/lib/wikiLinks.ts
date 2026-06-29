const PREFIX = 'stratum:';

export function isWikiLinkHref(href: string): boolean {
  return href.startsWith(PREFIX);
}

export function extractWikiLinkTarget(href: string): string {
  return href.slice(PREFIX.length);
}

export interface TextItem {
  type: 'text';
  text: string;
  styles: Record<string, boolean>;
}

export interface LinkItem {
  type: 'link';
  href: string;
  content: TextItem[];
}

export type InlineItem = TextItem | LinkItem;

// Normalize corrupted/legacy content to clean [[Target]] format
export function normalizeContent(text: string): string {
  if (!text) return '';
  let r = text;
  // [text](http://stratum.internal/target) → [[target|text]]
  r = r.replace(/\[([^\]]*)\]\(http:\/\/stratum\.internal\/([^)]+)\)/g, (_, display, encoded) => {
    const target = decodeURIComponent(encoded).trim();
    const d = display.trim();
    if (d.toLowerCase() === target.toLowerCase()) return `[[${target}]]`;
    return `[[${target}|${d}]]`;
  });
  // [text](stratum:target) → [[target|text]]
  r = r.replace(/\[([^\]]*)\]\(stratum:([^)]+)\)/g, (_, display, target) => {
    const t = target.trim();
    const d = display.trim();
    if (d.toLowerCase() === t.toLowerCase()) return `[[${t}]]`;
    return `[[${t}|${d}]]`;
  });
  // [[[[[text]]]]] → [[text]]
  // eslint-disable-next-line no-useless-escape
  r = r.replace(/\[{2,}([^\[\]]+)\]{2,}/g, '[[$1]]');
  // [[[text]]]() → [[text]]  (orphaned () from corrupted markdown links)
  // eslint-disable-next-line no-useless-escape
  r = r.replace(/\[{2,}([^\[\]]+)\]{2,}\(\)/g, '[[$1]]');
  return r;
}

// Parse content string to BlockNote inline items.
// Handles **bold**, *italic*, ~~strikethrough~~, `code`, [[wikilinks]].
export function parseContentToInlineItems(text: string): InlineItem[] {
  if (!text) return [{ type: 'text', text: '', styles: {} }];
  const items: InlineItem[] = [];
  let pos = 0;
  const RE = /(\*\*(.+?)\*\*)|(\*(.+?)\*)|(~~(.+?)~~)|(`(.+?)`)|(\[\[([^\]]+?)(?:\|([^\]]*))?\]\])/g;
  let m: RegExpExecArray | null;
  while ((m = RE.exec(text)) !== null) {
    if (m.index > pos) {
      items.push({ type: 'text', text: text.slice(pos, m.index), styles: {} });
    }
    if (m[2] !== undefined) {
      items.push({ type: 'text', text: m[2], styles: { bold: true } });
    } else if (m[4] !== undefined) {
      items.push({ type: 'text', text: m[4], styles: { italic: true } });
    } else if (m[6] !== undefined) {
      items.push({ type: 'text', text: m[6], styles: { strike: true } });
    } else if (m[8] !== undefined) {
      items.push({ type: 'text', text: m[8], styles: { code: true } });
    } else if (m[9] !== undefined) {
      const target = m[9].trim();
      const display = (m[11] || m[9]).trim();
      items.push({
        type: 'link',
        href: PREFIX + target,
        content: [{ type: 'text', text: display, styles: {} }],
      });
    }
    pos = RE.lastIndex;
  }
  if (pos < text.length) {
    items.push({ type: 'text', text: text.slice(pos), styles: {} });
  }
  return items;
}

// Serialize BlockNote inline items back to content string.
export function inlineItemsToContent(items: InlineItem[]): string {
  let out = '';
  for (const item of items) {
    if (item.type === 'link') {
      if (isWikiLinkHref(item.href || '')) {
        const target = extractWikiLinkTarget(item.href);
        const display = item.content?.map((c: TextItem) => c?.text || '').join('') || target;
        if (display === target || display.replace(/-/g, ' ') === target.replace(/-/g, ' ')) {
          out += `[[${target}]]`;
        } else {
          out += `[[${target}|${display}]]`;
        }
      } else {
        const display = item.content?.map((c: TextItem) => c?.text || '').join('') || item.href;
        out += `[${display}](${item.href})`;
      }
    } else {
      const s = item.styles || {};
      let t = item.text || '';
      if (t) {
        if (s.code) t = '`' + t + '`';
        if (s.strike) t = '~~' + t + '~~';
        if (s.bold) t = '**' + t + '**';
        if (s.italic) t = '*' + t + '*';
      }
      out += t;
    }
  }
  return out;
}
