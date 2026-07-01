import { useEffect } from 'react';
import { Plugin, PluginKey, type EditorState } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';
import { type Node } from 'prosemirror-model';
import katex from 'katex';

const MATH_RE = /\$([^$\n]+?)\$/g;

function scanMath(doc: Node): DecorationSet {
  const decorations: Decoration[] = [];
  doc.descendants((node, pos) => {
    if (!node.isText || !node.text) return;
    const text = node.text;
    let match: RegExpExecArray | null;
    MATH_RE.lastIndex = 0;
    while ((match = MATH_RE.exec(text)) !== null) {
      const from = pos + match.index;
      const to = from + match[0].length;
      const latex = match[1];
      let html: string;
      try {
        html = katex.renderToString(latex, { displayMode: false, throwOnError: true });
      } catch {
        continue;
      }
      const wrapper = document.createElement('span');
      wrapper.className = 'math-inline-rendered';
      wrapper.innerHTML = html;
      wrapper.dataset.latex = latex;
      wrapper.dataset.pos = String(from);
      wrapper.contentEditable = 'false';
      decorations.push(Decoration.widget(from, () => wrapper, { side: 1 }));
      decorations.push(Decoration.inline(from, to, { style: 'display: none;' }));
    }
  });
  return DecorationSet.create(doc, decorations);
}

const KEY = new PluginKey('math-inline');

function createPlugin() {
  return new Plugin({
    key: KEY,
    state: {
      init(_config, instance) {
        return scanMath(instance.doc);
      },
      apply(tr, _set: DecorationSet) {
        if (!tr.docChanged) return _set.map(tr.mapping, tr.doc);
        return scanMath(tr.doc);
      },
    },
    props: {
      decorations(state) {
        return KEY.getState(state) as DecorationSet | undefined;
      },
    },
  });
}

export function useMathInline(editor: unknown, enabled: boolean) {
  useEffect(() => {
    if (!enabled || !editor) return;

    const tryAdd = setInterval(() => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const te = (editor as any)._tiptapEditor as
        | { registerPlugin?: (p: Plugin) => void; isDestroyed?: boolean; state: unknown }
        | undefined;
      if (!te || te.isDestroyed) return;

      // Check if plugin already installed
      if (KEY.get(te.state as EditorState)) return;

      clearInterval(tryAdd);

      try {
        te.registerPlugin!(createPlugin());
      } catch (e) {
        console.error('[MathInline] registerPlugin failed:', e);
      }
    }, 50);

    return () => {
      clearInterval(tryAdd);
    };
  }, [editor, enabled]);
}

export function setupMathDblClick(
  container: HTMLElement | null,
  onEdit: (latex: string, pos: number) => void,
) {
  if (!container) return;

  const handler = (e: MouseEvent) => {
    if (e.detail !== 2) return;
    const target = (e.target as HTMLElement).closest('.math-inline-rendered') as HTMLElement | null;
    if (!target || !target.dataset.latex) return;
    e.preventDefault();
    e.stopPropagation();
    onEdit(target.dataset.latex, parseInt(target.dataset.pos || '0'));
  };

  container.addEventListener('dblclick', handler);
  return () => container.removeEventListener('dblclick', handler);
}
