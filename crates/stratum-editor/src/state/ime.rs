//! IME composition state and UTF-16/UTF-8 offset translation.
//!
//! Platform input methods report marked ranges and commit text as UTF-16 code
//! units. Composition updates rewrite the block content without touching the
//! undo history; committing pushes exactly one [`super::EditOp::SetContent`]
//! entry, and cancelling restores the pre-composition content.

use std::ops::Range;

use pkm_block::ops;
use pkm_block::BlockId;
use pkm_core::{PkmError, PkmResult};

use super::{Caret, EditorState};

/// An active IME composition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composition {
    pub block_id: BlockId,
    pub start_utf16: usize,
    pub end_utf16: usize,
    pub marked: String,
    pub original_content: String,
}

impl EditorState {
    /// Begin/replace an IME composition. Does not push history.
    pub fn ime_replace(&mut self, utf16_range: Range<usize>, text: &str) -> PkmResult<()> {
        let block_id = match &self.composition {
            Some(c) => c.block_id,
            None => self.require_caret()?.block_id,
        };
        let content = self.block_content(block_id)?;
        if self.composition.is_none() {
            self.composition = Some(Composition {
                block_id,
                start_utf16: utf16_range.start,
                end_utf16: utf16_range.end,
                marked: String::new(),
                original_content: content.clone(),
            });
        }
        let start = utf16_to_byte(&content, utf16_range.start)
            .ok_or_else(|| PkmError::Validation("IME start is not a UTF-16 boundary".into()))?;
        let end = utf16_to_byte(&content, utf16_range.end)
            .ok_or_else(|| PkmError::Validation("IME end is not a UTF-16 boundary".into()))?;
        let new_content = format!("{}{}{}", &content[..start], text, &content[end..]);
        self.doc
            .mutate(|tree| ops::update_content(tree, block_id, new_content.clone()))?;
        let marked_utf16 = text.encode_utf16().count();
        if let Some(c) = self.composition.as_mut() {
            c.start_utf16 = utf16_range.start;
            c.end_utf16 = utf16_range.start + marked_utf16;
            c.marked = text.to_string();
        }
        self.caret = Some(Caret {
            block_id,
            byte_offset: start + text.len(),
        });
        Ok(())
    }

    /// Commit the active composition as one history entry.
    pub fn ime_commit(&mut self) -> PkmResult<()> {
        let Some(composition) = self.composition.take() else {
            return Ok(());
        };
        let new = self.block_content(composition.block_id)?;
        if new == composition.original_content {
            return Ok(());
        }
        let op = super::EditOp::SetContent {
            block_id: composition.block_id,
            old: composition.original_content,
            new,
        };
        let inverse = op.inverse();
        self.history.push(op, inverse, chrono::Utc::now());
        Ok(())
    }

    /// Cancel the active composition, restoring the pre-composition content.
    pub fn ime_cancel(&mut self) -> PkmResult<()> {
        let Some(composition) = self.composition.take() else {
            return Ok(());
        };
        self.doc.mutate(|tree| {
            ops::update_content(
                tree,
                composition.block_id,
                composition.original_content.clone(),
            )
        })?;
        self.caret = Some(Caret {
            block_id: composition.block_id,
            byte_offset: 0,
        });
        Ok(())
    }

    /// The marked UTF-16 range of the active composition.
    pub fn marked_range_utf16(&self) -> Option<(usize, usize)> {
        self.composition
            .as_ref()
            .map(|c| (c.start_utf16, c.end_utf16))
    }
}

/// Convert a UTF-16 offset into a UTF-8 byte offset.
///
/// Returns `None` when `off` is past the end or lands inside a surrogate pair.
pub fn utf16_to_byte(text: &str, off: usize) -> Option<usize> {
    let mut units = 0usize;
    for (byte, ch) in text.char_indices() {
        if units == off {
            return Some(byte);
        }
        units += ch.len_utf16();
        if units > off {
            return None;
        }
    }
    (units == off).then_some(text.len())
}

/// Convert a UTF-8 byte offset into a UTF-16 offset.
///
/// Returns `None` when `byte` is past the end or not on a char boundary.
pub fn byte_to_utf16(text: &str, byte: usize) -> Option<usize> {
    if byte > text.len() || !text.is_char_boundary(byte) {
        return None;
    }
    Some(text[..byte].encode_utf16().count())
}
