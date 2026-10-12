import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import MobileEditorOverlays, { type MobileEditorOverlaysProps } from './MobileEditorOverlays';

/**
 * Row-mode regression test for bug t_1100de53:
 * the mobile journal list renders one content-sized OutlinerEditor per past
 * date, and the mobile editor previously rendered two bottom-anchored FABs
 * (insert + marker) that overflowed the row and overlapped each other/row
 * content. Row mode must render a single reserved-region action button
 * instead, with BOTH insert and marker capabilities reachable through its
 * combined menu.
 */

// The overlay talks to the backend only through the dead-link popup (api) and
// TTS (audio); neither is exercised by these tests, but their modules must
// load, so mock them to keep the tests hermetic.
vi.mock('../../lib/commands', () => ({
  createPage: vi.fn(),
}));
vi.mock('../../lib/audio', () => ({
  speakText: vi.fn(),
  textFromBlock: () => '',
}));
vi.mock('../../stores/recentsStore', () => ({
  useRecentsStore: { getState: () => ({ refresh: vi.fn() }) },
}));

const markerOptions = ['TODO', 'DOING', 'DONE', 'NOW', 'LATER', 'WAITING', 'CANCELLED'];

function makeProps(overrides: Partial<MobileEditorOverlaysProps> = {}): MobileEditorOverlaysProps {
  return {
    editor: {} as MobileEditorOverlaysProps['editor'],
    containerRef: { current: null },
    children: null,
    mathEdit: null,
    setMathEdit: vi.fn(),
    preview: null,
    setPreview: vi.fn(),
    deadLinkPopup: null,
    setDeadLinkPopup: vi.fn(),
    navigateRef: { current: () => {} },
    onInsert: vi.fn(),
    onInsertMath: vi.fn(),
    onApplyMarker: vi.fn(),
    ...overrides,
  };
}

describe('MobileEditorOverlays — row mode (journal list)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders NO floating FABs in row mode', () => {
    render(
      <MobileEditorOverlays {...makeProps({ rowActions: true })}>
        <div>row content</div>
      </MobileEditorOverlays>,
    );
    expect(screen.queryByLabelText('Insert block')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Toggle marker')).not.toBeInTheDocument();
  });

  it('renders a single 40px row-actions button in row mode', () => {
    render(
      <MobileEditorOverlays {...makeProps({ rowActions: true })}>
        <div>row content</div>
      </MobileEditorOverlays>,
    );
    const btn = screen.getByRole('button', { name: 'Row actions' });
    expect(btn).toBeInTheDocument();
    // Touch target >= 40px (MUI touch standard) — enforced via sx width/height.
    expect(btn).toHaveStyle({ width: '40px', height: '40px' });
  });

  it('opens the combined menu and triggers insert + marker actions', () => {
    const onInsert = vi.fn();
    const onInsertMath = vi.fn();
    const onApplyMarker = vi.fn();
    render(
      <MobileEditorOverlays
        {...makeProps({ rowActions: true, onInsert, onInsertMath, onApplyMarker })}
      >
        <div>row content</div>
      </MobileEditorOverlays>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Row actions' }));

    // Insert entries are reachable from the combined menu.
    fireEvent.click(screen.getByText('Paragraph'));
    expect(onInsert).toHaveBeenCalledWith('paragraph');

    // While the menu is open the MUI Modal marks the rest of the page
    // aria-hidden, so re-find the trigger with hidden: true.
    const rowActionsButton = () =>
      screen.getByRole('button', { name: 'Row actions', hidden: true });

    fireEvent.click(rowActionsButton());
    fireEvent.click(screen.getByText('Heading 2'));
    expect(onInsert).toHaveBeenCalledWith('heading', 2);

    fireEvent.click(rowActionsButton());
    fireEvent.click(screen.getByText('Math equation'));
    expect(onInsertMath).toHaveBeenCalled();

    // Marker entries are reachable; all seven marker options are offered.
    fireEvent.click(rowActionsButton());
    for (const m of markerOptions) {
      fireEvent.click(screen.getByText(m));
      expect(onApplyMarker).toHaveBeenCalledWith(m);
      fireEvent.click(rowActionsButton());
    }
  });

  it('renders the two floating FABs when row mode is off (full-page editor regression guard)', () => {
    render(
      <MobileEditorOverlays {...makeProps()}>
        <div>page content</div>
      </MobileEditorOverlays>,
    );
    expect(screen.getByLabelText('Insert block')).toBeInTheDocument();
    expect(screen.getByLabelText('Toggle marker')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Row actions' })).not.toBeInTheDocument();
  });
});
