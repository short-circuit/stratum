//! Applying AI results to the document.
//!
//! AI slash actions resolve to an [`AiCall`] whose [`AiCallScope`] decides how
//! the returned markdown lands: replace the page body, insert at the caret, or
//! add a sibling block. Every path records one undoable `SpliceBlocks` (or text
//! insertion) entry so the result is a normal edit, never an untracked mutation.

use pkm_block::Block;
use pkm_core::PkmResult;
use uuid::Uuid;

use super::splice::LinkChange;
use super::{normalize_paste, Caret, EditOp, EditorState};
use crate::document::EditorDocument;
use crate::menus::{AiCall, AiCallScope};

impl EditorState {
    /// Replace the whole document body with parsed `markdown`.
    ///
    /// The existing blocks are snapshotted as `removed` and the parsed blocks are
    /// inserted parents-before-children, so the replacement is one undoable
    /// history entry. The open document's frontmatter is untouched. An empty
    /// parsed body becomes one empty block so the tree is never empty.
    pub fn replace_from_markdown(&mut self, markdown: &str) -> PkmResult<()> {
        let parsed = EditorDocument::open(self.doc.path(), markdown)?;
        let mut inserted: Vec<Block> = parsed.blocks().into_iter().cloned().collect();
        if inserted.is_empty() {
            inserted.push(Block::new(Uuid::new_v4(), String::new()));
        }
        let removed: Vec<Block> = self.doc.blocks().into_iter().cloned().collect();
        let first = inserted[0].id;
        self.apply(EditOp::SpliceBlocks {
            removed,
            inserted,
            links: Vec::new(),
            contents: Vec::new(),
        })?;
        self.selection = None;
        self.caret = Some(Caret {
            block_id: first,
            byte_offset: 0,
        });
        Ok(())
    }

    /// Insert parsed `markdown` as sibling blocks after the caret block.
    ///
    /// Parsed root blocks are re-parented to the caret block's parent and chained
    /// after the caret block; the sibling that followed the caret is re-linked to
    /// the last inserted block. The whole operation is one undoable
    /// `SpliceBlocks` entry, and the caret ends at the end of the last inserted
    /// block. An empty parse is a no-op.
    pub fn insert_markdown_after_caret(&mut self, markdown: &str) -> PkmResult<()> {
        let caret = self.require_caret()?;
        let caret_parent = self.doc.block(caret.block_id).and_then(|b| b.parent_id);
        let parsed = EditorDocument::open(self.doc.path(), markdown)?;
        let mut inserted: Vec<Block> = parsed.blocks().into_iter().cloned().collect();
        if inserted.is_empty() {
            return Ok(());
        }

        // Root blocks are the parsed sibling chain heads; nested blocks keep
        // their internal parent/left links.
        let mut root_ids = Vec::new();
        for block in inserted.iter_mut() {
            if block.parent_id.is_none() {
                root_ids.push(block.id);
            }
        }
        if root_ids.is_empty() {
            return Ok(());
        }

        let mut left = Some(caret.block_id);
        for id in &root_ids {
            if let Some(block) = inserted.iter_mut().find(|block| block.id == *id) {
                block.parent_id = caret_parent;
                block.left_id = left;
            }
            left = Some(*id);
        }
        let last = *root_ids.last().expect("root_ids is non-empty");

        // Re-link the sibling that followed the caret so the chain stays whole.
        let mut links = Vec::new();
        if let Some(next) =
            self.doc.tree().all_blocks().find(|block| {
                block.left_id == Some(caret.block_id) && block.parent_id == caret_parent
            })
        {
            links.push(LinkChange {
                block_id: next.id,
                old_parent: next.parent_id,
                new_parent: next.parent_id,
                old_left: next.left_id,
                new_left: Some(last),
            });
        }

        self.apply(EditOp::SpliceBlocks {
            removed: Vec::new(),
            inserted,
            links,
            contents: Vec::new(),
        })?;
        let len = self.block_content(last)?.len();
        self.selection = None;
        self.caret = Some(Caret {
            block_id: last,
            byte_offset: len,
        });
        Ok(())
    }

    /// Apply one AI result according to the call's insertion scope.
    pub fn apply_ai_content(&mut self, call: &AiCall, content: &str) -> PkmResult<()> {
        match call.scope {
            AiCallScope::ReplacePage => self.replace_from_markdown(content),
            AiCallScope::InsertAtCaret => self.insert_text(&normalize_paste(content)),
            AiCallScope::InsertBlockAfterCaret => {
                self.insert_markdown_after_caret(&format!("```mermaid\n{}\n```", content.trim()))
            }
        }
    }
}
