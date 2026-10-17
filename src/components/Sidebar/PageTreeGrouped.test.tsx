import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import PageTreeGrouped from './PageTreeGrouped';
import type { PageDto } from '../../lib/types';

const PAGE_A: PageDto = {
  path: 'pages/a.md',
  slug: 'a',
  title: 'Alpha',
  block_count: 2,
  modified_at: '2026-09-22T10:00:00Z',
};
const PAGE_B: PageDto = {
  path: 'pages/projects/b.md',
  slug: 'b',
  title: 'Beta',
  block_count: 1,
  modified_at: '2026-09-22T11:00:00Z',
};
const PAGE_C: PageDto = {
  path: 'pages/projects/sub/c.md',
  slug: 'c',
  title: 'Gamma',
  block_count: 0,
  modified_at: '2026-09-22T12:00:00Z',
};

function renderGrouped(pages: PageDto[], overrides: Record<string, unknown> = {}) {
  const props = {
    pages,
    collapsed: false,
    showNew: false,
    newPath: '',
    newTitle: '',
    onShowNewChange: vi.fn(),
    onNewPathChange: vi.fn(),
    onNewTitleChange: vi.fn(),
    onCreatePage: vi.fn(),
    onDeletePage: vi.fn(),
    onNavigate: vi.fn(),
    onNavigateHome: vi.fn(),
    ...overrides,
  };
  const result = render(<PageTreeGrouped {...props} />);
  return { props, ...result };
}

describe('PageTreeGrouped — folder-tree page list', () => {
  it('renders the "Recent" header and the new-page form controls', () => {
    renderGrouped([PAGE_A], { showNew: true });
    expect(screen.getByText('Recent')).toBeInTheDocument();
    expect(screen.getByText('Create')).toBeInTheDocument();
  });

  it('renders folder nodes and leaves (all expanded by default)', () => {
    renderGrouped([PAGE_A, PAGE_B, PAGE_C]);
    expect(screen.getByText('projects')).toBeInTheDocument();
    expect(screen.getByText('Alpha')).toBeInTheDocument();
    expect(screen.getByText('Beta')).toBeInTheDocument();
    expect(screen.getByText('Gamma')).toBeInTheDocument();
  });

  it('navigates to the encoded page URL when a leaf is clicked', () => {
    const { props } = renderGrouped([PAGE_A, PAGE_B, PAGE_C]);
    fireEvent.click(screen.getByText('Alpha'));
    expect(props.onNavigate).toHaveBeenCalledWith('/page/pages%2Fa.md');
  });

  it('shows a delete confirmation for a nested leaf, then deletes on confirm', () => {
    const { props } = renderGrouped([PAGE_A, PAGE_B, PAGE_C]);
    const deleteBtn = screen.getByText('Beta')
      .closest('.MuiListItemButton-root')!
      .querySelector('.delete-btn') as HTMLElement;
    fireEvent.click(deleteBtn);
    expect(screen.getByText(/Delete pages\/projects\/b\.md/)).toBeInTheDocument();
    expect(props.onDeletePage).not.toHaveBeenCalled();
    fireEvent.click(screen.getAllByText('Delete')[0]);
    expect(props.onDeletePage).toHaveBeenCalledWith('pages/projects/b.md');
  });

  it('collapses and expands a folder on row click', () => {
    renderGrouped([PAGE_A, PAGE_B, PAGE_C]);
    expect(screen.getByText('Beta')).toBeInTheDocument();
    expect(screen.getByText('Gamma')).toBeInTheDocument();

    fireEvent.click(screen.getByText('projects'));
    expect(screen.queryByText('Beta')).not.toBeInTheDocument();
    expect(screen.queryByText('Gamma')).not.toBeInTheDocument();

    fireEvent.click(screen.getByText('projects'));
    expect(screen.getByText('Beta')).toBeInTheDocument();
    expect(screen.getByText('Gamma')).toBeInTheDocument();
  });

  it('shows the empty state when there are no pages or only journal pages', () => {
    renderGrouped([]);
    expect(screen.getByText('No pages yet.')).toBeInTheDocument();
  });

  it('drops journal-only pages from the tree', () => {
    renderGrouped([
      { path: 'journals/2026-10-14.md', slug: 'j', title: 'Journal', block_count: 0, modified_at: '2026-10-14T00:00:00Z' },
    ]);
    expect(screen.getByText('No pages yet.')).toBeInTheDocument();
  });
});
