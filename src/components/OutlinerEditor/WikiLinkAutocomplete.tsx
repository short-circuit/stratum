import { useCallback } from 'react';
import { useBlockNoteEditor } from '@blocknote/react';
import { SuggestionMenuController, type DefaultReactSuggestionItem } from '@blocknote/react';
import { filterSuggestionItems } from '@blocknote/core';
import * as api from '../../lib/commands';
import type { PageDto } from '../../lib/types';

interface Props {
  pagePath: string;
}

/**
 * Wiki-link autocomplete: type `[[` in the editor to search the vault's pages
 * and insert a `[[slug]]` wiki-link at the cursor. Existing pages are listed
 * from the backend; the chosen page is inserted as a `stratum:<slug>` link so
 * the on-disk `[[slug]]` round-trips byte-stable with the parser/serializer.
 */
export default function WikiLinkAutocomplete({ pagePath }: Props) {
  const editor = useBlockNoteEditor();

  const getItems = useCallback(
    async (query: string): Promise<DefaultReactSuggestionItem[]> => {
      let pages: PageDto[];
      try {
        const res = await api.listPages();
        pages = res.pages || [];
      } catch (e) {
        console.error('[WikiLinkAutocomplete] listPages failed:', e);
        return [];
      }
      // Exclude the page currently being edited (self-links add no value).
      if (pagePath) {
        pages = pages.filter((p) => p.path !== pagePath);
      }
      const q = query.trim().toLowerCase();
      const items: DefaultReactSuggestionItem[] = pages
        .filter((p) => {
          if (!q) return true;
          const slug = (p.slug || '').toLowerCase();
          const title = (p.title || '').toLowerCase();
          return slug.includes(q) || title.includes(q);
        })
        .slice(0, 30)
        .map((p) => {
          const display = p.title || p.slug;
          return {
            title: display,
            subtext: p.path,
            aliases: [p.slug, p.title ?? ''].filter(Boolean),
            group: 'Notes',
            icon: (
              <span style={{ fontSize: '0.75rem', opacity: 0.6 }}>🔗</span>
            ),
            onItemClick: () => {
              try {
                const slug = (p.slug || p.title || '').trim();
                if (!slug) return;
                editor.createLink('stratum:' + slug, display);
                editor.focus();
              } catch (e) {
                console.error('[WikiLinkAutocomplete] insert link failed:', e);
              }
            },
          };
        });
      return filterSuggestionItems(items, query);
    },
    [editor, pagePath],
  );

  return (
    <SuggestionMenuController
      triggerCharacter={'[['}
      getItems={getItems}
      minQueryLength={0}
    />
  );
}
