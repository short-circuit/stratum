import { useMemo, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import * as api from '../../lib/commands';
import { useStore } from '../../stores/appStore';
import { useReminders } from '../../lib/reminders/useReminders';
import { parseDue, isOverdue, humanizeDue } from '../../lib/reminders/reminderUtils';
import type { ReminderDto } from '../../lib/types';

export interface ReminderGroup {
  overdue: ReminderDto[];
  today: ReminderDto[];
  upcoming: ReminderDto[];
}

function sameLocalDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

export function groupReminders(reminders: ReminderDto[], now: Date): ReminderGroup {
  const overdue: ReminderDto[] = [];
  const today: ReminderDto[] = [];
  const upcoming: ReminderDto[] = [];
  for (const r of reminders) {
    const d = parseDue(r.due);
    if (!d) continue;
    if (isOverdue(d, now)) {
      overdue.push(r);
    } else if (sameLocalDay(d, now)) {
      today.push(r);
    } else {
      upcoming.push(r);
    }
  }
  return { overdue, today, upcoming };
}

/** Shared reminder panel state/actions, desktop + mobile. */
export function useRemindersPanel() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const setHighlightBlock = useStore((s) => s.setHighlightBlock);
  const { reminders, loading, error, refresh, toast, dismissToast } = useReminders();

  const now = useMemo(() => new Date(), []);
  const grouped = useMemo(() => groupReminders(reminders, now), [reminders, now]);

  const snooze = useCallback(
    async (pagePath: string, blockId: string, minutes: number) => {
      await api.snoozeReminder(pagePath, blockId, minutes);
      await refresh();
    },
    [refresh],
  );

  const complete = useCallback(
    async (pagePath: string, blockId: string) => {
      await api.clearReminder(pagePath, blockId);
      await refresh();
    },
    [refresh],
  );

  const open = useCallback(
    (pagePath: string, blockId: string) => {
      setHighlightBlock({ pagePath, blockId });
      navigate(`/page/${encodeURIComponent(pagePath)}`);
    },
    [navigate, setHighlightBlock],
  );

  const humanize = useCallback(
    (r: ReminderDto) => {
      const d = parseDue(r.due);
      return d ? humanizeDue(d, new Date(), t) : r.due;
    },
    [t],
  );

  return { grouped, loading, error, refresh, toast, dismissToast, snooze, complete, open, humanize };
}
