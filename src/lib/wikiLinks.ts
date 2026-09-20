const PREFIX = 'stratum:';
const TAG_PREFIX = 'stratum-tag:';

export function isWikiLinkHref(href: string): boolean {
  return href.startsWith(PREFIX) && !href.startsWith(TAG_PREFIX);
}

export function extractWikiLinkTarget(href: string): string {
  return href.slice(PREFIX.length);
}

export function isTagHref(href: string): boolean {
  return href.startsWith(TAG_PREFIX);
}

export function extractTagTarget(href: string): string {
  return href.slice(TAG_PREFIX.length);
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
  // Collapse any number of doubled brackets iteratively until stable:
  //   [[[[[text]]]]] → [[text]]
  //   [[[[Target|display]]|display]] → [[Target|display]]   (piped double-wrap)
  //   [[[text]]]() → [[text]]          (orphaned () from corrupted markdown links)
  // A single pass is insufficient because pipes can leave a trailing wrapper
  // ([[A|B]]|B]]) that earlier collapses cannot see.
  let prev: string;
  let guard = 0;
  do {
    prev = r;
    // eslint-disable-next-line no-useless-escape
    r = r.replace(/\[{2,}([^\[\]]+)\]{2,}/g, '[[$1]]');
    // Fix piped double-wrap where the outer trailing segment repeats the display.
    r = r.replace(
      // eslint-disable-next-line no-useless-escape
      /\[\[([^\[\]]+)\|([^\[\]]*)\]\](?:\|([^\[\]]*))?\]\]/g,
      (full, target, display, tail) => {
        if (tail === undefined) return `[[${target}|${display}]]`;
        if (tail === display) return `[[${target}|${display}]]`;
        return full;
      },
    );
    // Orphaned () from corrupted markdown links.
    // eslint-disable-next-line no-useless-escape
    r = r.replace(/\[{2,}([^\[\]]+)\]{2,}\(\)/g, '[[$1]]');
    if (++guard > 10) break;
  } while (r !== prev);
  return r;
}

// Parse content string to BlockNote inline items.
// Handles **bold**, *italic*, ~~strikethrough~~, `code`, [[wikilinks]], #tags.
export function parseContentToInlineItems(text: string): InlineItem[] {
  if (!text) return [{ type: 'text', text: '', styles: {} }];
  const items: InlineItem[] = [];
  let pos = 0;
  // Note: the wiki-link target capture excludes `[` as well as `]`. Legacy
  // corruption can carry extra brackets inside the target (e.g. `[[[[T]]]]`).
  // If we allowed `[` in the target, the first match would absorb the leading
  // bracket pair into the target and re-serialize into a mangled piped link
  // (`[[T|[[T]]]]`). Excluding it makes such input fall through to the text
  // branch, which round-trips byte-stable (the app normalizes it at load).
  // Note: `\[`/`\]` inside the character class are intentionally kept — they
  // exclude literal brackets from the wiki-link target capture (see above).
  // eslint-disable-next-line no-useless-escape -- brackets must be excluded literally
  const RE = /(\*\*(.+?)\*\*)|(\*(.+?)\*)|(~~(.+?)~~)|(`(.+?)`)|(\[\[([^\[\]]+?)(?:\|([^\[\]]*))?\]\])|(#([a-zA-Z][a-zA-Z0-9_\-/]*))/g;
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
      const target = (m[10] ?? m[9]).trim();
      const display = (m[11] || m[10] || m[9]).trim();
      items.push({
        type: 'link',
        href: PREFIX + target,
        content: [{ type: 'text', text: display, styles: {} }],
      });
    } else if (m[12] !== undefined) {
      // Validate #tag is at a word boundary (preceded by whitespace, (, or start of string)
      const prevChar = m.index > 0 ? text[m.index - 1] : null;
      if (prevChar && !/\s/.test(prevChar) && prevChar !== '(') {
        items.push({ type: 'text', text: m[12], styles: {} });
        pos = RE.lastIndex;
        continue;
      }
      const tagName = m[13];
      items.push({
        type: 'link',
        href: TAG_PREFIX + tagName,
        content: [{ type: 'text', text: m[12], styles: {} }],
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
      if (isTagHref(item.href || '')) {
        const text = item.content?.map((c: TextItem) => c?.text || '').join('') || '';
        out += text;
      } else if (isWikiLinkHref(item.href || '')) {
        // Guard against legacy corrupted hrefs that carry stray brackets
        // ([[Target]] wrapped into the target) — strip them so the serializer
        // cannot re-wrap into [[[[Target]]]] on the next round trip.
        const target = extractWikiLinkTarget(item.href).replace(/[[\]]/g, '').trim();
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
