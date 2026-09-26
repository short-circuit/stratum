import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import PageTree from './PageTree';
import type { PageDto } from '../../lib/types';

const PAGE_A: PageDto = {
  path: 'pages/a.md',
  slug: 'a',
  title: 'Alpha',
  block_count: 2,
  modified_at: '2026-09-22T10:00:00Z',
};
const PAGE_B: PageDto = {
  path: 'pages/b.md',
  slug: 'b',
  title: null,
  block_count: 0,
  modified_at: '2026-09-22T11:00:00Z',
};

function renderPageTree(pages: PageDto[]) {
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
  };
  const result = render(<PageTree {...props} />);
  return { props, ...result };
}

describe('PageTree — sidebar recents list consumer', () => {
  it('renders the "Recent" section header', () => {
    renderPageTree([PAGE_A]);
    expect(screen.getByText('Recent')).toBeInTheDocument();
  });

  it('renders each provided page (the recents-store-backed list)', () => {
    renderPageTree([PAGE_A, PAGE_B]);
    expect(screen.getByText('Alpha')).toBeInTheDocument();
    expect(screen.getByText('b')).toBeInTheDocument();
  });

  it('renders pages in the order received (store provides modified_at DESC)', () => {
    renderPageTree([PAGE_B, PAGE_A]);
    const titles = screen.getAllByRole('button').map(n => n.textContent ?? '');
    const alphaIdx = titles.findIndex(t => t.includes('Alpha'));
    const bIdx = titles.findIndex(t => t.includes('b'));
    expect(alphaIdx).toBeGreaterThan(bIdx);
  });

  it('navigates to the page path on click', () => {
    const { props } = renderPageTree([PAGE_A]);
    fireEvent.click(screen.getByText('Alpha'));
    expect(props.onNavigate).toHaveBeenCalledWith('/page/pages%2Fa.md');
  });

  it('shows empty state when the recents list is empty', () => {
    renderPageTree([]);
    expect(screen.getByText('No pages yet.')).toBeInTheDocument();
  });

  it('shows a delete confirmation when the delete button is clicked', () => {
    const { props } = renderPageTree([PAGE_A]);
    const deleteBtn = screen.getByText('Alpha')
      .closest('.MuiListItemButton-root')!
      .querySelector('.delete-btn') as HTMLElement;
    fireEvent.click(deleteBtn);
    expect(screen.getByText(/Delete pages\/a\.md/)).toBeInTheDocument();
    expect(props.onDeletePage).not.toHaveBeenCalled();
    fireEvent.click(screen.getAllByText('Delete')[0]);
    expect(props.onDeletePage).toHaveBeenCalledWith('pages/a.md');
  });
});
