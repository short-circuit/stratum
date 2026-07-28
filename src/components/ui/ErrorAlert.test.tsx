import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import ErrorAlert from './ErrorAlert';

describe('ErrorAlert', () => {
  it('renders error message', () => {
    render(<ErrorAlert message="Something went wrong" />);
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
  });

  it('renders nothing when message is null', () => {
    const { container } = render(<ErrorAlert message={null} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders nothing when message is empty string', () => {
    const { container } = render(<ErrorAlert message="" />);
    expect(container.firstChild).toBeNull();
  });

  it('fires onClose callback when dismiss button clicked', () => {
    const onClose = vi.fn();
    render(<ErrorAlert message="Error" onClose={onClose} />);
    const closeBtn = document.querySelector('.MuiAlert-action button');
    expect(closeBtn).toBeInTheDocument();
    fireEvent.click(closeBtn!);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('has error role', () => {
    render(<ErrorAlert message="Oops" />);
    const alert = document.querySelector('[role="alert"]');
    expect(alert).toBeInTheDocument();
    expect(alert?.textContent).toContain('Oops');
  });
});
