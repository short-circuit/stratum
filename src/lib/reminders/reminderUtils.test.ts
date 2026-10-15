import { describe, it, expect } from 'vitest';
import {
  parseDue,
  formatDueLocal,
  dueToLocalInput,
  localInputToUtcIso,
  isOverdue,
  humanizeDue,
} from './reminderUtils';

describe('parseDue', () => {
  it('returns a Date for a valid ISO string', () => {
    const d = parseDue('2026-10-15T07:00:00Z');
    expect(d).not.toBeNull();
    expect(d!.toISOString()).toBe('2026-10-15T07:00:00.000Z');
  });

  it('returns null for empty or invalid input', () => {
    expect(parseDue('')).toBeNull();
    expect(parseDue('not-a-date')).toBeNull();
    expect(parseDue('garbage')).toBeNull();
  });
});

describe('formatDueLocal / dueToLocalInput', () => {
  it('formats a Date as YYYY-MM-DDTHH:mm', () => {
    const d = new Date(2026, 9, 15, 9, 5); // local Oct 15 2026 09:05
    expect(formatDueLocal(d)).toBe('2026-10-15T09:05');
  });

  it('dueToLocalInput converts an ISO string to local input', () => {
    const local = dueToLocalInput('2026-10-15T07:00:00Z');
    expect(local).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/);
  });

  it('dueToLocalInput returns empty for invalid', () => {
    expect(dueToLocalInput('')).toBe('');
  });
});

describe('localInputToUtcIso', () => {
  it('converts a local datetime-local value to RFC3339 UTC ISO', () => {
    const iso = localInputToUtcIso('2026-10-15T09:00');
    expect(iso).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/);
    const d = new Date(iso);
    expect(d.getFullYear()).toBe(2026);
    expect(d.getMonth()).toBe(9);
    expect(d.getDate()).toBe(15);
  });

  it('returns empty for invalid', () => {
    expect(localInputToUtcIso('')).toBe('');
  });
});

describe('isOverdue', () => {
  it('is true when due is before now', () => {
    expect(isOverdue(new Date('2020-01-01T00:00:00Z'), new Date('2026-01-01T00:00:00Z'))).toBe(true);
  });
  it('is false when due is in the future', () => {
    expect(isOverdue(new Date('2026-01-02T00:00:00Z'), new Date('2026-01-01T00:00:00Z'))).toBe(false);
  });
});

describe('humanizeDue', () => {
  // Mock t that interpolates {{key}} placeholders for assertions.
  const t = (key: string, opts?: Record<string, unknown>): string => {
    const base: Record<string, string> = {
      'reminders.humanize.now': 'Due now',
      'reminders.humanize.overdue_now': 'Overdue',
      'reminders.humanize.due_in_min': 'Due in {minutes} min',
      'reminders.humanize.overdue_min': 'Overdue by {minutes} min',
      'reminders.humanize.due_in_hours': 'Due in {hours} h',
      'reminders.humanize.overdue_hours': 'Overdue by {hours} h',
      'reminders.humanize.due_date': 'Due {date}',
    };
    let s = base[key] ?? key;
    if (opts) {
      for (const [k, v] of Object.entries(opts)) {
        s = s.replace(`{${k}}`, String(v));
      }
    }
    return s;
  };

  const now = new Date('2026-10-15T12:00:00Z');

  it('says due now when within a minute', () => {
    expect(humanizeDue(new Date(now.getTime() + 30_000), now, t)).toBe('Due now');
  });

  it('says overdue now when past within a minute', () => {
    expect(humanizeDue(new Date(now.getTime() - 30_000), now, t)).toBe('Overdue');
  });

  it('reports minutes when under an hour', () => {
    expect(humanizeDue(new Date(now.getTime() + 5 * 60_000), now, t)).toBe('Due in 5 min');
  });

  it('reports overdue minutes when under an hour past', () => {
    expect(humanizeDue(new Date(now.getTime() - 5 * 60_000), now, t)).toBe('Overdue by 5 min');
  });

  it('reports hours when under a day', () => {
    expect(humanizeDue(new Date(now.getTime() + 2 * 3_600_000), now, t)).toBe('Due in 2 h');
  });

  it('reports overdue hours when under a day past', () => {
    expect(humanizeDue(new Date(now.getTime() - 2 * 3_600_000), now, t)).toBe('Overdue by 2 h');
  });

  it('falls back to a date for reminders more than a day away', () => {
    const s = humanizeDue(new Date(now.getTime() + 2 * 86_400_000), now, t);
    expect(s).toMatch(/^Due \d{4}-\d{2}-\d{2} \d{2}:\d{2}$/);
  });
});
