/**
 * ProseView — read-only prose renderer for a page.
 *
 * Loads the page's blocks, converts them via the existing pipeline, and renders
 * clean read-only prose (no editor chrome). Wiki-links/tags navigate; double
 * clicking a block exits prose and focuses that block in the editor. Never
 * writes to disk.
 *
 * @module ProseView
 */

import { useEffect, useRef, useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import CircularProgress from '@mui/material/CircularProgress';
import { useTranslation } from 'react-i18next';
import * as api from '../../lib/commands';
import { normalizeContent } from '../../lib/wikiLinks';
import { dtoToBlockNote } from '../OutlinerEditor/dtoConverters';
import { proseBlocksToHtml } from './proseRenderer';
import { renderMermaidSvg } from '../../lib/mermaidRender';

interface Props {
  pagePath: string;
  /** Exit prose back to editor (header "Edit" button). */
  onExit: () => void;
  /** Exit prose and focus the given block in the editor (double-click jump). */
  onEditBlock: (blockId: string) => void;
}

type LoadState = 'loading' | 'error' | 'empty' | 'ready';

export default function ProseView({ pagePath, onEditBlock }: Props) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const containerRef = useRef<HTMLDivElement>(null);
  const [state, setState] = useState<LoadState>('loading');
  const [html, setHtml] = useState('');

  useEffect(() => {
    let cancelled = false;
    setState('loading');
    setHtml('');
    api
      .getBlocks(pagePath)
      .then(({ blocks }) => {
        if (cancelled) return;
        const metaRef = new Map();
        for (const b of blocks) b.content = normalizeContent(b.content);
        const bn = dtoToBlockNote(blocks, metaRef);
        if (bn.length === 0) {
          setState('empty');
          return;
        }
        setHtml(proseBlocksToHtml(bn, metaRef));
        setState('ready');
      })
      .catch((err) => {
        if (cancelled) return;
        console.error('[ProseView] getBlocks failed:', err);
        setState('error');
      });
    return () => {
      cancelled = true;
    };
  }, [pagePath]);

  // Hydrate Mermaid placeholders and asset images once the prose DOM is committed.
  useEffect(() => {
    if (state !== 'ready') return;
    const container = containerRef.current;
    if (!container) return;
    container.querySelectorAll<HTMLElement>('[data-mermaid-code]').forEach((el, i) => {
      const code = el.getAttribute('data-mermaid-code') || '';
      renderMermaidSvg(code, `prose-mermaid-${i}-${Date.now()}`)
        .then((svg) => {
          el.innerHTML = svg;
        })
        .catch(() => {
          el.innerHTML = '<pre class="bn-prose-mermaid-error"></pre>';
          const pre = el.querySelector('pre');
          if (pre) pre.textContent = code;
        });
    });
    container.querySelectorAll<HTMLImageElement>('[data-asset-ref]').forEach((img) => {
      const ref = img.getAttribute('data-asset-ref') || '';
      api
        .readAsset(ref)
        .then((bytes) => {
          if (bytes?.data_base64) img.src = `data:${bytes.mime};base64,${bytes.data_base64}`;
        })
        .catch(() => {
          // Leave the image broken; the reference path stays intact.
        });
    });
  }, [state, html]);

  const onContainerClick = useCallback(
    async (e: React.MouseEvent) => {
      const targetEl = (e.target as HTMLElement).closest<HTMLElement>('[data-prose-target]');
      if (targetEl) {
        e.preventDefault();
        const target = decodeURIComponent(targetEl.getAttribute('data-prose-target') || '');
        try {
          const resolved = await api.resolveLinkTarget(target);
          if (resolved.page_path) navigate('/page/' + encodeURIComponent(resolved.page_path));
        } catch {
          // Dead link — stay put.
        }
        return;
      }
      const tagEl = (e.target as HTMLElement).closest<HTMLElement>('[data-prose-tag]');
      if (tagEl) {
        e.preventDefault();
        const tag = tagEl.getAttribute('data-prose-tag') || '';
        navigate('/search?q=' + encodeURIComponent('#' + tag));
      }
    },
    [navigate],
  );

  const onContainerDoubleClick = useCallback(
    (e: React.MouseEvent) => {
      const blockEl = (e.target as HTMLElement).closest<HTMLElement>('.bn-prose-block');
      const id = blockEl?.getAttribute('data-block-id');
      if (id) onEditBlock(id);
    },
    [onEditBlock],
  );

  if (state === 'loading') {
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: 256 }}>
        <CircularProgress size={20} />
      </Box>
    );
  }
  if (state === 'error') {
    return (
      <Box sx={{ p: 2 }}>
        <Typography variant="body2" color="error.main">
          {t('common.loading')}
        </Typography>
      </Box>
    );
  }
  if (state === 'empty') {
    return (
      <Box sx={{ p: 2 }}>
        <Typography variant="body2" color="text.secondary">
          {t('page.prose_empty')}
        </Typography>
      </Box>
    );
  }
  return (
    <Box
      ref={containerRef}
      className="bn-prose"
      sx={{ px: 3, py: 2 }}
      dangerouslySetInnerHTML={{ __html: html }}
      onClick={onContainerClick}
      onDoubleClick={onContainerDoubleClick}
    />
  );
}
