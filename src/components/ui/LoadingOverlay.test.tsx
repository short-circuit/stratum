import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import LoadingOverlay from './LoadingOverlay';

describe('LoadingOverlay', () => {
  it('renders spinner with default message', () => {
    render(<LoadingOverlay />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
    expect(document.querySelector('.MuiCircularProgress-root')).toBeInTheDocument();
  });

  it('renders custom message', () => {
    render(<LoadingOverlay message="Indexing..." />);
    expect(screen.getByText('Indexing...')).toBeInTheDocument();
  });

  it('renders in overlay mode by default', () => {
    const { container } = render(<LoadingOverlay />);
    // Overlay mode adds an outer wrapper Box with position: absolute
    const outer = container.firstChild as HTMLElement;
    const inner = outer?.firstChild as HTMLElement;
    expect(outer).toBeInTheDocument();
    expect(inner?.textContent).toContain('Loading...');
  });

  it('renders inline when overlay=false', () => {
    const { container } = render(<LoadingOverlay overlay={false} />);
    // Inline mode has no outer wrapper — directly renders content
    const el = container.firstChild as HTMLElement;
    expect(el).toBeInTheDocument();
    expect(el.textContent).toContain('Loading...');
  });
});
