/**
 * Shared editor types for the Outliner editor.
 *
 * Split out of useEditorData.ts during the AGENTS.md sizing-gate followup
 * (E6.F1) so the hook module stays under the 500-line file gate. Consumers
 * import these via the useEditorData module barrel, unchanged.
 */

import type { RefObject, Dispatch, SetStateAction, MutableRefObject } from 'react';

export interface Props {
  pagePath: string;
  autoFocus?: boolean;
  minHeight?: string;
}

export type MathEditState = { latex: string; pos: number } | null;
export type PreviewState = {
  content: string;
  pageTitle: string | null;
  pagePath: string;
  position: { x: number; y: number };
  loading: boolean;
} | null;
export type DeadLinkPopupState = {
  target: string;
  position: { x: number; y: number };
} | null;

/**
 * Everything returned by useEditorData(). Declared here so platform variants
 * and MobileEditorOverlays can type their props without importing the hook.
 */
export interface EditorData {
  editor: ReturnType<typeof import('@blocknote/react').useCreateBlockNote>;
  status: string;
  error: string | null;
  setStatus: Dispatch<SetStateAction<string>>;
  setError: Dispatch<SetStateAction<string | null>>;
  pageMarkers: string[];
  mathEdit: MathEditState;
  setMathEdit: Dispatch<SetStateAction<MathEditState>>;
  containerRef: RefObject<HTMLDivElement | null>;
  ctrlHeld: MutableRefObject<boolean>;
  preview: PreviewState;
  setPreview: Dispatch<SetStateAction<PreviewState>>;
  deadLinkPopup: DeadLinkPopupState;
  setDeadLinkPopup: Dispatch<SetStateAction<DeadLinkPopupState>>;
  markDeadLinks: (root: HTMLElement) => void;
  showPreview: (href: string, x: number, y: number) => void;
  dismissPreview: () => void;
  navigateRef: MutableRefObject<(path: string) => void>;
  pagePath: string;
  minHeight: string;
  persistBlocks: (blockNoteBlocks: any[]) => void;
  saving: boolean;
  lastSavedAt: number | null;
}
