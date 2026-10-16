import i18n from '../../i18n';

export const COLUMNS = ['todo', 'in_progress', 'done'] as const;
export type ColumnId = (typeof COLUMNS)[number];

/**
 * Column configuration. `label` is a translation key resolved at render time
 * through `useTranslation().t()` — never call `i18n.t()` at module scope,
 * i18next is not initialized when this module is first evaluated, which
 * silently renders empty column headers (defect fixed in t_5774176b).
 */
export const COLUMN_CONFIG: Record<ColumnId, { labelKey: string; markers: string[]; color: string }> = {
  todo:        { labelKey: 'kanban.to_do',        markers: ['TODO', 'NOW', 'LATER', 'WAITING'], color: '#f59e0b' },
  in_progress: { labelKey: 'kanban.in_progress',  markers: ['DOING'],                           color: '#3b82f6' },
  done:        { labelKey: 'kanban.done',         markers: ['DONE', 'CANCELLED'],               color: '#10b981' },
};

/** Resolve a column's display label through the active translator. */
export function columnLabel(columnId: ColumnId, t: (key: string) => string): string {
  // Fall back to the raw English label so a missing translation key can never
  // render an empty header.
  return t(COLUMN_CONFIG[columnId].labelKey) || String(i18n.t(COLUMN_CONFIG[columnId].labelKey, { lng: 'en' })) || columnId;
}
