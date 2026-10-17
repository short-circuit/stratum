import type { PageDto } from '../../lib/types';

/**
 * Pure decision: which wiki-link target to insert for a chosen page.
 * Pages whose basename slug is shared by 2+ pages cannot be disambiguated by a
 * bare `[[slug]]` (ambiguous → resolves to nothing); the vault-relative path
 * (minus `.md`) is unambiguous and must be used instead.
 */
export function computeWikiLinkTarget(
  page: PageDto,
  slugCounts: Map<string, number>,
): string {
  const slug = (page.slug || page.title || '').trim();
  if (!slug) return '';
  const isDuplicate = (slugCounts.get(slug) ?? 0) > 1;
  return isDuplicate ? (page.path || '').replace(/\.md$/, '') : slug;
}
