import { extractTextContent } from './dtoConverters';
import type { BlockMeta } from './dtoConverters';

export const MARKER_KEYWORDS = ['TODO', 'DOING', 'DONE', 'NOW', 'LATER', 'WAITING', 'CANCELLED'];

/**
 * Detect marker keywords at the start of block content and apply them as metadata.
 * Only operates on journal pages. Skips blocks that already have a marker.
 * Mutates blockMetaRef and block content in place.
 */
export function detectAndApplyMarkers(
  blocks: unknown[],
  pagePath: string,
  blockMetaRef: Map<string, BlockMeta>,
): void {
  if (!pagePath.startsWith('journals/')) return;
  for (const b of blocks) {
    const block = b as { id: string; content?: { text?: string }[] };
    const content = extractTextContent(block);
    const firstWord = content.trim().split(/\s+/)[0]?.toUpperCase();
    if (firstWord && MARKER_KEYWORDS.includes(firstWord)) {
      const existingMeta = blockMetaRef.get(block.id);
      if (existingMeta?.marker) continue;
      const existing = blockMetaRef.get(block.id) ?? { marker: null, priority: null, properties: [] as [string, string][] };
      blockMetaRef.set(block.id, { ...existing, marker: firstWord });
      const rest = content.trim().slice(firstWord.length).trim();
      if (rest !== content.trim() && Array.isArray(block.content) && block.content.length > 0) {
        block.content[0].text = rest;
      }
    }
  }
}
