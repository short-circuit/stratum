/**
 * Marker values offered by every mobile editor action menu.
 *
 * Single source of truth shared by the floating FAB menus (MobileMarkerMenu)
 * and the journal row-actions menu (MobileEditorOverlays) so the two surfaces
 * can never drift (AGENTS.md: no duplicated logic).
 *
 * Kept in a constants-only module so `editorActionMenuItems.tsx` stays a
 * components-only file (react-refresh/only-export-components).
 *
 * @module OutlinerEditor/markerOptions
 */

export const MARKER_OPTIONS: string[] = [
  'TODO',
  'DOING',
  'DONE',
  'NOW',
  'LATER',
  'WAITING',
  'CANCELLED',
];
