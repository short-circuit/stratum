import { useCallback, useMemo } from 'react';
import { useBlockNoteEditor } from '@blocknote/react';
import { SuggestionMenuController, type DefaultReactSuggestionItem } from '@blocknote/react';
import { filterSuggestionItems } from '@blocknote/core';
import type { BlockMeta } from './dtoConverters';
import { MARKER_KEYWORDS } from './markerDetection';

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

interface Props {
  blockMetaRef: React.MutableRefObject<Map<string, BlockMeta>>;
  onSelect: () => void;
}

export default function MarkerSuggestMenu({ blockMetaRef, onSelect }: Props) {
  const editor = useBlockNoteEditor();

  const suggestionItems = useMemo((): DefaultReactSuggestionItem[] => {
    const markerItems: DefaultReactSuggestionItem[] = MARKER_KEYWORDS.map(m => {
      const colors = MARKER_COLORS[m] ?? { bg: '#6b7280', text: '#fff' };
      return {
        title: m,
        subtext: 'Task marker',
        aliases: [m.toLowerCase()],
        group: 'Markers',
        icon: (
          <span
            style={{
              display: 'inline-block',
              width: 32,
              height: 16,
              borderRadius: 3,
              backgroundColor: colors.bg,
              color: colors.text,
              fontSize: '0.55rem',
              fontWeight: 700,
              lineHeight: '16px',
              textAlign: 'center',
              letterSpacing: '0.02em',
            }}
          >
            {m}
          </span>
        ),
        onItemClick: () => {
          try {
            const pos = editor.getTextCursorPosition();
            const blockId = (pos.block as any).id as string;
            if (!blockId) return;
            const existing = blockMetaRef.current.get(blockId) ?? { marker: null, priority: null, properties: [] as [string, string][] };
            blockMetaRef.current.set(blockId, { ...existing, marker: m });
            onSelect();
          } catch (e) {
            console.error('[MarkerSuggest] set marker failed:', e);
          }
        },
      };
    });

    const priorityItems: DefaultReactSuggestionItem[] = ['A', 'B', 'C'].map(p => {
      const colors = PRIORITY_COLORS[p];
      const subtext = p === 'A' ? 'High priority' : p === 'B' ? 'Medium priority' : 'Low priority';
      return {
        title: `Priority ${p}`,
        subtext,
        aliases: [`${p.toLowerCase()}`, `priority ${p.toLowerCase()}`],
        group: 'Priority',
        icon: (
          <span
            style={{
              display: 'inline-block',
              width: 32,
              height: 16,
              borderRadius: 3,
              backgroundColor: colors.bg,
              color: colors.text,
              fontSize: '0.55rem',
              fontWeight: 700,
              lineHeight: '16px',
              textAlign: 'center',
            }}
          >
            {p}
          </span>
        ),
        onItemClick: () => {
          try {
            const pos = editor.getTextCursorPosition();
            const blockId = (pos.block as any).id as string;
            if (!blockId) return;
            // eslint-disable-next-line react-hooks/refs
            const existing = blockMetaRef.current.get(blockId) ?? { marker: null, priority: null, properties: [] as [string, string][] };
            blockMetaRef.current.set(blockId, { ...existing, priority: p });
            onSelect();
          } catch (e) {
            console.error('[MarkerSuggest] set priority failed:', e);
          }
        },
      };
    });

    return [...markerItems, ...priorityItems];
  }, [editor, blockMetaRef, onSelect]);

  const getItems = useCallback(
    async (query: string): Promise<DefaultReactSuggestionItem[]> => {
      // Only show when : typed at block start (cursor offset <= 1 after consuming :)
      try {
        const pos = editor.getTextCursorPosition();
        // offset=1 means : was the first character (now consumed)
        if ((pos as any).offset > 1) return [];
      } catch {
        return [];
      }
      return filterSuggestionItems(suggestionItems, query);
    },
    [editor, suggestionItems],
  );

  return (
    <SuggestionMenuController
      triggerCharacter=":"
      getItems={getItems}
      minQueryLength={0}
    />
  );
}
