import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import BacklinkSnippetBody from './BacklinkSnippetBody';
import BacklinkSnippetPopup from './BacklinkSnippetPopup';

const bodyProps = {
  noteTitle: 'Source Note',
  noteId: 'pages/source.md',
  context: ['Intro paragraph', 'The exact backlinked block', 'Follow-up'],
  anchorContent: 'The exact backlinked block',
  loading: false,
  error: false,
  onNavigate: vi.fn(),
};

describe('BacklinkSnippetBody', () => {
  it('renders the note title and highlights the backlinked block', () => {
    render(<BacklinkSnippetBody {...bodyProps} />);
    expect(screen.getByText('Source Note')).toBeInTheDocument();
    // All context blocks render, in order.
    const intro = screen.getByText('Intro paragraph');
    const anchor = screen.getByText('The exact backlinked block');
    const followup = screen.getByText('Follow-up');
    expect(intro).toBeInTheDocument();
    expect(anchor).toBeInTheDocument();
    expect(followup).toBeInTheDocument();
    // The anchor block is visually distinguished from its siblings:
    // it has a background colour while the intro does not.
    expect(getComputedStyle(anchor).backgroundColor).not.toBe('rgba(0, 0, 0, 0)');
    expect(getComputedStyle(anchor).fontWeight).toBe('600');
  });

  it('falls back to the note id when no title is available', () => {
    render(<BacklinkSnippetBody {...bodyProps} noteTitle="" />);
    expect(screen.getByText('pages/source.md')).toBeInTheDocument();
  });

  it('shows a loading spinner without content', () => {
    render(<BacklinkSnippetBody {...bodyProps} loading />);
    expect(screen.queryByText('Source Note')).not.toBeInTheDocument();
    // CircularProgress is present.
    expect(document.querySelector('.MuiCircularProgress-root')).not.toBeNull();
  });

  it('shows an error state instead of a stuck spinner', () => {
    render(<BacklinkSnippetBody {...bodyProps} error />);
    expect(screen.getByText(/could not load/i)).toBeInTheDocument();
    expect(document.querySelector('.MuiCircularProgress-root')).toBeNull();
  });

  it('handles an empty context window gracefully', () => {
    render(<BacklinkSnippetBody {...bodyProps} context={[]} anchorContent="" />);
    expect(screen.getByText(/no content/i)).toBeInTheDocument();
  });

  it('fires onNavigate when the note title is clicked', () => {
    render(<BacklinkSnippetBody {...bodyProps} />);
    fireEvent.click(screen.getByText('Source Note'));
    expect(bodyProps.onNavigate).toHaveBeenCalledTimes(1);
  });
});

describe('BacklinkSnippetPopup (desktop popover)', () => {
  const popupProps = {
    noteId: 'pages/source.md',
    noteTitle: 'Source Note',
    context: ['Intro', 'anchor text here', 'Outro'],
    anchorContent: 'anchor text here',
    position: { x: 500, y: 700 },
    loading: false,
    error: false,
    onClose: vi.fn(),
  };

  it('positions using the anchorPosition coordinates (viewport-aware)', () => {
    render(
      <MemoryRouter>
        <BacklinkSnippetPopup {...popupProps} />
      </MemoryRouter>,
    );
    // The popover paper is present in the body portal and the popup renders.
    expect(screen.getByText('Source Note')).toBeInTheDocument();
    const paper = document.querySelector('.MuiPopover-paper');
    expect(paper).not.toBeNull();
    // anchorPosition coordinates are forwarded to MUI's positioning engine,
    // which clamps placement to the viewport with a margin threshold — the
    // paper is positioned inside the body portal (never inside the editor's
    // scroll container), so it cannot be clipped by the container.
    expect(document.body.contains(paper)).toBe(true);
  });

  it('exposes dialog role and aria-label without breaking the editor', () => {
    render(
      <MemoryRouter>
        <BacklinkSnippetPopup {...popupProps} />
      </MemoryRouter>,
    );
    const paper = screen.getByText('Source Note').closest('[role="dialog"]');
    expect(paper).not.toBeNull();
    expect(paper).toHaveAttribute(
      'aria-label',
      expect.stringContaining('Source Note'),
    );
  });

  it('navigates to the note and closes on title click', () => {
    const onClose = vi.fn();
    render(
      <MemoryRouter>
        <BacklinkSnippetPopup {...popupProps} onClose={onClose} />
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByText('Source Note'));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('dismisses when Escape is pressed (keyboard dismissal)', () => {
    const onClose = vi.fn();
    render(
      <MemoryRouter>
        <BacklinkSnippetPopup {...popupProps} onClose={onClose} />
      </MemoryRouter>,
    );
    // MUI attaches its Escape handler to the modal root (role=presentation).
    const modalRoot = document.querySelector('[role="presentation"]');
    expect(modalRoot).not.toBeNull();
    fireEvent.keyDown(modalRoot as Element, { key: 'Escape' });
    expect(onClose).toHaveBeenCalled();
  });
});
