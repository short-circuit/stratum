//! Editing state: caret/selection, editing operations, undo/redo, clipboard,
//! paste normalization and IME composition.
//!
//! All structural edits go through [`pkm_block::ops`]. Undo is expressed as
//! concrete forward/inverse [`EditOp`] pairs rather than document snapshots:
//! [`History`] keeps at most 200 entries, coalesces contiguous text insertions in
//! the same block within 750 ms, clears the redo stack on a new edit, and applies
//! the inverse operation on undo.
//!
//! Byte offsets are always UTF-8 byte offsets on char boundaries. The IME helpers
//! ([`utf16_to_byte`], [`byte_to_utf16`]) translate the UTF-16 ranges reported by
//! platform input methods and reject offsets that land inside a surrogate pair.
//!
//! The implementation is split into submodules so no source file exceeds the
//! repository limit:
//!
//! - [`ops`] — the reversible [`EditOp`] pairs and their tree application.
//! - [`history`] — bounded undo/redo stacks and coalescing.
//! - [`editing`] — caret-driven editing operations.
//! - [`clipboard`] — copy/cut/paste and paste normalization.
//! - [`splice`] — reversible block-range splices with explicit link/content
//!   deltas, used by cross-block delete and multi-block paste.
//! - [`ime`] — composition state and UTF-16 offset translation.
//! - `text` — byte-offset and delimiter helpers shared by the above.

mod clipboard;
mod editing;
mod history;
mod ime;
mod ops;
mod splice;
mod text;

#[cfg(test)]
mod tests;

pub use clipboard::{normalize_paste, paste_markdown, ClipboardPayload};
pub use history::{History, HistoryEntry};
pub use ime::{byte_to_utf16, utf16_to_byte, Composition};
pub use ops::EditOp;
pub use splice::{ContentChange, LinkChange};

use pkm_block::BlockId;
use pkm_core::{PkmError, PkmResult};

use crate::document::EditorDocument;
use text::clamp_offset;

/// A text caret inside one block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caret {
    pub block_id: BlockId,
    pub byte_offset: usize,
}

/// A selection between two carets.
///
/// A same-block selection is a text range. A cross-block selection is a block
/// range plus head/tail offsets; clipboard helpers expand it in document order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Caret,
    pub head: Caret,
}

/// The headless editor state for one document.
#[derive(Debug)]
pub struct EditorState {
    doc: EditorDocument,
    caret: Option<Caret>,
    selection: Option<Selection>,
    history: History,
    composition: Option<Composition>,
    clipboard: Option<ClipboardPayload>,
}

impl EditorState {
    /// Create state over a document.
    pub fn new(doc: EditorDocument) -> Self {
        Self {
            doc,
            caret: None,
            selection: None,
            history: History::new(),
            composition: None,
            clipboard: None,
        }
    }

    /// Borrow the document.
    pub fn document(&self) -> &EditorDocument {
        &self.doc
    }

    /// Consume the state and return the document.
    pub fn into_document(self) -> EditorDocument {
        self.doc
    }

    /// The current caret.
    pub fn caret(&self) -> Option<Caret> {
        self.caret
    }

    /// The current selection.
    pub fn selection(&self) -> Option<Selection> {
        self.selection
    }

    /// The clipboard payload from the last copy/cut.
    pub fn clipboard(&self) -> Option<&ClipboardPayload> {
        self.clipboard.as_ref()
    }

    /// Undo depth.
    pub fn history_depth(&self) -> usize {
        self.history.depth()
    }

    /// Place the caret, validating the block and the char boundary.
    pub fn set_caret(&mut self, block_id: BlockId, byte_offset: usize) -> PkmResult<()> {
        let content = self.block_content(block_id)?;
        let offset = clamp_offset(&content, byte_offset);
        self.caret = Some(Caret {
            block_id,
            byte_offset: offset,
        });
        self.selection = None;
        Ok(())
    }

    /// Set a selection, validating both carets.
    pub fn set_selection(&mut self, anchor: Caret, head: Caret) -> PkmResult<()> {
        let a = clamp_offset(&self.block_content(anchor.block_id)?, anchor.byte_offset);
        let h = clamp_offset(&self.block_content(head.block_id)?, head.byte_offset);
        self.selection = Some(Selection {
            anchor: Caret {
                block_id: anchor.block_id,
                byte_offset: a,
            },
            head: Caret {
                block_id: head.block_id,
                byte_offset: h,
            },
        });
        self.caret = Some(Caret {
            block_id: head.block_id,
            byte_offset: h,
        });
        Ok(())
    }

    /// Content of one block, or a [`PkmError::BlockNotFound`].
    pub(super) fn block_content(&self, block_id: BlockId) -> PkmResult<String> {
        self.doc
            .block(block_id)
            .map(|b| b.content.clone())
            .ok_or_else(|| PkmError::BlockNotFound(block_id.to_string()))
    }

    /// The caret or a [`PkmError::Internal`] when none is placed.
    pub(super) fn require_caret(&self) -> PkmResult<Caret> {
        self.caret
            .ok_or_else(|| PkmError::Internal("no caret in editor state".into()))
    }

    /// Apply one forward operation and record its inverse.
    pub(super) fn apply(&mut self, op: EditOp) -> PkmResult<()> {
        let inverse = op.inverse();
        self.doc.mutate(|tree| ops::apply_op(tree, &op))?;
        self.history.push(op, inverse, chrono::Utc::now());
        Ok(())
    }
}
