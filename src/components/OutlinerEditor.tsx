import { useCallback, useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useCreateBlockNote } from '@blocknote/react';
import { BlockNoteView } from '@blocknote/mantine';
import '@blocknote/core/fonts/inter.css';
import '@blocknote/mantine/style.css';
import type { BlockDto } from '../lib/types';
import * as api from '../lib/commands';
import { parseWikiLinks, isWikiLinkHref, extractWikiLinkTarget } from '../lib/wikiLinks';
import { useCtrlHeld } from '../lib/useCtrlHeld';
import LinkPreviewPopup from './LinkPreviewPopup';
import AISlashMenu from './AISlashMenu';
import AIFormattingToolbar from './AIFormattingToolbar';

interface Props {
  pagePath: string;
}

interface PreviewState {
  content: string;
  pageTitle: string | null;
  pagePath: string;
  position: { x: number; y: number };
  loading: boolean;
}

export default function OutlinerEditor({ pagePath }: Props) {
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState('init');
  const [preview, setPreview] = useState<PreviewState | null>(null);
  const hoverTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const hoveredLink = useRef<string | null>(null);
  const hoveredPos = useRef<{ x: number; y: number }>({ x: 0, y: 0 });
  const containerRef = useRef<HTMLDivElement>(null);
  const navigate = useNavigate();
  const { ctrlHeld } = useCtrlHeld();

  const editor = useCreateBlockNote({
    links: {
      isValidLink: () => true,
    },
  });
  console.log('[OutlinerEditor] created editor for', pagePath);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
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

  // Wiki-link interaction: click navigation, hover preview, Ctrl key detection
  useEffect(() => {
    const el = containerRef.current;
    if (!el || status !== 'ready') return;

    function getWikiLinkHref(target: EventTarget | null): string | null {
      const linkEl = (target as HTMLElement).closest('a');
      if (!linkEl) return null;
      const href = linkEl.getAttribute('href');
      if (!href || !isWikiLinkHref(href)) return null;
      return href;
    }

    const handleClick = (e: MouseEvent) => {
      const href = getWikiLinkHref(e.target);
      if (!href) return;
      e.preventDefault();
      const target = extractWikiLinkTarget(href);
      api.resolveLinkTarget(target).then(resolved => {
        if (resolved.page_path) {
          navigate(`/page/${encodeURIComponent(resolved.page_path)}`);
        }
      });
    };

    const triggerPreview = (href: string, x: number, y: number) => {
      const target = extractWikiLinkTarget(href);
      if (hoverTimer.current) clearTimeout(hoverTimer.current);
      hoverTimer.current = setTimeout(async () => {
        if (!ctrlHeld.current) { setPreview(null); return; }
        setPreview({
          content: '', pageTitle: null, pagePath: '',
          position: { x: x + 10, y: y + 10 },
          loading: true,
        });
        try {
          const resolved = await api.resolveLinkTarget(target);
          if (!resolved.page_path || !ctrlHeld.current) { setPreview(null); return; }
          const ctx = await api.getBacklinkContext(resolved.page_path, pagePath);
          if (!ctrlHeld.current) { setPreview(null); return; }
          setPreview({
            content: ctx?.content || '(empty)',
            pageTitle: ctx?.page_title || resolved.title || resolved.slug || target,
            pagePath: resolved.page_path,
            position: { x: x + 10, y: y + 10 },
            loading: false,
          });
        } catch { setPreview(null); }
      }, 200);
    };

    const handleMouseOver = (e: MouseEvent) => {
      const href = getWikiLinkHref(e.target);
      if (!href) { hoveredLink.current = null; return; }
      hoveredLink.current = href;
      hoveredPos.current = { x: e.clientX, y: e.clientY };
      if (ctrlHeld.current) {
        triggerPreview(href, e.clientX, e.clientY);
      }
    };

    const handleMouseOut = (e: MouseEvent) => {
      if (!getWikiLinkHref(e.target)) return;
      hoveredLink.current = null;
      if (hoverTimer.current) { clearTimeout(hoverTimer.current); hoverTimer.current = null; }
      setPreview(null);
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.key === 'Control' || e.key === 'Meta') && hoveredLink.current) {
        triggerPreview(hoveredLink.current, hoveredPos.current.x, hoveredPos.current.y);
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      if (e.key === 'Control' || e.key === 'Meta') {
        setPreview(null);
      }
    };

    el.addEventListener('click', handleClick);
    el.addEventListener('mouseover', handleMouseOver);
    el.addEventListener('mouseout', handleMouseOut);
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    return () => {
      el.removeEventListener('click', handleClick);
      el.removeEventListener('mouseover', handleMouseOver);
      el.removeEventListener('mouseout', handleMouseOut);
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
      if (hoverTimer.current) clearTimeout(hoverTimer.current);
    };
  }, [status, pagePath, ctrlHeld, navigate]);

  const persistBlocks = useCallback(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
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
      <div className="flex items-center justify-center h-64 text-[var(--secondary-400)] text-sm">
        <div className="text-center">
          <div className="animate-pulse mb-2">Loading editor...</div>
          <div className="text-xs text-[var(--secondary-500)]">{pagePath}</div>
        </div>
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
      <div ref={containerRef} style={{ height: '100%' }}>
        <BlockNoteView
          editor={editor}
          theme={document.documentElement.classList.contains('dark') ? 'dark' : 'light'}
          className="min-h-[400px] h-full"
          slashMenu={false}
          formattingToolbar={false}
        >
          <AISlashMenu pagePath={pagePath} />
          <AIFormattingToolbar />
        </BlockNoteView>
      </div>

      {preview && (
        <LinkPreviewPopup
          content={preview.content}
          pageTitle={preview.pageTitle}
          pagePath={preview.pagePath}
          position={preview.position}
          loading={preview.loading}
          onClose={() => setPreview(null)}
        />
      )}
    </div>
  );
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function dtoToBlockNote(dtos: BlockDto[]): any[] {
  if (dtos.length === 0) return [];
  const rootBlocks = dtos.filter(d => !d.parent_id);
  rootBlocks.sort((a, b) => {
    if (!a.left_id) return -1;
    if (a.left_id === b.id) return 1;
    return 0;
  });
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  function convert(dto: BlockDto): any {
    const children = dtos.filter(b => b.parent_id === dto.id);
    children.sort((a, b) => {
      if (!a.left_id) return -1;
      if (a.left_id === b.id) return 1;
      return 0;
    });
    let type = 'paragraph';
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const props: Record<string, any> = {};
    if (dto.heading_level) { type = 'heading'; props.level = dto.heading_level; }
    else if (dto.marker) { type = 'checkListItem'; }
    return {
      type,
      content: parseWikiLinks(dto.content || ''),
      props,
      children: children.map(convert),
    };
  }
  return rootBlocks.map(convert);
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function blockNoteToDto(blockNoteBlocks: any[]): BlockDto[] {
  const result: BlockDto[] = [];
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  function walk(blocks: any[], parentId: string | null) {
    let prevId: string | null = null;
    for (const b of blocks) {
      const id = b.id || crypto.randomUUID();
      let content = '';
      if (b.content) {
        if (typeof b.content === 'string') content = b.content;
        else if (Array.isArray(b.content)) {
          content = b.content.map(// eslint-disable-next-line @typescript-eslint/no-explicit-any
            (item: any) => {
            if (typeof item === 'string') return item;
            if (item?.text) return item.text;
            if (item?.type === 'link') {
              const h = item.href || '';
              if (h.includes('stratum.internal/')) {
                const target = decodeURIComponent(h.split('stratum.internal/')[1] || '');
                return `[[${target}]]`;
              }
              return `[[${h}]]`;
            }
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
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        heading_level: b.type === 'heading' ? (b.props as any)?.level ?? null : null,
      });
      if (b.children?.length) walk(b.children, id);
      prevId = id;
    }
  }
  walk(blockNoteBlocks, null);
  return result;
}
