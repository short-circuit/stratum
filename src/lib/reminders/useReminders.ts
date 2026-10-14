import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import * as api from '../commands';
import { useStore } from '../../stores/appStore';
import { parseDue, isOverdue } from './reminderUtils';
import type { ReminderDto } from '../types';

export interface ReminderToast {
  title: string;
  message: string;
  pagePath: string;
  blockId: string;
}

export interface UseRemindersResult {
  reminders: ReminderDto[];
  overdue: ReminderDto[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  toast: ReminderToast | null;
  dismissToast: () => void;
}

/**
 * Shared reminder state: loads the reminder list, refreshes when a reminder
 * becomes due, surfaces an in-app toast, and navigates/highlights on
 * notification open. Shared by the desktop + mobile reminder panels.
 */
export function useReminders(): UseRemindersResult {
  const { t } = useTranslation();
  const [reminders, setReminders] = useState<ReminderDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [toast, setToast] = useState<ReminderToast | null>(null);
  const setHighlightBlock = useStore((s) => s.setHighlightBlock);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const list = await api.listReminders();
      setReminders(list);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let mounted = true;
    void refresh();

    const onDue = api.onReminderDue((p) => {
      if (!mounted) return;
      setToast({
        title: t('reminders.toast.due_title'),
        message: p.content,
        pagePath: p.pagePath,
        blockId: p.blockId,
      });
      void refresh();
    });
    const onOpen = api.onReminderOpen((p) => {
      if (!mounted) return;
      setHighlightBlock({ pagePath: p.pagePath, blockId: p.blockId });
      void refresh();
    });

    return () => {
      mounted = false;
      onDue.then((un) => un());
      onOpen.then((un) => un());
    };
  }, [refresh, t, setHighlightBlock]);

  const overdue = useMemo(
    () =>
      reminders.filter((r) => {
        const d = parseDue(r.due);
        return d ? isOverdue(d, new Date()) : false;
      }),
    [reminders],
  );

  const dismissToast = useCallback(() => setToast(null), []);

  return { reminders, overdue, loading, error, refresh, toast, dismissToast };
}
