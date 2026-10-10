import i18n from '../../i18n';

export const COLUMNS = ['todo', 'in_progress', 'done'] as const;
export type ColumnId = (typeof COLUMNS)[number];

export const COLUMN_CONFIG: Record<ColumnId, { label: string; markers: string[]; color: string }> = {
  todo:        { label: i18n.t('kanban.to_do'),        markers: ['TODO', 'NOW', 'LATER', 'WAITING'], color: '#f59e0b' },
  in_progress: { label: i18n.t('kanban.in_progress'),  markers: ['DOING'],                           color: '#3b82f6' },
  done:        { label: 'Done',                        markers: ['DONE', 'CANCELLED'],               color: '#10b981' },
};
