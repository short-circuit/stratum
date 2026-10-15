import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import RemindersPanelDesktop from './RemindersPanel.desktop';
import * as api from '../../lib/commands';
import type { ReminderDto } from '../../lib/types';

vi.mock('../../lib/commands', () => ({
  listReminders: vi.fn(),
  snoozeReminder: vi.fn(),
  clearReminder: vi.fn(),
  onReminderDue: vi.fn().mockResolvedValue(() => {}),
  onReminderOpen: vi.fn().mockResolvedValue(() => {}),
}));

const now = new Date();
const overdue: ReminderDto = {
  block_id: 'b1',
  page_path: 'pages/a.md',
  content: 'Overdue task',
  due: '2020-01-01T00:00:00.000Z',
  marker: null,
  priority: null,
  page_title: 'Page A',
};
const today: ReminderDto = {
  block_id: 'b2',
  page_path: 'pages/b.md',
  content: 'Today task',
  due: new Date(now.getTime() + 60_000).toISOString(),
  marker: null,
  priority: null,
  page_title: 'Page B',
};
const upcoming: ReminderDto = {
  block_id: 'b3',
  page_path: 'pages/c.md',
  content: 'Upcoming task',
  due: new Date(now.getTime() + 3 * 86_400_000).toISOString(),
  marker: null,
  priority: null,
  page_title: 'Page C',
};

describe('RemindersPanelDesktop', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (api.onReminderDue as ReturnType<typeof vi.fn>).mockResolvedValue(() => {});
    (api.onReminderOpen as ReturnType<typeof vi.fn>).mockResolvedValue(() => {});
    (api.listReminders as ReturnType<typeof vi.fn>).mockResolvedValue([overdue, today, upcoming]);
  });

  it('renders the three grouping sections', async () => {
    render(
      <MemoryRouter>
        <RemindersPanelDesktop />
      </MemoryRouter>,
    );
    await waitFor(() => {
      expect(screen.getByText('Overdue (1)')).toBeTruthy();
    });
    expect(screen.getByText('Overdue task')).toBeTruthy();
    expect(screen.getByText('Today (1)')).toBeTruthy();
    expect(screen.getByText('Today task')).toBeTruthy();
    expect(screen.getByText('Upcoming (1)')).toBeTruthy();
    expect(screen.getByText('Upcoming task')).toBeTruthy();
  });

  it('calls clearReminder when the Done chip is clicked', async () => {
    render(
      <MemoryRouter>
        <RemindersPanelDesktop />
      </MemoryRouter>,
    );
    await waitFor(() => {
      expect(screen.getByText('Overdue task')).toBeTruthy();
    });
    fireEvent.click(screen.getAllByText('Done')[0]);
    await waitFor(() => {
      expect(api.clearReminder).toHaveBeenCalledWith('pages/a.md', 'b1');
    });
  });

  it('calls snoozeReminder with 10 minutes when the 10m chip is clicked', async () => {
    render(
      <MemoryRouter>
        <RemindersPanelDesktop />
      </MemoryRouter>,
    );
    await waitFor(() => {
      expect(screen.getByText('Overdue task')).toBeTruthy();
    });
    fireEvent.click(screen.getAllByText('10m')[0]);
    await waitFor(() => {
      expect(api.snoozeReminder).toHaveBeenCalledWith('pages/a.md', 'b1', 10);
    });
  });

  it('shows the empty state when no reminders', async () => {
    (api.listReminders as ReturnType<typeof vi.fn>).mockResolvedValue([]);
    render(
      <MemoryRouter>
        <RemindersPanelDesktop />
      </MemoryRouter>,
    );
    await waitFor(() => {
      expect(screen.getByText('No reminders set')).toBeTruthy();
    });
  });
});
