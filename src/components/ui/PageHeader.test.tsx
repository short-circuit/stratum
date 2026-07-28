import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import PageHeader from './PageHeader';

describe('PageHeader', () => {
  it('renders title', () => {
    render(<PageHeader title="My Notes" />);
    expect(screen.getByText('My Notes')).toBeInTheDocument();
  });

  it('renders subtitle when provided', () => {
    render(<PageHeader title="Notes" subtitle="3 pages" />);
    expect(screen.getByText('3 pages')).toBeInTheDocument();
  });

  it('renders back button and fires onBack callback', () => {
    const onBack = vi.fn();
    render(<PageHeader title="Note" onBack={onBack} />);
    const backBtn = document.querySelector('[data-testid="ArrowBackIcon"]')?.closest('button');
    expect(backBtn).toBeInTheDocument();
    fireEvent.click(backBtn!);
    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it('does not render back button when onBack not provided', () => {
    render(<PageHeader title="Note" />);
    expect(document.querySelector('[data-testid="ArrowBackIcon"]')).not.toBeInTheDocument();
  });

  it('renders action buttons', () => {
    const onClick = vi.fn();
    render(
      <PageHeader
        title="Notes"
        actions={[
          { label: 'Edit', onClick },
          { label: 'Delete', onClick, color: 'error' },
        ]}
      />
    );
    expect(screen.getByText('Edit')).toBeInTheDocument();
    expect(screen.getByText('Delete')).toBeInTheDocument();
  });

  it('fires action onClick', () => {
    const onClick = vi.fn();
    render(
      <PageHeader
        title="Notes"
        actions={[{ label: 'Save', onClick }]}
      />
    );
    fireEvent.click(screen.getByText('Save'));
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
