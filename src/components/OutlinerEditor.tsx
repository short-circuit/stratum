import { useCallback, useEffect, useRef } from 'react';
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
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const editor = useCreateBlockNote();

  // Load blocks into editor after mount/page change
  useEffect(() => {
    api.getBlocks(pagePath).then(({ blocks }) => {
      const bnBlocks = dtoToBlockNote(blocks);
      if (editor) {
        editor.replaceBlocks(editor.document, bnBlocks);
      }
    }).catch(err => {
      console.error('Failed to load blocks for', pagePath, err);
    });
  }, [pagePath, editor]);

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

  // Persist changes to backend via Rust (proper .id: serialization + SQLite)
  const persistBlocks = useCallback(
    (blockNoteBlocks: any[]) => {
      if (saveTimer.current) clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(async () => {
        try {
          const dtos = blockNoteToDto(blockNoteBlocks);
          await api.saveBlocks(pagePath, dtos);
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

function blockNoteToDto(blockNoteBlocks: any[]): BlockDto[] {
  const result: BlockDto[] = [];

  function walk(blocks: any[], parentId: string | null) {
    let prevId: string | null = null;
    for (const block of blocks) {
      const id = block.id || crypto.randomUUID();
      const content = extractText(block);

      result.push({
        id,
        content,
        parent_id: parentId,
        left_id: prevId,
        properties: [],
        marker: block.type === 'checkListItem' ? 'TODO' : null,
        priority: null,
        collapsed: false,
        heading_level: block.type === 'heading' ? (block.props as any)?.level ?? null : null,
      });

      if (block.children && block.children.length > 0) {
        walk(block.children, id);
      }

      prevId = id;
    }
  }

  walk(blockNoteBlocks, null);
  return result;
}

function dtoToBlockNote(dtos: BlockDto[]): PartialBlock[] {
  if (dtos.length === 0) return [];

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
      content: dto.content || '',
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
