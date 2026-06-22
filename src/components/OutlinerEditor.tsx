import { useCallback, useEffect, useRef, useState } from 'react';
import {
  useCreateBlockNote,
  getDefaultReactSlashMenuItems,
  SuggestionMenuController,
} from '@blocknote/react';
import { BlockNoteView } from '@blocknote/mantine';
import { filterSuggestionItems } from '@blocknote/core';
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
  const [initialBlocks, setInitialBlocks] = useState<PartialBlock[]>([]);

  useEffect(() => {
    setLoading(true);
    api.getBlocks(pagePath).then(({ blocks }) => {
      setInitialBlocks(dtoToBlockNote(blocks));
      setLoading(false);
    });
  }, [pagePath]);

  const editor = useCreateBlockNote({
    initialContent: initialBlocks,
  });

  // Custom slash menu items
  const getSlashMenuItems = useCallback(
    async (query: string) => {
      const today = new Date().toISOString().split('T')[0];

      const customItems = [
        {
          title: 'Task (TODO)',
          group: 'Stratum',
          icon: '☐',
          subtext: 'Create a task item',
          onItemClick: () => {
            editor.insertBlocks(
              [{ type: 'checkListItem' as any, content: '' }],
              editor.getTextCursorPosition().block,
              'after',
            );
          },
        },
        {
          title: 'Heading 1',
          group: 'Stratum',
          icon: 'H1',
          subtext: 'Large heading',
          onItemClick: () => {
            editor.insertBlocks(
              [{ type: 'heading' as any, content: '', props: { level: 1 } }],
              editor.getTextCursorPosition().block,
              'after',
            );
          },
        },
        {
          title: 'Heading 2',
          group: 'Stratum',
          icon: 'H2',
          subtext: 'Medium heading',
          onItemClick: () => {
            editor.insertBlocks(
              [{ type: 'heading' as any, content: '', props: { level: 2 } }],
              editor.getTextCursorPosition().block,
              'after',
            );
          },
        },
        {
          title: 'Code Block',
          group: 'Stratum',
          icon: '</>',
          subtext: 'Insert a code block',
          onItemClick: () => {
            editor.insertBlocks(
              [{ type: 'codeBlock' as any, content: '' }],
              editor.getTextCursorPosition().block,
              'after',
            );
          },
        },
        {
          title: 'Link Page',
          group: 'Stratum',
          icon: '🔗',
          subtext: 'Insert a link to another page',
          onItemClick: () => {
            editor.insertBlocks(
              [{ type: 'paragraph' as any, content: '[[Page Name]]' }],
              editor.getTextCursorPosition().block,
              'after',
            );
          },
        },
        {
          title: "Today's Date",
          group: 'Stratum',
          icon: '📅',
          subtext: today,
          onItemClick: () => {
            editor.insertBlocks(
              [{ type: 'paragraph' as any, content: today }],
              editor.getTextCursorPosition().block,
              'after',
            );
          },
        },
      ];

      return filterSuggestionItems(
        [...customItems, ...getDefaultReactSlashMenuItems(editor)],
        query,
      );
    },
    [editor],
  );

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
      persistBlocks(editor.document);
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
        slashMenu={false}
      />
      <SuggestionMenuController
        getItems={getSlashMenuItems as any}
        triggerCharacter="/"
        suggestionMenuComponent={undefined as any}
        onItemClick={() => {}}
      />
    </div>
  );
}

// Convert our BlockDto array to BlockNote PartialBlock array
function dtoToBlockNote(dtos: BlockDto[]): PartialBlock[] {
  const rootBlocks: BlockDto[] = [];

  for (const dto of dtos) {
    if (!dto.parent_id) {
      rootBlocks.push(dto);
    }
  }

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

    let type: string = 'paragraph';
    const props: Record<string, any> = {};

    if (dto.heading_level) {
      type = 'heading';
      props.level = dto.heading_level;
    } else if (dto.marker) {
      type = 'checkListItem';
    }

    return {
      type: type as any,
      content: dto.content,
      props,
      children: children.map(convertBlock),
    };
  }

  return rootBlocks.map(convertBlock);
}

function extractText(block: any): string {
  if (!block.content) return '';
  if (typeof block.content === 'string') return block.content;

  const contentArr = block.content as any[];
  if (!contentArr || !Array.isArray(contentArr)) return '';

  return contentArr
    .map((item: any) => {
      if (typeof item === 'string') return item;
      if (item?.text) return item.text;
      if (item?.type === 'link') return `[[${item.href || ''}]]`;
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
