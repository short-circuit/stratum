import { describe, it, expect } from 'vitest';
import { computeWikiLinkTarget } from './wikiLinkTargets';
import type { PageDto } from '../../lib/types';

function makePage(path: string, slug: string, title: string): PageDto {
  return {
    path,
    slug,
    title,
    block_count: 0,
    modified_at: '',
  };
}

describe('computeWikiLinkTarget (duplicate-slug disambiguation)', () => {
  it('uses the bare slug when it is unique', () => {
    const counts = new Map<string, number>([['unique', 1]]);
    const target = computeWikiLinkTarget(makePage('pages/unique.md', 'unique', 'Unique'), counts);
    expect(target).toBe('unique');
  });

  it('uses the path (minus .md) when the slug is duplicated', () => {
    const counts = new Map<string, number>([['note', 2]]);
    const target = computeWikiLinkTarget(makePage('pages/a/note.md', 'note', 'Note A'), counts);
    expect(target).toBe('pages/a/note');
  });

  it('path-qualified target drops the .md extension', () => {
    const counts = new Map<string, number>([['note', 2]]);
    const target = computeWikiLinkTarget(makePage('pages/b/note.md', 'note', 'Note B'), counts);
    expect(target).toBe('pages/b/note');
    expect(target.endsWith('.md')).toBe(false);
  });

  it('returns empty when the page has no slug or title', () => {
    const counts = new Map<string, number>();
    const target = computeWikiLinkTarget(makePage('pages/x.md', '', ''), counts);
    expect(target).toBe('');
  });
});
