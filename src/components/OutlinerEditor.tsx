import { useCallback, useEffect, useRef, useState } from 'react';
import { useCreateBlockNote } from '@blocknote/react';
import { BlockNoteView } from '@blocknote/mantine';
import { useNavigate } from 'react-router-dom';
import '@blocknote/core/fonts/inter.css';
import '@blocknote/mantine/style.css';
import { BlockNoteSchema, defaultBlockSpecs } from '@blocknote/core';
import type { BlockDto } from '../lib/types';
import * as api from '../lib/commands';
import { parseContentToInlineItems, inlineItemsToContent, isWikiLinkHref, extractWikiLinkTarget, isTagHref, extractTagTarget, normalizeContent } from '../lib/wikiLinks';
import { useCtrlHeld } from '../lib/useCtrlHeld';
import LinkPreviewPopup from './LinkPreviewPopup';
import AISlashMenu from './AISlashMenu';
import AIFormattingToolbar from './AIFormattingToolbar';
import { createMermaidSpec } from './MermaidBlock';

const schema = BlockNoteSchema.create({
  blockSpecs: {
    ...defaultBlockSpecs,
    mermaid: createMermaidSpec(),
  },
});

const mermaidBlockRegex = /^```mermaid\n?([\s\S]*?)\n?```\s*$/;

interface Props {
  pagePath: string;
}

export default function OutlinerEditor({ pagePath }: Props) {
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState('init');
  const containerRef = useRef<HTMLDivElement>(null);
  const ctrlHeld = useCtrlHeld();
  const hoverTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [preview, setPreview] = useState<{
    content: string;
    pageTitle: string | null;
    pagePath: string;
    position: { x: number; y: number };
    loading: boolean;
  } | null>(null);
  const navigate = useNavigate();
  const navigateRef = useRef(navigate);
  useEffect(() => { navigateRef.current = navigate; }, [navigate]);

  const editor = useCreateBlockNote({
    schema,
    links: {
      isValidLink: (href) => {
        if (href.startsWith('stratum:') || href.startsWith('stratum-tag:')) return true;
        if (!href) return true;
        return /^(?:https?|ftp|ftps|mailto|tel|callto|sms|cid|xmpp):/i.test(href);
      },
      onClick: (event) => {
        const a = (event.target as HTMLElement).closest?.('a');
        if (!a) return false;
        const href = a.getAttribute('href');
        if (!href) return false;
        if (isTagHref(href)) {
          if (!event.ctrlKey && !event.metaKey) return true;
          const tagName = extractTagTarget(href);
          navigateRef.current('/search?q=' + encodeURIComponent('#' + tagName));
          return true;
        }
        if (isWikiLinkHref(href)) {
          api.resolveLinkTarget(extractWikiLinkTarget(href)).then(resolved => {
            if (resolved.page_path) {
              navigateRef.current('/page/' + encodeURIComponent(resolved.page_path));
            }
          });
          return true;
        }
        return false;
      },
    },
  });

  // Step 1: Load blocks as strings
  useEffect(() => {
    setStatus('loading');
    api.getBlocks(pagePath)
      .then(({ blocks }) => {
        try {
          for (const b of blocks) b.content = normalizeContent(b.content);
          const bnBlocks = dtoToBlockNote(blocks);
          if (bnBlocks.length > 0) {
            editor.replaceBlocks(editor.document, bnBlocks);
          }
          // onChange not registered yet — no save from load
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

  // Step 2: onChange — debounced save
  const persistBlocks = useCallback(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (blockNoteBlocks: any[]) => {
      if (saveTimer.current) clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(async () => {
        try {
          const dtos = blockNoteToDto(blockNoteBlocks);
          await api.saveBlocks(pagePath, dtos);
        } catch (e) {
          console.error('[OutlinerEditor] save failed:', e);
        }
      }, 500);
    },
    [pagePath],
  );

  useEffect(() => {
    if (!editor || status !== 'ready') return;
    return editor.onChange(() => {
      persistBlocks(editor.document);
    });
  }, [editor, persistBlocks, status]);

  // Wiki-link hover preview delegation
  useEffect(() => {
    const el = containerRef.current;
    if (!el || status !== 'ready') return;

    let currentHovered = '';

    const handleMouseOver = (e: MouseEvent) => {
      const a = (e.target as HTMLElement).closest?.('a');
      if (!a) return;
      const href = a.getAttribute('href');
      if (!href || !isWikiLinkHref(href)) return;
      if (href === currentHovered) return;
      currentHovered = href;
      const rect = a.getBoundingClientRect();
      showPreview(href, rect.left, rect.bottom + 4);
    };

    const handleMouseOut = (e: MouseEvent) => {
      const a = (e.target as HTMLElement).closest?.('a');
      if (!a) { currentHovered = ''; dismissPreview(); return; }
      const href = a.getAttribute('href');
      if (!href || !isWikiLinkHref(href)) { currentHovered = ''; dismissPreview(); return; }
      currentHovered = '';
      dismissPreview();
    };

    el.addEventListener('mouseover', handleMouseOver);
    el.addEventListener('mouseout', handleMouseOut);
    return () => {
      el.removeEventListener('mouseover', handleMouseOver);
      el.removeEventListener('mouseout', handleMouseOut);
    };
  }, [status]);

  // Preview popup helpers
  const showPreview = useCallback((href: string, x: number, y: number) => {
    if (!ctrlHeld.current) return;
    const target = extractWikiLinkTarget(href);
    if (hoverTimer.current) clearTimeout(hoverTimer.current);
    hoverTimer.current = setTimeout(async () => {
      if (!ctrlHeld.current) { setPreview(null); return; }
      setPreview({
        content: '', pageTitle: null, pagePath: '',
        position: { x, y },
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
          position: { x, y },
          loading: false,
        });
      } catch { setPreview(null); }
    }, 200);
  }, [ctrlHeld, pagePath]);

  const dismissPreview = useCallback(() => {
    if (hoverTimer.current) { clearTimeout(hoverTimer.current); hoverTimer.current = null; }
    setPreview(null);
  }, []);

  useEffect(() => {
    const check = () => { if (!ctrlHeld.current) dismissPreview(); };
    const interval = setInterval(check, 100);
    return () => clearInterval(interval);
  }, [ctrlHeld, dismissPreview]);

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
    <div ref={containerRef} className="blocknote-editor-container" style={{ height: '100%' }}>
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
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const props: Record<string, any> = {};
    if (dto.heading_level) { type = 'heading'; props.level = dto.heading_level; }
    else if (dto.marker) { type = 'checkListItem'; }
    // Parse inline content (bold, italic, wiki-links, code, strikethrough)
    // before passing to BlockNote, which stores content as InlineContent[] internally
    const content = parseContentToInlineItems(contentStr);
    return { type, content, props, children: children.map(convert) };
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
