import { useEffect } from 'react';
import { Plugin, PluginKey, type EditorState } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';
import type { Node } from 'prosemirror-model';

const MARKER_COLORS: Record<string, { bg: string; text: string }> = {
  TODO: { bg: '#f59e0b', text: '#fff' },
  DOING: { bg: '#3b82f6', text: '#fff' },
  DONE: { bg: '#10b981', text: '#fff' },
  NOW: { bg: '#8b5cf6', text: '#fff' },
  LATER: { bg: '#f97316', text: '#fff' },
  WAITING: { bg: '#ec4899', text: '#fff' },
  CANCELLED: { bg: '#6b7280', text: '#fff' },
};

const PRIORITY_COLORS: Record<string, { bg: string; text: string }> = {
  A: { bg: '#ef4444', text: '#fff' },
  B: { bg: '#f59e0b', text: '#fff' },
  C: { bg: '#3b82f6', text: '#fff' },
};

export interface BlockMeta {
  marker: string | null;
  priority: string | null;
  properties: [string, string][];
}

const KEY = new PluginKey('marker-decorations');

function createMarkerWidget(
  blockId: string,
  marker: string | null,
  priority: string | null,
  onToggle: (id: string) => void,
  onClear: (id: string) => void,
): HTMLElement {
  const wrapper = document.createElement('span');
  wrapper.className = 'marker-decoration-widget';
  wrapper.style.cssText = [
    'display: inline-flex',
    'align-items: center',
    'gap: 3px',
    'margin-right: 2px',
    'user-select: none',
    'vertical-align: middle',
    'position: relative',
  ].join('; ');

  if (marker) {
    const colors = MARKER_COLORS[marker] ?? { bg: '#6b7280', text: '#fff' };
    const badge = document.createElement('span');
    badge.textContent = marker;
    badge.style.cssText = [
      'display: inline-block',
      'padding: 0 5px',
      'height: 18px',
      'line-height: 18px',
      'border-radius: 4px',
      `background-color: ${colors.bg}`,
      `color: ${colors.text}`,
      'font-size: 0.6rem',
      'font-weight: 700',
      'letter-spacing: 0.02em',
      'cursor: pointer',
    ].join('; ');
    badge.title = 'Click to cycle marker state';
    wrapper.appendChild(badge);

    const clearBtn = document.createElement('span');
    clearBtn.textContent = '\u00d7';
    clearBtn.style.cssText = [
      'display: inline-flex',
      'align-items: center',
      'justify-content: center',
      'width: 14px',
      'height: 14px',
      'line-height: 14px',
      'font-size: 0.6rem',
      'font-weight: 700',
      'cursor: pointer',
      'color: #6b7280',
      'border-radius: 50%',
      'margin-left: 1px',
    ].join('; ');
    clearBtn.title = 'Remove marker';
    wrapper.appendChild(clearBtn);

    badge.addEventListener('mousedown', (e) => {
      e.preventDefault();
      e.stopPropagation();
      onToggle(blockId);
    });

    clearBtn.addEventListener('mousedown', (e) => {
      e.preventDefault();
      e.stopPropagation();
      onClear(blockId);
    });
  } else if (priority) {
    const colors = PRIORITY_COLORS[priority] ?? { bg: '#6b7280', text: '#fff' };
    const badge = document.createElement('span');
    badge.textContent = priority;
    badge.style.cssText = [
      'display: inline-block',
      'padding: 0 5px',
      'height: 18px',
      'line-height: 18px',
      'border-radius: 4px',
      `background-color: ${colors.bg}`,
      `color: ${colors.text}`,
      'font-size: 0.6rem',
      'font-weight: 700',
      'letter-spacing: 0.02em',
      'cursor: default',
    ].join('; ');
    wrapper.appendChild(badge);

    const clearBtn = document.createElement('span');
    clearBtn.textContent = '\u00d7';
    clearBtn.style.cssText = [
      'display: inline-flex',
      'align-items: center',
      'justify-content: center',
      'width: 14px',
      'height: 14px',
      'line-height: 14px',
      'font-size: 0.6rem',
      'font-weight: 700',
      'cursor: pointer',
      'color: #6b7280',
      'border-radius: 50%',
      'margin-left: 1px',
    ].join('; ');
    clearBtn.title = 'Remove priority';
    wrapper.appendChild(clearBtn);

    clearBtn.addEventListener('mousedown', (e) => {
      e.preventDefault();
      e.stopPropagation();
      onClear(blockId);
    });
  }

  return wrapper;
}

function scanMarkers(
  doc: Node,
  blockMetaRef: React.MutableRefObject<Map<string, BlockMeta>>,
  onToggle: (id: string) => void,
  onClear: (id: string) => void,
): DecorationSet {
  const decorations: Decoration[] = [];
  const blockMetaMap = blockMetaRef.current;

  doc.descendants((node: Node, pos: number) => {
    const nodeId = node.attrs?.id as string | undefined;
    if (nodeId && blockMetaMap.has(nodeId)) {
      const meta = blockMetaMap.get(nodeId)!;
      if (meta.marker || meta.priority) {
        const widget = createMarkerWidget(nodeId, meta.marker, meta.priority, onToggle, onClear);
        decorations.push(Decoration.widget(pos, () => widget, { side: -1 }));
      }
    }
  });

  return DecorationSet.create(doc, decorations);
}

function createPlugin(
  blockMetaRef: React.MutableRefObject<Map<string, BlockMeta>>,
  onToggle: (id: string) => void,
  onClear: (id: string) => void,
): Plugin {
  return new Plugin({
    key: KEY,
    state: {
      init(_config, instance: { doc: Node }) {
        return scanMarkers(instance.doc, blockMetaRef, onToggle, onClear);
      },
      apply(tr, oldSet: DecorationSet, _oldState, newState) {
        if (!tr.docChanged && !tr.getMeta('marker-refresh')) {
          return oldSet.map(tr.mapping, tr.doc);
        }
        return scanMarkers(newState.doc, blockMetaRef, onToggle, onClear);
      },
    },
    props: {
      decorations(state: EditorState): DecorationSet | undefined {
        return KEY.getState(state) as DecorationSet | undefined;
      },
    },
  });
}

/**
 * Force a refresh of marker decorations without changing the document.
 * Call this after updating blockMetaRef from toggle/clear operations.
 */
export function refreshMarkerDecorations(editor: unknown): void {
  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const te = (editor as any)._tiptapEditor as
      | { view?: { dispatch?: (tr: import('prosemirror-state').Transaction) => void }; state?: import('prosemirror-state').EditorState; isDestroyed?: boolean }
      | undefined;
    if (te && !te.isDestroyed && te.view?.dispatch && te.state) {
      te.view.dispatch(te.state.tr.setMeta('marker-refresh', true));
    }
  } catch {
    // silently fail — editor might not be ready
  }
}

/**
 * React hook that registers a ProseMirror decoration plugin for inline marker/priority badges.
 * Follows the same pattern as useMathInline.tsx.
 */
export function useMarkerDecorations(
  editor: unknown,
  enabled: boolean,
  blockMetaRef: React.MutableRefObject<Map<string, BlockMeta>>,
  onToggle: (id: string) => void,
  onClear: (id: string) => void,
): void {
  useEffect(() => {
    if (!enabled || !editor) return;

    const tryAdd = setInterval(() => {
      try {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const te = (editor as any)._tiptapEditor as
          | { registerPlugin?: (p: Plugin) => void; isDestroyed?: boolean; state: import('prosemirror-state').EditorState }
          | undefined;
        if (!te || te.isDestroyed) return;

        // Skip if already installed
        if (KEY.get(te.state)) {
          clearInterval(tryAdd);
          return;
        }

        clearInterval(tryAdd);
        te.registerPlugin!(createPlugin(blockMetaRef, onToggle, onClear));
      } catch (e) {
        // Plugin registration may fail if editor not fully initialized
        console.error('[MarkerDecorations] registerPlugin failed:', e);
      }
    }, 50);

    return () => {
      clearInterval(tryAdd);
    };
  }, [editor, enabled, blockMetaRef, onToggle, onClear]);
}
