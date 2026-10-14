// Pure reminder date utilities. All functions are side-effect free so they can
// be unit-tested without a Tauri runtime or i18n instance.

/** Parse a due timestamp string into a `Date`, or `null` if invalid. */
export function parseDue(s: string): Date | null {
  if (!s) return null;
  const d = new Date(s);
  return Number.isNaN(d.getTime()) ? null : d;
}

function pad(n: number): string {
  return String(n).padStart(2, '0');
}

/** Format a `Date` as a local `datetime-local` input value (`YYYY-MM-DDTHH:mm`). */
export function formatDueLocal(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(
    d.getHours(),
  )}:${pad(d.getMinutes())}`;
}

/** Convert an ISO due string to a local `datetime-local` input value. */
export function dueToLocalInput(iso: string): string {
  const d = parseDue(iso);
  return d ? formatDueLocal(d) : '';
}

/** Convert a local `datetime-local` input value to an RFC3339 UTC ISO string. */
export function localInputToUtcIso(input: string): string {
  const d = new Date(input);
  return Number.isNaN(d.getTime()) ? '' : d.toISOString();
}

/** Whether the given due date is strictly before `now`. */
export function isOverdue(due: Date, now: Date): boolean {
  return due.getTime() < now.getTime();
}

/**
 * Produce a short human-readable due description.
 *
 * Returns e.g. `"Due in 5 min"`, `"Overdue by 2 h"`, or a formatted date for
 * reminders more than a day away. `t` is the i18next translate function.
 */
export function humanizeDue(
  due: Date,
  now: Date,
  t: (key: string, opts?: Record<string, unknown>) => string,
): string {
  const diffMs = due.getTime() - now.getTime();
  const absMs = Math.abs(diffMs);
  const isFuture = diffMs >= 0;

  if (absMs < 60_000) {
    return isFuture ? t('reminders.humanize.now') : t('reminders.humanize.overdue_now');
  }
  const minutes = Math.max(1, Math.round(absMs / 60_000));
  if (minutes < 60) {
    return isFuture
      ? t('reminders.humanize.due_in_min', { minutes })
      : t('reminders.humanize.overdue_min', { minutes });
  }
  const hours = Math.round(absMs / 3_600_000);
  if (hours < 24) {
    return isFuture
      ? t('reminders.humanize.due_in_hours', { hours })
      : t('reminders.humanize.overdue_hours', { hours });
  }
  return t('reminders.humanize.due_date', { date: formatDueLocal(due).replace('T', ' ') });
}
