import { describe, it, expect } from 'vitest';
import {
  parseContentToInlineItems,
  inlineItemsToContent,
  normalizeContent,
  isWikiLinkHref,
  extractWikiLinkTarget,
} from './wikiLinks';
import type { InlineItem } from './wikiLinks';

/**
 * Round-trip tests for the wikilink parser/serializer.
 *
 * Regression coverage for the acceptance CRITICAL defect (ED-07 / FF-04 /
 * LK-03): the parser previously used the full `[[...]]` match (capture group 9)
 * as the link target instead of the inner target (group 10), so every wikilink
 * round-tripped into a doubled bracket form `[[[[Target]]]]`. Keeping content
 * byte-stable across parse → serialize (the editor's save path) is the
 * invariant that prevents re-corruption of vault files.
 */

/** Parse then serialize; the result must be byte-identical to the input. */
function roundTrip(input: string): string {
  const items = parseContentToInlineItems(input);
  return inlineItemsToContent(items);
}

describe('wikilink round-trip stability (ED-07 / FF-04 regression)', () => {
  it('keeps a plain `[[Target]]` intact', () => {
    const cases = ['[[Alpha Project]]', '[[todo]]', '[[research/notes]]'];
    for (const c of cases) {
      expect(roundTrip(c)).toBe(c);
    }
  });

  it('keeps an embedded `{{embed [[Target]]}}` intact (no doubled brackets)', () => {
    const src = '{{embed [[Alpha Project]]}}';
    expect(roundTrip(src)).toBe(src);
  });

  it('keeps half-bracket embeds from legacy corruption intact', () => {
    // The bug produced these; once normalized they must be stable, not re-wrapped.
    const src = '{{embed [[[[Alpha Project]]]]}}';
    expect(normalizeContent(src)).toBe('{{embed [[Alpha Project]]}}');
  });

  it('keeps piped `[[Target|display]]` intact', () => {
    const cases = ['[[Alpha Project|see notes]]', '[[todo|Task list]]'];
    for (const c of cases) {
      expect(roundTrip(c)).toBe(c);
    }
  });

  it('keeps inline text around links intact (sentences, commas, parens)', () => {
    const src = 'This block references ((00000000-0000-0000-0000-000000000001)). {{embed [[Alpha Project]]}}';
    expect(roundTrip(src)).toBe(src);
  });

  it('keeps tags intact and stable', () => {
    const cases = ['#project', 'see #rust for notes', 'a#b stays literal'];
    for (const c of cases) {
      expect(roundTrip(c)).toBe(c);
    }
  });

  it('keeps markdown emphasis intact around text', () => {
    const cases = ['**bold**', '*italic*', '~~done~~', '`code`'];
    for (const c of cases) {
      expect(roundTrip(c)).toBe(c);
    }
  });

  it('produces a stratum: href with the bare target (no brackets) for links', () => {
    const items = parseContentToInlineItems('[[Alpha Project]]');
    const link = items.find((i) => i.type === 'link') as Extract<typeof items[number], { type: 'link' }>;
    expect(link).toBeDefined();
    expect(isWikiLinkHref(link.href)).toBe(true);
    expect(extractWikiLinkTarget(link.href)).toBe('Alpha Project');
  });

  it('normalizes doubled-bracket legacy corruption to single brackets', () => {
    expect(normalizeContent('[[[[Alpha Project]]]]')).toBe('[[Alpha Project]]');
    expect(normalizeContent('[[[[Alpha Project|display]]|display]]')).toBe('[[Alpha Project|display]]');
  });

  it('keeps un-normalized doubled-bracket input byte-stable through a round trip', () => {
    // Defense-in-depth: even if corrupted `[[[[T]]]]` somehow reaches the
    // serializer without first passing through normalizeContent, parsing must
    // not re-wrap it into `[[T|[[T]]]]` (the previous bug).
    expect(inlineItemsToContent(parseContentToInlineItems('[[[[Alpha Project]]]]'))).toBe(
      '[[[[Alpha Project]]]]',
    );
  });

  it('is idempotent: a normalized string does not change on second normalize', () => {
    const once = normalizeContent('{{embed [[[[Alpha Project]]]]}}');
    expect(normalizeContent(once)).toBe(once);
  });

  it('round-trips the full alpha-project fixture block content byte-identically', () => {
    const blocks = [
      'A TODO ship MVP',
      'B DOING write docs',
      'This block references ((00000000-0000-0000-0000-000000000001)).',
      '{{embed [[Alpha Project]]}}',
      'What is a monad?',
      '## Math',
    ];
    for (const b of blocks) {
      expect(roundTrip(b)).toBe(b);
    }
  });
});

/**
 * E7.F1 — wiki-link AUTCOMPLETE insert contract.
 *
 * The editor's `[[` autocomplete (WikiLinkAutocomplete) inserts a link by
 * calling `editor.createLink('stratum:' + slug, display)`. Whatever the editor
 * produces as an inline link item, the serializer must write a byte-stable,
 * resolvable `[[slug]]` (or `[[slug|display]]`) to disk — never a doubled or
 * mangled form. These tests pin that the href shape the autocomplete constructs
 * (kebab slug from `list_page`'s `file_stem`, pretty title as display) round-
 * trips exactly to what the linker/navigation resolve (see resolve_link_target).
 */
describe('wiki-link autocomplete insert contract (E7.F1)', () => {
  it('serializes a stratum:slug href created by createLink to [[slug]]', () => {
    // `stratum:` + kebab slug is exactly what WikiLinkAutocomplete builds.
    const items: InlineItem[] = [
      { type: 'text', text: 'See ', styles: {} },
      {
        type: 'link',
        href: 'stratum:alpha-project',
        content: [{ type: 'text', text: 'Alpha Project', styles: {} }],
      },
      { type: 'text', text: ' for details.', styles: {} },
    ];
    expect(inlineItemsToContent(items)).toBe('See [[alpha-project|Alpha Project]] for details.');
  });

  it('serializes an untitled target (display === slug) to a bare [[slug]]', () => {
    // When no title exists the autocomplete uses the slug as display; the
    // serializer must collapse the duplicate into the clean form.
    const items: InlineItem[] = [
      {
        type: 'link',
        href: 'stratum:beta-notes',
        content: [{ type: 'text', text: 'beta-notes', styles: {} }],
      },
    ];
    expect(inlineItemsToContent(items)).toBe('[[beta-notes]]');
  });

  it('round-trips an autocomplete-inserted link byte-identically after a parse', () => {
    const src = 'See [[alpha-project|Alpha Project]] for details.';
    expect(roundTrip(src)).toBe(src);
  });

  it('keeps a no-display link stable (lowercase-slug target matches navigation)', () => {
    // resolve_link_target lowercases + dashifies the target, so an autocomplete
    // insert must stay resolvable across parse→serialize.
    const items: InlineItem[] = [
      {
        type: 'link',
        href: 'stratum:alpha-project',
        content: [{ type: 'text', text: 'alpha-project', styles: {} }],
      },
    ];
    const serialized = inlineItemsToContent(items);
    expect(serialized).toBe('[[alpha-project]]');
    expect(roundTrip(serialized)).toBe(serialized);
  });
});
