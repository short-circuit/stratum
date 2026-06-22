import { useCallback, useEffect, useRef, useState } from 'react';
import { useCreateBlockNote } from '@blocknote/react';
import { BlockNoteView } from '@blocknote/mantine';
import '@blocknote/core/fonts/inter.css';
import '@blocknote/mantine/style.css';
import type { PartialBlock } from '@blocknote/core';
import type { BlockDto } from '../lib/types';
import * as api from '../lib/commands';

interface Props {
  pagePath: string;
}

export default function OutlinerEditor({ pagePath }: Props) {
  const [loading, setLoading] = useState(true);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Load initial blocks from backend
  const [initialBlocks, setInitialBlocks] = useState<PartialBlock[]>([]);

  useEffect(() => {
    setLoading(true);
    api.getBlocks(pagePath).then(({ blocks }) => {
      const bnBlocks = dtoToBlockNote(blocks);
      setInitialBlocks(bnBlocks);
      setLoading(false);
    });
  }, [pagePath]);

  const editor = useCreateBlockNote({
    initialContent: initialBlocks,
  });

  // Persist changes to backend (debounced)
  const persistBlocks = useCallback(
    (blockNoteBlocks: any[]) => {
      if (saveTimer.current) clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(async () => {
        try {
          const markdown = blocksToMarkdown(blockNoteBlocks);
          await api.savePage(pagePath, markdown);
        } catch (e) {
          console.error('Failed to save blocks:', e);
        }
      }, 500);
    },
    [pagePath],
  );

  // Listen for changes
  useEffect(() => {
    if (!editor) return;
    return editor.onChange(() => {
      const currentBlocks = editor.document;
      persistBlocks(currentBlocks);
    });
  }, [editor, persistBlocks]);

  if (loading) {
    return <div className="p-4 text-gray-400 text-sm">Loading editor...</div>;
  }

  return (
    <div className="blocknote-editor-container">
      <BlockNoteView
        editor={editor}
        theme="light"
        className="min-h-[400px]"
      />
    </div>
  );
}

// Convert our BlockDto array to BlockNote PartialBlock array
function dtoToBlockNote(dtos: BlockDto[]): PartialBlock[] {
  // Build parent/child relationships from parent_id/left_id
  const blockMap = new Map<string, BlockDto>();
  const rootBlocks: BlockDto[] = [];

  for (const dto of dtos) {
    blockMap.set(dto.id, dto);
    if (!dto.parent_id) {
      rootBlocks.push(dto);
    }
  }

  // Sort roots by left_id chain
  rootBlocks.sort((a, b) => {
    if (!a.left_id) return -1;
    if (a.left_id === b.id) return 1;
    return 0;
  });

  function convertBlock(dto: BlockDto): PartialBlock {
    const children = dtos.filter(b => b.parent_id === dto.id);
    children.sort((a, b) => {
      if (!a.left_id) return -1;
      if (a.left_id === b.id) return 1;
      return 0;
    });

    // Determine block type based on marker
    let type: string = 'paragraph';
    let props: Record<string, any> = {};

    if (dto.heading_level) {
      type = 'heading';
      props.level = dto.heading_level;
    } else if (dto.marker) {
      type = 'checkListItem';
    }

    const partial: PartialBlock = {
      type: type as any,
      content: dto.content,
      props,
      children: children.map(convertBlock),
    };

    return partial;
  }

  return rootBlocks.map(convertBlock);
}

// Extract plain text from BlockNote block content
function extractText(block: any): string {
  if (!block.content) return '';
  if (typeof block.content === 'string') return block.content;

  const contentArr = block.content as any[];
  if (!contentArr || !Array.isArray(contentArr)) return '';

  return contentArr
    .map((item: any) => {
      if (typeof item === 'string') return item;
      if (item?.text) return item.text;
      if (item?.type === 'link') return item.href || '';
      return '';
    })
    .join('');
}

function blocksToMarkdown(blocks: any[]): string {
  let md = '';

  function walk(blks: any[], depth: number) {
    const indent = '  '.repeat(depth);
    for (const block of blks) {
      const text = extractText(block);
      const type = block.type;

      if (type === 'heading') {
        const level = (block.props as any)?.level || 1;
        md += `${'#'.repeat(level)} ${text}\n`;
      } else if (type === 'checkListItem') {
        md += `${indent}- [ ] ${text}\n`;
      } else if (type === 'bulletListItem') {
        md += `${indent}- ${text}\n`;
      } else if (type === 'numberedListItem') {
        md += `${indent}1. ${text}\n`;
      } else if (type === 'codeBlock') {
        md += `${indent}\`\`\`\n${text}\n${indent}\`\`\`\n`;
      } else {
        md += `${indent}- ${text}\n`;
      }

      if (block.children && block.children.length > 0) {
        walk(block.children, depth + 1);
      }
    }
  }

  walk(blocks, 0);
  return md;
}
