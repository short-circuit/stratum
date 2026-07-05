import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useCreateBlockNote } from '@blocknote/react';
import { BlockNoteView } from '@blocknote/mantine';
import { useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import Button from '@mui/material/Button';
import Alert from '@mui/material/Alert';
import CircularProgress from '@mui/material/CircularProgress';
import Popover from '@mui/material/Popover';
import IconButton from '@mui/material/IconButton';
import Tooltip from '@mui/material/Tooltip';
import AddCircleIcon from '@mui/icons-material/AddCircle';
import { useStore } from '../../stores/appStore';
import '@blocknote/core/fonts/inter.css';
import '@blocknote/mantine/style.css';
import { BlockNoteSchema, defaultBlockSpecs } from '@blocknote/core';
import * as api from '../../lib/commands';
import { normalizeContent, isWikiLinkHref, extractWikiLinkTarget, isTagHref, extractTagTarget } from '../../lib/wikiLinks';
import { useCtrlHeld } from '../../lib/useCtrlHeld';
import LinkPreviewPopup from '../LinkPreviewPopup';
import AISlashMenu from '../AISlashMenu';
import AIFormattingToolbar from '../AIFormattingToolbar';
import { createMermaidSpec } from '../MermaidBlock';
import { useMathInline, setupMathDblClick } from '../../lib/useMathInline';
import MathEditorModal from '../MathEditorModal';
import MarkerBadge from '../MarkerBadge';
import { dtoToBlockNote, blockNoteToDto } from './dtoConverters';
import type { BlockMeta } from './dtoConverters';
import { MARKER_KEYWORDS, detectAndApplyMarkers } from './markerDetection';

const schema = BlockNoteSchema.create({
  blockSpecs: {
    ...defaultBlockSpecs,
    mermaid: createMermaidSpec(),
  },
});

interface Props {
  pagePath: string;
}

export default function OutlinerEditor({ pagePath }: Props) {
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const blockMetaRef = useRef<Map<string, BlockMeta>>(new Map());
  const isProcessingRef = useRef(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState('init');
  const [mathEdit, setMathEdit] = useState<{
    latex: string;
    pos: number;
  } | null>(null);
  const [pageMarkers, setPageMarkers] = useState<string[]>([]);
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

  const [deadLinkPopup, setDeadLinkPopup] = useState<{
    target: string;
    position: { x: number; y: number };
  } | null>(null);

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
          const rect = a.getBoundingClientRect();
          const pos = { x: rect.left, y: rect.bottom };
          let target = extractWikiLinkTarget(href);
          target = target.replace(/[[\]]/g, '').trim().toLowerCase();
          api.resolveLinkTarget(target).then(resolved => {
            if (resolved.page_path) {
              navigateRef.current('/page/' + encodeURIComponent(resolved.page_path));
            } else {
              setDeadLinkPopup({ target, position: pos });
            }
          });
          return true;
        }
        return false;
      },
    },
  });

  function setBlockMeta(id: string, meta: Partial<BlockMeta>): void {
    const existing = blockMetaRef.current.get(id) ?? { marker: null, priority: null, properties: [] };
    blockMetaRef.current.set(id, { ...existing, ...meta });
  }

  // Step 1: Load blocks as strings
  useEffect(() => {
    api.getBlocks(pagePath)
      .then(({ blocks }) => {
        try {
          blockMetaRef.current.clear();
          for (const b of blocks) b.content = normalizeContent(b.content);
          const bnBlocks = dtoToBlockNote(blocks, blockMetaRef.current);
          if (bnBlocks.length > 0) {
            editor.replaceBlocks(editor.document, bnBlocks);
          }
          // Collect unique markers from loaded blocks
          const markers = [...new Set(blocks.filter(b => b.marker).map(b => b.marker!))];
          setPageMarkers(markers);
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
  }, [pagePath, editor]);

  // Step 2: onChange — debounced save
  const persistBlocks = useCallback(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (blockNoteBlocks: any[]) => {
      if (saveTimer.current) clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(async () => {
        try {
          // Run marker detection before converting to DTOs
          detectAndApplyMarkers(blockNoteBlocks, pagePath, blockMetaRef.current);
          const dtos = blockNoteToDto(blockNoteBlocks, blockMetaRef.current);
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
      if (isProcessingRef.current) return;
      persistBlocks(editor.document);
    });
  }, [editor, persistBlocks, status]);

  // Inline math rendering via ProseMirror decorations
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  useMathInline(editor as any, status === 'ready');

  // Double-click on rendered math to open editor
  useEffect(() => {
    if (status !== 'ready') return;
    return setupMathDblClick(
      containerRef.current,
      (latex: string, pos: number) => setMathEdit({ latex, pos }),
    );
  }, [status]);

  // Check pages in the store to mark dead wiki-links without API calls
  function markDeadLinks(root: HTMLElement) {
    const anchors = root.querySelectorAll<HTMLAnchorElement>('a[data-inline-content-type="link"][href^="stratum:"]');
    if (!anchors.length) return;
    const slugs = new Set(
      useStore.getState().pages.map(p =>
        (p.slug || p.path.replace(/\.md$/i, '')).toLowerCase(),
      ),
    );
    for (const a of anchors) {
      const href = a.getAttribute('href');
      if (!href || href.startsWith('stratum-tag:')) continue;
      const target = extractWikiLinkTarget(href).replace(/[[\]]/g, '').trim().toLowerCase();
      if (!slugs.has(target)) {
        a.style.color = '#d97706';
        a.style.textDecoration = 'underline dashed';
      } else {
        a.style.color = '';
        a.style.textDecoration = '';
      }
    }
  }

  // Delayed dead-link scan after editor fully renders
  useEffect(() => {
    if (status !== 'ready') return;
    const el = containerRef.current;
    if (!el) return;
    const t = setTimeout(() => markDeadLinks(el), 1000);
    return () => clearTimeout(t);
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

  // Wiki-link hover preview delegation + click handling (native click prevents browser navigation)
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

    const handleLinkPrevent = (e: MouseEvent) => {
      // Only prevent browser from following stratum: href — does NOT stop propagation
      // so ProseMirror's handleClick can still fire the links.onClick callback
      const a = (e.target as HTMLElement).closest?.('a');
      if (!a) return;
      const href = a.getAttribute('href');
      if (!href || !isWikiLinkHref(href)) return;
      e.preventDefault();
    };

    el.addEventListener('mouseover', handleMouseOver);
    el.addEventListener('mouseout', handleMouseOut);
    el.addEventListener('mousedown', handleLinkPrevent, true);
    el.addEventListener('click', handleLinkPrevent, true);
    return () => {
      el.removeEventListener('mouseover', handleMouseOver);
      el.removeEventListener('mouseout', handleMouseOut);
      el.removeEventListener('mousedown', handleLinkPrevent, true);
      el.removeEventListener('click', handleLinkPrevent, true);
    };
  }, [status, showPreview, dismissPreview]);

  useEffect(() => {
    const check = () => { if (!ctrlHeld.current) dismissPreview(); };
    const interval = setInterval(check, 100);
    return () => clearInterval(interval);
  }, [ctrlHeld, dismissPreview]);

  // Memoize the editor view so it doesn't re-render on popup state changes (which would reset scroll position)
  const editorView = useMemo(() => (
    <BlockNoteView
      editor={editor}
      theme={document.documentElement.classList.contains('dark') ? 'dark' : 'light'}
      style={{ minHeight: '400px', height: '100%' }}
      slashMenu={false}
      formattingToolbar={false}
      linkToolbar={false}
    >
      <AISlashMenu pagePath={pagePath} />
      <AIFormattingToolbar />
    </BlockNoteView>
  ), [editor, pagePath]);

  if (status === 'init' || status === 'loading') {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: 256 }}>
        <Box sx={{ textAlign: 'center' }}>
          <CircularProgress size={20} sx={{ mb: 1.5 }} />
          <Typography variant="body2" color="text.secondary">Loading editor...</Typography>
          <Typography variant="caption" color="text.disabled" sx={{ display: 'block', mt: 0.25 }}>{pagePath}</Typography>
        </Box>
      </Box>
    );
  }

  if (error) {
    return (
      <Alert severity="error" sx={{ m: 2 }} action={
        <Button size="small" color="inherit" onClick={() => { setError(null); setStatus('init'); }}>Retry</Button>
      }>
        Editor error: {error}
      </Alert>
    );
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {pageMarkers.length > 0 && (
        <Box sx={{ px: 2, py: 0.5, display: 'flex', gap: 0.5, flexWrap: 'wrap', borderBottom: 1, borderColor: 'divider', bgcolor: 'action.hover' }}>
          {pageMarkers.map(m => <MarkerBadge key={m} marker={m} />)}
        </Box>
      )}
      <div ref={containerRef} className="blocknote-editor-container" style={{ flex: 1, minHeight: 0 }}>
        {editorView}
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

      <Popover
        open={Boolean(deadLinkPopup)}
        anchorReference="anchorPosition"
        anchorPosition={deadLinkPopup ? { left: deadLinkPopup.position.x, top: deadLinkPopup.position.y } : undefined}
        onClose={() => setDeadLinkPopup(null)}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
        transformOrigin={{ vertical: 'top', horizontal: 'left' }}
        disableScrollLock
        slotProps={{ paper: { sx: { p: 0.5 } } }}
      >
        <Tooltip title="Create page">
          <IconButton
            size="small"
            color="primary"
            onClick={async () => {
              if (!deadLinkPopup) return;
              const slug = deadLinkPopup.target;
              try {
                await api.createPage(slug);
              } catch (e) {
                if (!String(e).includes('already exists')) {
                  console.error('Failed to create page:', e);
                  setDeadLinkPopup(null);
                  return;
                }
              }
              navigate('/page/' + encodeURIComponent(slug));
              setDeadLinkPopup(null);
            }}
          >
            <AddCircleIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      </Popover>

      {mathEdit && (
        <MathEditorModal
          initialLatex={mathEdit.latex}
          onSave={(latex) => {
            const pos = mathEdit.pos;
            setMathEdit(null);
            if (!latex.trim()) return;
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            const view = (editor as any)?.prosemirrorView as import('prosemirror-view').EditorView | undefined;
            if (!view) return;
            const text = `$${latex}$`;
            const tr = view.state.tr.replaceWith(pos, pos + mathEdit.latex.length + 2, view.state.schema.text(text));
            view.dispatch(tr);
            view.focus();
          }}
          onCancel={() => setMathEdit(null)}
        />
      )}
    </div>
    </Box>
  );
}
