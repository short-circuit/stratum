//! Highlight-on-open support for the native editor.
//!
//! A reminder or backlink can request that a page opens with a specific block
//! highlighted. [`EditorView::reveal_block`] locates the row, scrolls to the
//! layout offset and records the highlight; the next edit (or a reveal of
//! another block) clears or replaces it.
//!
//! The state transition is a pure model ([`RevealState`]) so the contract tests
//! can exercise the scroll target and highlight replacement without a window.

use gpui_kit::*;
use stratum_editor::{DecoratedRow, VirtualLayout};

use super::EditorView;

/// Pure highlight-on-open state shared by the view and its tests.
pub(crate) struct RevealState {
    highlight: Option<String>,
    scroll_offset: f32,
}

impl RevealState {
    /// Create an empty reveal state.
    pub(crate) fn new() -> Self {
        Self {
            highlight: None,
            scroll_offset: 0.0,
        }
    }

    /// The highlighted block id, if any.
    pub(crate) fn highlight(&self) -> Option<&str> {
        self.highlight.as_deref()
    }

    /// The scroll offset requested by the last reveal.
    pub(crate) fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    /// Locate `block_id` and record the reveal.
    ///
    /// Returns `false` for an unknown block, leaving both the highlight and the
    /// scroll offset untouched.
    pub(crate) fn reveal(
        &mut self,
        rows: &[DecoratedRow],
        layout: &VirtualLayout,
        block_id: &str,
    ) -> bool {
        let Some(index) = rows
            .iter()
            .position(|row| row.block_id.to_string() == block_id)
        else {
            return false;
        };
        let Some(top) = layout.scroll_top_for_block(index) else {
            return false;
        };
        self.scroll_offset = top;
        self.highlight = Some(block_id.to_string());
        true
    }

    /// Clear the highlight after an edit.
    pub(crate) fn clear(&mut self) {
        self.highlight = None;
    }
}

impl EditorView {
    /// Whether the editor sizes itself to its content.
    pub fn auto_height(&self) -> bool {
        self.auto_height
    }

    /// Size the editor to its content instead of the parent height.
    ///
    /// The journal stack embeds several editors inside one scroll container, so
    /// each editor must contribute its full row height to the stack rather than
    /// filling a viewport it does not own.
    pub fn set_auto_height(&mut self, auto_height: bool, cx: &mut Context<Self>) {
        if self.auto_height == auto_height {
            return;
        }
        self.auto_height = auto_height;
        cx.notify();
    }

    /// Highlight and scroll to `block_id`.
    ///
    /// Returns `false` when the block is not part of the current document; the
    /// highlight and scroll offset are left untouched in that case.
    pub fn reveal_block(&mut self, block_id: &str) -> bool {
        if !self.reveal.reveal(&self.rows, &self.layout, block_id) {
            return false;
        }
        let top = self.reveal.scroll_offset();
        self.scroll.set_offset(point(px(0.0), px(-top)));
        true
    }

    /// The currently highlighted block id, if any.
    pub fn highlight_block(&self) -> Option<&str> {
        self.reveal.highlight()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;
    use stratum_editor::{dto_to_blocks, EditorDocument, EditorState};

    use crate::panels::editor::build_rows_for_state;
    use crate::provider::{FixtureProvider, ShellData, FIXTURE_PAGE};

    fn fixture_rows() -> (Vec<DecoratedRow>, VirtualLayout) {
        let provider = FixtureProvider::new();
        let blocks = provider.load_blocks(FIXTURE_PAGE).expect("fixture page");
        let document = EditorDocument::from_blocks(FIXTURE_PAGE, dto_to_blocks(blocks))
            .expect("fixture document");
        let state = EditorState::new(document);
        let rows = build_rows_for_state(&state, true);
        let layout = VirtualLayout::from_rows(&rows);
        (rows, layout)
    }

    #[test]
    fn unknown_block_leaves_the_offset_and_highlight_untouched() {
        let (rows, layout) = fixture_rows();
        let mut reveal = RevealState::new();
        assert!(!reveal.reveal(&rows, &layout, "missing-block"));
        assert_eq!(reveal.scroll_offset(), 0.0);
        assert_eq!(reveal.highlight(), None);
    }

    #[test]
    fn known_block_records_the_layout_offset_and_highlight() {
        let (rows, layout) = fixture_rows();
        let target = rows[1].block_id.to_string();
        let expected = layout.scroll_top_for_block(1).expect("offset");
        let mut reveal = RevealState::new();
        assert!(reveal.reveal(&rows, &layout, &target));
        assert_eq!(reveal.scroll_offset(), expected);
        assert_eq!(reveal.highlight(), Some(target.as_str()));
    }

    #[test]
    fn a_second_reveal_replaces_the_highlight() {
        let (rows, layout) = fixture_rows();
        let first = rows[1].block_id.to_string();
        let second = rows[2].block_id.to_string();
        let mut reveal = RevealState::new();
        assert!(reveal.reveal(&rows, &layout, &first));
        assert!(reveal.reveal(&rows, &layout, &second));
        assert_eq!(reveal.highlight(), Some(second.as_str()));
        reveal.clear();
        assert_eq!(reveal.highlight(), None);
    }
}
