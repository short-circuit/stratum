export const COLUMNS = ['todo', 'in_progress', 'done'] as const;
export type ColumnId = (typeof COLUMNS)[number];

export const COLUMN_CONFIG: Record<ColumnId, { label: string; markers: string[]; color: string }> = {
  todo:        { label: 'To Do',        markers: ['TODO', 'NOW', 'LATER', 'WAITING'], color: '#f59e0b' },
  in_progress: { label: 'In Progress',  markers: ['DOING'],                           color: '#3b82f6' },
  done:        { label: 'Done',         markers: ['DONE', 'CANCELLED'],               color: '#10b981' },
};
