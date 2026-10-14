import { parseDue, humanizeDue } from './reminders/reminderUtils';

const CHIP_CLASS = 'reminder-chip';

/**
 * Append a small due-reminder chip to every rendered block that carries a
 * `due` property. Existing chips are removed first so re-application never
 * duplicates them. Overdue chips are flagged with an extra class for styling.
 */
export function applyReminderDecorations(
  container: HTMLElement,
  metaByBlock: Map<string, { properties: [string, string][] }>,
  t: (key: string, opts?: Record<string, unknown>) => string,
): void {
  clearReminderDecorations(container);
  const blocks = container.querySelectorAll<HTMLElement>('[data-block-id]');
  blocks.forEach((el) => {
    const blockId = el.getAttribute('data-block-id');
    if (!blockId) return;
    const meta = metaByBlock.get(blockId);
    if (!meta) return;
    const dueEntry = meta.properties.find(([k]) => k === 'due');
    if (!dueEntry) return;
    const due = parseDue(dueEntry[1]);
    if (!due) return;
    const now = new Date();
    const overdue = due.getTime() < now.getTime();
    const chip = document.createElement('span');
    chip.className = `${CHIP_CLASS} ${overdue ? 'reminder-chip--overdue' : 'reminder-chip--upcoming'}`;
    chip.textContent = humanizeDue(due, now, t);
    chip.setAttribute('data-reminder-chip', '');
    el.appendChild(chip);
  });
}

/** Remove all reminder chips under `el`. */
export function clearReminderDecorations(el: HTMLElement): void {
  el.querySelectorAll<HTMLElement>(`.${CHIP_CLASS}`).forEach((c) => c.remove());
}
