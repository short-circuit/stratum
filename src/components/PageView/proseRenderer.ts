/**
 * Pure, deterministic read-mode prose renderer.
 *
 * Converts BlockNote-shaped blocks (the output of dtoToBlockNote) into an HTML
 * string of clean read-only prose. It is a string builder — NOT React — so it is
 * trivially testable and never touches the editor or the on-disk file.
 *
 * Inline content reuses the existing parse pipeline (parseContentToInlineItems)
 * and KaTeX for math; Mermaid is left as a placeholder hydrated by ProseView.
 *
 * @module proseRenderer
 */

import katex from 'katex';
import { MARKER_COLORS } from '../MarkerBadge';
import type { BlockMeta } from '../OutlinerEditor/dtoConverters';
import {
  isWikiLinkHref,
  extractWikiLinkTarget,
  isTagHref,
  extractTagTarget,
  isAssetHref,
  extractAssetRefPath,
  type InlineItem,
} from '../../lib/wikiLinks';

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function applyStyles(html: string, styles: Record<string, boolean>): string {
  let h = html;
  if (styles.code) h = `<code>${h}</code>`;
  if (styles.strike) h = `<del>${h}</del>`;
  if (styles.bold) h = `<strong>${h}</strong>`;
  if (styles.italic) h = `<em>${h}</em>`;
  return h;
}

// Render a plain-text segment to HTML, handling inline $…$ math and markdown
// external links [text](http…). Everything else is escaped. Styles are applied
// to the whole segment. On render failure the raw (escaped) text is kept so
// content is never lost.
function renderTextWithMath(text: string, styles: Record<string, boolean>): string {
  let html = '';
  let last = 0;
  const re = /(\$[^$\n]+\$)|(\[([^\]]+)\]\((https?:\/\/[^)\s]+|mailto:[^)\s]+)\))/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    html += escapeHtml(text.slice(last, m.index));
    if (m[1]) {
      const latex = m[1].slice(1, -1);
      try {
        html += `<span class="bn-prose-math">${katex.renderToString(latex, { displayMode: false, throwOnError: true })}</span>`;
      } catch {
        html += escapeHtml(m[1]);
      }
    } else if (m[3] && m[4]) {
      html += `<a href="${escapeHtml(m[4])}" target="_blank" rel="noreferrer">${escapeHtml(m[3])}</a>`;
    }
    last = re.lastIndex;
  }
  html += escapeHtml(text.slice(last));
  return applyStyles(html, styles);
}

function renderLink(item: InlineItem): string {
  if (item.type !== 'link') return '';
  const href = item.href || '';
  const display = (item.content || [])
    .map((c) => renderTextWithMath(c.text || '', c.styles || {}))
    .join('');
  if (isWikiLinkHref(href)) {
    const target = extractWikiLinkTarget(href).replace(/[[\]]/g, '').trim();
    return `<a class="bn-prose-link bn-prose-wikilink" href="#" data-prose-target="${escapeHtml(encodeURIComponent(target))}">${display}</a>`;
  }
  if (isTagHref(href)) {
    const tag = extractTagTarget(href);
    return `<a class="bn-prose-tag" href="#" data-prose-tag="${escapeHtml(tag)}">${display}</a>`;
  }
  if (isAssetHref(href)) {
    const ref = extractAssetRefPath(href).replace(/^\/\//, '');
    return `<a class="bn-prose-link" href="${escapeHtml(ref)}">${display}</a>`;
  }
  return `<a href="${escapeHtml(href)}" target="_blank" rel="noreferrer">${display}</a>`;
}

function renderInlineItem(item: InlineItem): string {
  if (item.type === 'link') return renderLink(item);
  return renderTextWithMath(item.text || '', item.styles || {});
}

function renderInlineItems(items: InlineItem[]): string {
  return (items || []).map(renderInlineItem).join('');
}

// Concatenate the plain-text portions of inline items (link items carry no text
// node of their own; their display text lives in item.content).
function itemsToRawText(items: InlineItem[]): string {
  return (items || []).map((c) => (c.type === 'text' ? c.text : '')).join('');
}

// Reconstruct the raw content string and check for a whole-block display math.
function blockContentHtml(block: { content?: InlineItem[] }): string {
  const joined = itemsToRawText(block.content || []);
  const dm = joined.match(/^\$\$([\s\S]+)\$\$$/);
  if (dm) {
    try {
      return `<span class="bn-prose-math-display">${katex.renderToString(dm[1], { displayMode: true, throwOnError: true })}</span>`;
    } catch {
      return escapeHtml(joined);
    }
  }
  return renderInlineItems(block.content || []);
}

function extractBlockText(block: { content?: InlineItem[] }): string {
  return itemsToRawText(block.content || []);
}

function markerChip(marker: string): string {
  const normalized = marker.toUpperCase();
  const colors = MARKER_COLORS[normalized] ?? { bg: '#6b7280', text: '#fff' };
  return `<span class="bn-prose-badge" style="background:${colors.bg};color:${colors.text}">${escapeHtml(marker)}</span>`;
}

function priorityChip(priority: string): string {
  return `<span class="bn-prose-priority" style="background:#6366f1;color:#fff">${escapeHtml(priority)}</span>`;
}

function clampLevel(level: unknown): number {
  const n = typeof level === 'number' ? level : 1;
  return Math.min(6, Math.max(1, n));
}

// Render the inner content of a block (without the wrapping .bn-prose-block div).
function renderBlockType(block: any, ctx: { num: number }): string {
  switch (block.type) {
    case 'heading': {
      const level = clampLevel(block.props?.level);
      return `<h${level}>${blockContentHtml(block)}</h${level}>`;
    }
    case 'codeBlock':
      return `<pre><code class="bn-prose-code">${escapeHtml(extractBlockText(block))}</code></pre>`;
    case 'mermaid':
      return `<div class="bn-prose-mermaid" data-mermaid-code="${escapeHtml(extractBlockText(block))}"></div>`;
    case 'divider':
      return '<hr>';
    case 'image': {
      const url = block.props?.url || '';
      const name = block.props?.name || '';
      if (isAssetHref(url)) {
        return `<img data-asset-ref="${escapeHtml(extractAssetRefPath(url))}" alt="${escapeHtml(name)}">`;
      }
      return `<img src="${escapeHtml(url)}" alt="${escapeHtml(name)}">`;
    }
    case 'file':
    case 'video':
    case 'audio': {
      const url = block.props?.url || '';
      const name = block.props?.name || '';
      const href = isAssetHref(url) ? extractAssetRefPath(url) : url;
      return `<a class="bn-prose-${block.type}" href="${escapeHtml(href)}">${escapeHtml(name)}</a>`;
    }
    case 'quote':
      return `<blockquote>${blockContentHtml(block)}</blockquote>`;
    case 'bulletListItem':
      return `<div class="bn-prose-li bn-prose-bullet"><span>•</span>${blockContentHtml(block)}</div>`;
    case 'numberedListItem':
      return `<div class="bn-prose-li bn-prose-num"><span>${ctx.num++}.</span>${blockContentHtml(block)}</div>`;
    case 'checkListItem':
      return `<div class="bn-prose-li bn-prose-check"><input type="checkbox" disabled${block.props?.checked ? ' checked' : ''}/>${blockContentHtml(block)}</div>`;
    case 'toggleListItem':
      return `<div class="bn-prose-li bn-prose-toggle">${blockContentHtml(block)}</div>`;
    default:
      return `<p>${blockContentHtml(block)}</p>`;
  }
}

function renderBlock(block: any, metaMap: Map<string, BlockMeta>, ctx: { num: number }): string {
  const meta = metaMap.get(block.id);
  let badges = '';
  if (meta) {
    if (meta.marker) badges += markerChip(meta.marker);
    if (meta.priority) badges += priorityChip(meta.priority);
  }
  const inner = renderBlockType(block, ctx);
  const children = (block.children || []).map((c: any) => renderBlock(c, metaMap, ctx)).join('');
  return `<div class="bn-prose-block" data-block-id="${escapeHtml(block.id)}">${badges}${inner}${children}</div>`;
}

/**
 * Render BlockNote-shaped blocks into read-only prose HTML.
 *
 * @param blocks - The output of dtoToBlockNote (BlockNote shape).
 * @param metaMap - Block metadata map (marker/priority/properties) keyed by block id.
 * @returns An HTML string of prose blocks. Never mutates the inputs.
 */
export function proseBlocksToHtml(
  blocks: any[],
  metaMap: Map<string, BlockMeta>,
): string {
  const ctx = { num: 1 };
  return (blocks || []).map((b) => renderBlock(b, metaMap, ctx)).join('');
}

// Extract text from a single block, excluding nested .bn-prose-block children
// (they are emitted as their own lines by proseBlocksToText).
function blockText(el: Element): string {
  return Array.from(el.childNodes)
    .map((n: Node) => {
      if (n.nodeType === 3) return n.nodeValue || '';
      if (n.nodeType !== 1) return '';
      const node = n as Element;
      if (node.classList.contains('bn-prose-block')) return '';
      if (node.hasAttribute('data-mermaid-code')) return node.getAttribute('data-mermaid-code') || '';
      if (node.tagName === 'IMG') return node.getAttribute('alt') || '';
      return blockText(node);
    })
    .join('');
}

/**
 * Extract plain text from prose HTML (test helper).
 *
 * Joins each block root with a newline, including Mermaid placeholder code,
 * image alt text, and KaTeX-rendered math text.
 *
 * @param html - Prose HTML produced by proseBlocksToHtml.
 * @param root - Optional detached container already holding the HTML. When
 *   omitted, a container is created from the html string.
 * @returns Each block's plain text on its own line.
 */
export function proseBlocksToText(html: string, root?: HTMLElement): string {
  let container: HTMLElement;
  if (root) {
    container = root;
  } else {
    const el = document.createElement('div');
    el.innerHTML = html;
    container = el;
  }
  const roots = container.querySelectorAll('.bn-prose-block');
  return Array.from(roots).map(blockText).join('\n');
}
