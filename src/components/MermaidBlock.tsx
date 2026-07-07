import { createBlockSpec } from '@blocknote/core';
import mermaid from 'mermaid';

let mermaidInitialized = false;

function ensureMermaid() {
  if (mermaidInitialized) return;
  const dark = typeof document !== 'undefined' && document.documentElement.classList.contains('dark');
  mermaid.initialize({
    startOnLoad: false,
    theme: dark ? 'dark' : 'default',
    securityLevel: 'loose',
    fontFamily: 'sans-serif',
  });
  mermaidInitialized = true;
}

export const createMermaidSpec = createBlockSpec(
  {
    type: 'mermaid' as const,
    propSchema: {
      language: { default: 'mermaid' },
    },
    content: 'inline' as const,
  },
  {
    runsBefore: ['codeBlock'],
    meta: {
      code: true,
    },
    render: function(block, editor) {
      const container = document.createElement('div');
      container.style.cssText = 'width:100%;';

      // contentDOM — ProseMirror edits the code text here
      const contentDOM = document.createElement('div');
      contentDOM.style.cssText = 'font-family:monospace;font-size:13px;padding:8px;box-sizing:border-box;min-height:80px;';

      // diagram wrapper
      const diagramEl = document.createElement('div');
      diagramEl.style.cssText = 'width:100%;overflow:hidden;padding:8px 0;cursor:grab;';

      // Put contentDOM first so it determines container height when visible.
      // diagramEl comes second so it appears below contentDOM.
      container.appendChild(contentDOM);
      container.appendChild(diagramEl);

      let showingDiagram = true;
      let scale = 1;
      let offsetX = 0;
      let offsetY = 0;
      let isDragging = false;
      let dragStartX = 0;
      let dragStartY = 0;
      let offsetStartX = 0;
      let offsetStartY = 0;
      let scheduledRender: number | null = null;

      function getCode(): string {
        return contentDOM.textContent || '';
      }

      function applyTransform() {
        const w = diagramEl.firstElementChild as HTMLElement | null;
        if (w) {
          w.style.transform = `scale(${scale}) translate(${offsetX / scale}px, ${offsetY / scale}px)`;
          w.style.transformOrigin = '0 0';
        }
      }

      function switchToView() {
        if (showingDiagram) return;
        showingDiagram = true;
        contentDOM.style.display = 'none';
        diagramEl.style.display = '';
        renderDiagram();
        container.style.minHeight = '';
      }

      function switchToEdit() {
        if (!showingDiagram) return;
        showingDiagram = false;
        diagramEl.style.display = 'none';
        contentDOM.style.display = '';
        container.style.minHeight = '80px';
        setTimeout(() => editor.focus(), 0);
      }

      function renderDiagram() {
        ensureMermaid();
        const code = getCode();
        if (!code.trim()) {
          diagramEl.innerHTML = '<div style="padding:16px;text-align:center;color:var(--secondary-500);font-size:13px;">Empty mermaid diagram</div>';
          return;
        }
        const renderId = `mermaid-${block.id.replace(/[^a-zA-Z0-9-]/g, '')}-${Date.now()}`;
        mermaid.render(renderId, code)
          .then(({ svg }) => {
            diagramEl.innerHTML = '';
            const wrapper = document.createElement('div');
            wrapper.style.cssText = 'pointer-events:none;transform-origin:0 0;';
            wrapper.style.transform = `scale(${scale}) translate(${offsetX / scale}px, ${offsetY / scale}px)`;
            wrapper.innerHTML = svg;
            diagramEl.appendChild(wrapper);
            // Let the container grow to fit the diagram
            diagramEl.style.display = '';
          })
          .catch((err: Error) => {
            diagramEl.innerHTML = `<div style="padding:12px;border:1px solid #f87171;border-radius:6px;background:var(--bg-secondary);"><div style="color:#ef4444;font-size:13px;margin-bottom:8px;">Mermaid error: ${err.message}</div><pre style="font-size:12px;white-space:pre-wrap;font-family:monospace;color:var(--text-primary);">${code}</pre></div>`;
          });
      }

      // Double-click diagram → edit mode
      container.addEventListener('dblclick', (e) => {
        e.stopPropagation();
        switchToEdit();
      });

      // Focus leaves the block → back to diagram view
      container.addEventListener('focusout', (e) => {
        if (container.contains(e.relatedTarget as Node)) return;
        switchToView();
      });

      // Click anywhere outside the container → back to view
      function onDocDown(e: MouseEvent) {
        if (showingDiagram) return;
        if (!container.contains(e.target as Node)) {
          switchToView();
        }
      }
      document.addEventListener('mousedown', onDocDown);

      // Ctrl+Enter in edit mode → save and switch to view
      contentDOM.addEventListener('keydown', (e) => {
        if (!showingDiagram && e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
          e.preventDefault();
          e.stopPropagation();
          switchToView();
        }
      });

      // Ctrl+wheel zoom
      diagramEl.addEventListener('wheel', (e) => {
        if (!e.ctrlKey && !e.metaKey) return;
        e.preventDefault();
        e.stopPropagation();
        scale = Math.max(0.1, Math.min(10, scale * (e.deltaY > 0 ? 0.9 : 1.1)));
        applyTransform();
      }, { passive: false });

      // Pan via drag
      diagramEl.addEventListener('mousedown', (e) => {
        isDragging = true;
        dragStartX = e.clientX;
        dragStartY = e.clientY;
        offsetStartX = offsetX;
        offsetStartY = offsetY;
        diagramEl.style.cursor = 'grabbing';
      });

      function onDocMove(e: MouseEvent) {
        if (!isDragging) return;
        offsetX = offsetStartX + (e.clientX - dragStartX);
        offsetY = offsetStartY + (e.clientY - dragStartY);
        applyTransform();
      }

      function onDocUp() {
        isDragging = false;
        diagramEl.style.cursor = 'grab';
      }

      document.addEventListener('mousemove', onDocMove);
      document.addEventListener('mouseup', onDocUp);
      document.addEventListener('mousedown', onDocDown);

      // Re-render when code changes
      // Auto-switch to diagram view once content appears
      const observer = new MutationObserver(() => {
        if (!showingDiagram && getCode().trim()) {
          switchToView();
          return;
        }
        if (scheduledRender) cancelAnimationFrame(scheduledRender);
        scheduledRender = requestAnimationFrame(() => {
          renderDiagram();
          scheduledRender = null;
        });
      });
      observer.observe(contentDOM, { characterData: true, childList: true, subtree: true });

      // Start in edit mode if empty, view mode if content exists
      const hasContent = !!getCode().trim();
      if (hasContent) {
        contentDOM.style.display = 'none';
        renderDiagram();
      } else {
        showingDiagram = false;
        diagramEl.style.display = 'none';
        contentDOM.style.display = '';
        container.style.minHeight = '80px';
      }

      return {
        dom: container,
        contentDOM,
        ignoreMutation: () => true,
        destroy: () => {
          observer.disconnect();
          document.removeEventListener('mousemove', onDocMove);
          document.removeEventListener('mouseup', onDocUp);
          document.removeEventListener('mousedown', onDocDown);
          if (scheduledRender) cancelAnimationFrame(scheduledRender);
        },
      };
    },
    parse: (el) => {
      const codeEl = el.querySelector('pre > code.language-mermaid');
      if (codeEl) {
        return { language: 'mermaid' };
      }
      return undefined;
    },
    toExternalHTML: (block) => {
      const wrapper = document.createElement('div');
      const pre = document.createElement('pre');
      const codeEl = document.createElement('code');
      codeEl.className = 'language-mermaid';
      const content = block.content;
      let text = '';
      if (typeof content === 'string') text = content;
      else if (Array.isArray(content)) text = content.map((c: any) => c?.text || '').join('');
      codeEl.textContent = text;
      pre.appendChild(codeEl);
      wrapper.appendChild(pre);
      return { dom: wrapper };
    },
  },
  () => ([
    {
      key: 'mermaid-input-rule',
      inputRules: [
        {
          find: /^```mermaid\s$/i,
          replace: () => ({
            type: 'mermaid' as const,
            props: { language: 'mermaid' },
          }),
        },
      ],
    },
  ]),
);
