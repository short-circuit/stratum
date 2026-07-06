import type { BlockDto } from '../../lib/types';
import { parseContentToInlineItems, inlineItemsToContent } from '../../lib/wikiLinks';

export interface BlockMeta {
  marker: string | null;
  priority: string | null;
  properties: [string, string][];
}

interface InlineContentItem {
  text?: string;
  type?: string;
}

export const mermaidBlockRegex = /^```mermaid\n?([\s\S]*?)\n?```\s*$/;

export function extractTextContent(block: { content?: InlineContentItem[] }): string {
  if (!block.content || !Array.isArray(block.content)) return '';
  return block.content.map((c: InlineContentItem) => c.text || '').join('');
}

export function dtoToBlockNote(dtos: BlockDto[], metaMap: Map<string, BlockMeta>): any[] {
  if (dtos.length === 0) return [];
  const rootBlocks = dtos.filter(d => !d.parent_id);
  rootBlocks.sort((a, b) => {
    if (!a.left_id) return -1;
    if (a.left_id === b.id) return 1;
    return 0;
  });
  function convert(dto: BlockDto): any {
    const children = dtos.filter(b => b.parent_id === dto.id);
    children.sort((a, b) => {
      if (!a.left_id) return -1;
      if (a.left_id === b.id) return 1;
      return 0;
    });
    const contentStr = dto.content || '';
    const mermaidMatch = contentStr.match(mermaidBlockRegex);
    if (mermaidMatch) {
      return {
        type: 'mermaid',
        props: { language: 'mermaid' },
        content: [{ type: 'text' as const, text: mermaidMatch[1].trimEnd(), styles: {} }],
        children: children.map(convert),
      };
    }
    let type = 'paragraph';
    const props: Record<string, any> = {};
    if (dto.heading_level) { type = 'heading'; props.level = dto.heading_level; }
    // Store ALL block metadata in blockMetaRef (even without marker, to preserve priority/properties)
    metaMap.set(dto.id, {
      marker: dto.marker,
      priority: dto.priority,
      properties: dto.properties,
    });
    // Parse inline content (bold, italic, wiki-links, code, strikethrough)
    // before passing to BlockNote, which stores content as InlineContent[] internally
    const content = parseContentToInlineItems(contentStr);
    if (dto.marker === 'DONE' || dto.marker === 'CANCELLED') {
      for (const item of content) {
        if (item.type === 'text' && !item.styles?.strike) {
          item.styles = { ...item.styles, strike: true };
        }
      }
    }
    return { id: dto.id, type, content, props, children: children.map(convert) };
  }
  return rootBlocks.map(convert);
}

export function blockNoteToDto(blockNoteBlocks: any[], metaMap: Map<string, BlockMeta>): BlockDto[] {
  const result: BlockDto[] = [];
  function walk(blocks: any[], parentId: string | null) {
    let prevId: string | null = null;
    for (const b of blocks) {
      const id = b.id || crypto.randomUUID();
      let content = '';
      if (b.type === 'mermaid') {
        let code = '';
        if (typeof b.content === 'string') code = b.content;
        else if (Array.isArray(b.content)) code = b.content.map((c: { text?: string }) => c?.text || '').join('');
        content = '```mermaid\n' + code + '\n```';
      } else if (b.type === 'codeBlock' && b.props?.language === 'mermaid') {
        let code = '';
        if (typeof b.content === 'string') code = b.content;
        else if (Array.isArray(b.content)) code = b.content.map((c: { text?: string }) => c?.text || '').join('');
        content = '```mermaid\n' + code + '\n```';
      } else if (b.content) {
        if (typeof b.content === 'string') content = b.content;
        else if (Array.isArray(b.content)) content = inlineItemsToContent(b.content);
      }
      const meta = metaMap.get(id) ?? { marker: null, priority: null, properties: [] };
      result.push({
        id, content,
        parent_id: parentId,
        left_id: prevId,
        properties: meta.properties,
        marker: meta.marker,
        priority: meta.priority,
        collapsed: false,
        heading_level: b.type === 'heading' ? (b.props as any)?.level ?? null : null,
      });
      if (b.children?.length) walk(b.children, id);
      prevId = id;
    }
  }
  walk(blockNoteBlocks, null);
  return result;
}
