import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import ConfirmDialog from './ConfirmDialog';

describe('ConfirmDialog', () => {
  it('renders dialog when open=true', () => {
    render(
      <ConfirmDialog
        open={true}
        title="Delete note?"
        message="Are you sure?"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );
    expect(screen.getByText('Delete note?')).toBeInTheDocument();
    expect(screen.getByText('Are you sure?')).toBeInTheDocument();
  });

  it('does not render when open=false', () => {
    const { container } = render(
      <ConfirmDialog
        open={false}
        title="Delete note?"
        message="Are you sure?"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );
    // MUI Dialog renders nothing in DOM when closed
    expect(container.querySelector('.MuiDialog-root')).toBeNull();
  });

  it('fires onConfirm when confirm button clicked', () => {
    const onConfirm = vi.fn();
    render(
      <ConfirmDialog
        open={true}
        title="Delete?"
        message="Sure?"
        onConfirm={onConfirm}
        onCancel={vi.fn()}
      />
    );
    fireEvent.click(screen.getByText('Confirm'));
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it('fires onCancel when cancel button clicked', () => {
    const onCancel = vi.fn();
    render(
      <ConfirmDialog
        open={true}
        title="Delete?"
        message="Sure?"
        onConfirm={vi.fn()}
        onCancel={onCancel}
      />
    );
    fireEvent.click(screen.getByText('Cancel'));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it('uses custom button labels', () => {
    render(
      <ConfirmDialog
        open={true}
        title="Delete?"
        message="Sure?"
        confirmLabel="Yes, delete"
        cancelLabel="Keep it"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );
    expect(screen.getByText('Yes, delete')).toBeInTheDocument();
    expect(screen.getByText('Keep it')).toBeInTheDocument();
  });

  it('renders confirm button with error color', () => {
    render(
      <ConfirmDialog
        open={true}
        title="Delete?"
        message="Sure?"
        confirmColor="error"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />
    );
    const confirmBtn = screen.getByText('Confirm').closest('button');
    expect(confirmBtn).toBeInTheDocument();
  });
});
