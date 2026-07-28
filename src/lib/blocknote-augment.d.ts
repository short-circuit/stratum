/**
 * Augments BlockNote's BlockNoteEditor interface to expose the Tiptap
 * editor instance and ProseMirror view as typed public properties.
 *
 * BlockNote already declares these at runtime — this augmentation makes
 * TypeScript aware of them so callers can access `_tiptapEditor` and
 * `prosemirrorView` without `as any` casts.
 */
import '@blocknote/core';

declare module '@blocknote/core' {
  interface BlockNoteEditor {
    /** The underlying Tiptap editor instance. */
    _tiptapEditor: import('@tiptap/core').Editor;
    /** The underlying ProseMirror view (set after mount). */
    prosemirrorView?: import('prosemirror-view').EditorView;
  }
}
