/**
 * Shared logic and data for the OutlinerEditor platform variants.
 *
 * Provides the BlockNote schema, shared Props interface, and the useEditorData()
 * hook that encapsulates all editor state management: creation, block loading,
 * auto-save (debounced), math rendering, wiki-link preview popup, dead-link
 * detection, and hover/click event delegation.
 *
 * The implementation was split out during the E6 sizing-gate refactor
 * (§2.2 of .sisyphus/refactoring-plan.md):
 *  - schema + highlighter  → ./editorSchema
 *  - useEditorData + types → ./useEditorData
 * This module is a thin barrel so existing importers (index.tsx,
 * OutlinerEditor.desktop.tsx, OutlinerEditor.mobile.tsx) are unaffected.
 *
 * @module OutlinerEditor/OutlinerEditor.shared
 */

/* eslint-disable react-refresh/only-export-components */
export { schema } from './editorSchema';
export {
  useEditorData,
  type Props,
  type MathEditState,
  type PreviewState,
  type DeadLinkPopupState,
  type EditorData,
} from './useEditorData';
