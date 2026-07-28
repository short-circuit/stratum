import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import EmptyState from './EmptyState';

describe('EmptyState', () => {
  it('renders message', () => {
    render(<EmptyState message="No items found" />);
    expect(screen.getByText('No items found')).toBeInTheDocument();
  });

  it('renders description when provided', () => {
    render(<EmptyState message="Empty" description="Create a new item to get started" />);
    expect(screen.getByText('Create a new item to get started')).toBeInTheDocument();
  });

  it('does not render description when not provided', () => {
    const { container } = render(<EmptyState message="Empty" />);
    const captions = container.querySelectorAll('.MuiTypography-caption');
    expect(captions.length).toBe(0);
  });

  it('renders action button and fires callback', () => {
    const onAction = vi.fn();
    render(<EmptyState message="Empty" actionLabel="Add Note" onAction={onAction} />);
    const btn = screen.getByText('Add Note');
    expect(btn).toBeInTheDocument();
    fireEvent.click(btn);
    expect(onAction).toHaveBeenCalledTimes(1);
  });

  it('does not render button when onAction not provided', () => {
    render(<EmptyState message="Empty" actionLabel="Add Note" />);
    expect(screen.queryByText('Add Note')).not.toBeInTheDocument();
  });

  it('renders icon when provided', () => {
    render(<EmptyState message="Empty" icon={<span>🔍</span>} />);
    expect(screen.getByText('🔍')).toBeInTheDocument();
  });

  it('does not render icon when not provided', () => {
    const { container } = render(<EmptyState message="Empty" />);
    const icons = container.querySelectorAll('.MuiSvgIcon-root');
    // No icon wrapper should have any SVG icon
    expect(icons.length).toBe(0);
  });
});
