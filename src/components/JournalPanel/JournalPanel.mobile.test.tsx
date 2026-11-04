import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, it, expect, vi } from 'vitest';
import JournalPanelMobile from './JournalPanel.mobile';

/**
 * Regression test for bug t_1100de53: every mobile journal row (today AND
 * past dates) must render its OutlinerEditor in "row mode" so the two
 * bottom-anchored FABs are replaced by a single reserved-region action button
 * (no overlapping/clipped floating buttons, no row special-casing).
 */

// The editors are heavy (BlockNote) — assert on the props JournalPanelMobile
// passes down instead of mounting real editors.
const editorSpy = vi.fn((props: Record<string, unknown>) => (
  <div data-testid="editor">{JSON.stringify(props)}</div>
));
vi.mock('../OutlinerEditor', () => ({ default: (props: Record<string, unknown>) => editorSpy(props) }));
// VoiceCaptureButton (rendered by the journal header) subscribes to Tauri events;
// jsdom has no __TAURI_INTERNALS__, so stub listen() to keep the render hermetic.
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async () => () => {}),
}));
vi.mock('../JournalCalendar', () => ({
  default: () => <div data-testid="calendar" />,
}));

const PAST_DATES = ['2026-09-29', '2026-09-28', '2026-09-27'];

vi.mock('./JournalPanel.shared', () => ({
  useJournalPanel: () => ({
    today: new Date(),
    todayPagePath: 'journals/2026-10-12.md',
    todayExists: true,
    journalLoading: false,
    journalError: null,
    retryJournal: vi.fn(),
    repairJournal: vi.fn(),
    targetDate: '2026-09-29',
    allJournalDates: new Set<string>(),
    pastDates: PAST_DATES,
    visibleCount: PAST_DATES.length,
    visibleSections: new Set(PAST_DATES),
    sectionRef: () => ({ current: null }),
    sentinelRef: { current: null },
    scrollRootRef: { current: null },
    calendarOpen: false,
    setCalendarOpen: vi.fn(),
    handleDateSelect: vi.fn(),
    createNewDay: vi.fn(),
  }),
  formatDisplayDate: (d: string) => d,
}));

describe('JournalPanelMobile — row mode wiring', () => {
  it('passes rowActions to the today editor', () => {
    render(
      <MemoryRouter>
        <JournalPanelMobile />
      </MemoryRouter>,
    );
    const editors = screen.getAllByTestId('editor');
    const todayEditor = editors.find((el) =>
      el.textContent?.includes('journals/2026-10-12.md'),
    );
    expect(todayEditor).toBeDefined();
    expect(todayEditor?.textContent).toContain('"rowActions":true');
  });

  it('passes rowActions to every visible past-date editor', () => {
    render(
      <MemoryRouter>
        <JournalPanelMobile />
      </MemoryRouter>,
    );
    const editors = screen.getAllByTestId('editor');
    expect(editors.length).toBe(PAST_DATES.length + 1);
    for (const date of PAST_DATES) {
      const rowEditor = editors.find((el) => el.textContent?.includes(`journals/${date}.md`));
      expect(rowEditor, `past row ${date}`).toBeDefined();
      expect(rowEditor?.textContent).toContain('"rowActions":true');
    }
  });

  it('renders no floating insert/marker controls itself (they are row actions now)', () => {
    render(
      <MemoryRouter>
        <JournalPanelMobile />
      </MemoryRouter>,
    );
    expect(screen.queryByLabelText('Insert block')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Toggle marker')).not.toBeInTheDocument();
  });
});
