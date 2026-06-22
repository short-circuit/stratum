import { useCallback, useEffect, useRef, useState } from 'react';
import { useCreateBlockNote } from '@blocknote/react';
import { BlockNoteView } from '@blocknote/mantine';
import '@blocknote/core/fonts/inter.css';
import '@blocknote/mantine/style.css';
import type { BlockDto } from '../lib/types';
import * as api from '../lib/commands';

interface Props {
  pagePath: string;
}

export default function OutlinerEditor({ pagePath }: Props) {
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState('init');

  const editor = useCreateBlockNote();
  console.log('[OutlinerEditor] created editor for', pagePath);

  useEffect(() => {
    setStatus('loading');
    console.log('[OutlinerEditor] loading blocks for', pagePath);
    api.getBlocks(pagePath)
      .then(({ blocks }) => {
        console.log('[OutlinerEditor] got', blocks.length, 'blocks');
        try {
          const bnBlocks = dtoToBlockNote(blocks);
          // Don't replace if no blocks — keep BlockNote's default paragraph
          if (bnBlocks.length > 0) {
            editor.replaceBlocks(editor.document, bnBlocks);
          }
          setStatus('ready');
        } catch (e) {
          console.error('[OutlinerEditor] replaceBlocks failed:', e);
          setError(String(e));
          setStatus('error');
        }
      })
      .catch(err => {
        console.error('[OutlinerEditor] getBlocks failed:', err);
        setError(String(err));
        setStatus('error');
      });
  }, [pagePath]);

  const persistBlocks = useCallback(
    (blockNoteBlocks: any[]) => {
      if (saveTimer.current) clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(async () => {
        try {
          const dtos = blockNoteToDto(blockNoteBlocks);
          await api.saveBlocks(pagePath, dtos);
          console.log('[OutlinerEditor] saved', dtos.length, 'blocks');
        } catch (e) {
          console.error('[OutlinerEditor] save failed:', e);
        }
      }, 500);
    },
    [pagePath],
  );

  useEffect(() => {
    if (!editor) return;
    return editor.onChange(() => {
      persistBlocks(editor.document);
    });
  }, [editor, persistBlocks]);

  if (status === 'init' || status === 'loading') {
    return (
      <div className="flex items-center justify-center h-64 text-gray-400 text-sm">
        Loading editor for {pagePath}...
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-4 text-red-500 text-sm">
        Editor error: {error}
        <button
          onClick={() => { setError(null); setStatus('init'); }}
          className="ml-2 underline"
        >
          Retry
        </button>
      </div>
    );
  }

  return (
    <div className="blocknote-editor-container" style={{ height: '100%' }}>
      <BlockNoteView
        editor={editor}
        theme="light"
        className="min-h-[400px] h-full"
      />
    </div>
  );
}

function dtoToBlockNote(dtos: BlockDto[]): any[] {
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
    let type = 'paragraph';
    const props: Record<string, any> = {};
    if (dto.heading_level) { type = 'heading'; props.level = dto.heading_level; }
    else if (dto.marker) { type = 'checkListItem'; }
    return {
      type,
      content: dto.content || '',
      props,
      children: children.map(convert),
    };
  }
  return rootBlocks.map(convert);
}

function blockNoteToDto(blockNoteBlocks: any[]): BlockDto[] {
  const result: BlockDto[] = [];
  function walk(blocks: any[], parentId: string | null) {
    let prevId: string | null = null;
    for (const b of blocks) {
      const id = b.id || crypto.randomUUID();
      let content = '';
      if (b.content) {
        if (typeof b.content === 'string') content = b.content;
        else if (Array.isArray(b.content)) {
          content = b.content.map((item: any) => {
            if (typeof item === 'string') return item;
            if (item?.text) return item.text;
            if (item?.type === 'link') return `[[${item.href || ''}]]`;
            return '';
          }).join('');
        }
      }
      result.push({
        id, content,
        parent_id: parentId,
        left_id: prevId,
        properties: [],
        marker: b.type === 'checkListItem' ? 'TODO' : null,
        priority: null,
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
