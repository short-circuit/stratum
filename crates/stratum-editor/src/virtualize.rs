//! Deterministic virtualization math for the editor list.
//!
//! [`VirtualLayout`] stores one estimated height per row and answers viewport
//! queries without touching any UI framework. The height formula is
//! deterministic (same inputs → same output) so golden tests and screenshots are
//! stable:
//!
//! `height = base(kind) + (ceil(content_len / 80) - 1) * 18`
//!
//! with `base` 24 for paragraphs/quote/list items, 32 for headings, 22 for code,
//! 180 for mermaid, 48 for display math, 24 for inline math and 12 for dividers.
//! Heights are clamped to a minimum of [`MIN_ROW_HEIGHT`] so the visible-window
//! bound holds even for malformed layouts.

use std::ops::Range;

use crate::render::{BlockKind, DecoratedRow};

/// Minimum row height used by the visible-window bound.
pub const MIN_ROW_HEIGHT: f32 = 12.0;

/// Estimated row heights for a page.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VirtualLayout {
    heights: Vec<f32>,
}

impl VirtualLayout {
    /// Build a layout from rendered rows.
    pub fn from_rows(rows: &[DecoratedRow]) -> Self {
        let heights = rows
            .iter()
            .map(|row| {
                let content_len = row.spans.iter().map(|s| s.text.len()).sum();
                estimate_height(&row.kind, content_len)
            })
            .collect();
        Self { heights }
    }

    /// Row heights.
    pub fn heights(&self) -> &[f32] {
        &self.heights
    }

    /// Cumulative offsets for every row (monotonic non-decreasing).
    pub fn offsets(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.heights.len());
        let mut at = 0.0f32;
        for height in &self.heights {
            out.push(at);
            at += height;
        }
        out
    }

    /// Total estimated height.
    pub fn total_height(&self) -> f32 {
        self.heights.iter().sum()
    }

    /// Rows intersecting the viewport plus `overscan_rows` on each side.
    ///
    /// The returned window is bounded by the viewport height divided by
    /// [`MIN_ROW_HEIGHT`] plus twice the overscan.
    pub fn visible_window(
        &self,
        scroll_top: f32,
        viewport_height: f32,
        overscan_rows: usize,
    ) -> Range<usize> {
        let len = self.heights.len();
        if len == 0 {
            return 0..0;
        }
        let offsets = self.offsets();
        let bottom = scroll_top + viewport_height.max(0.0);

        // First row whose bottom edge is past the viewport top.
        let mut first = offsets.partition_point(|offset| *offset <= scroll_top);
        if first > 0 && offsets[first - 1] + self.heights[first - 1] > scroll_top {
            first -= 1;
        }
        let mut last = offsets.partition_point(|offset| *offset < bottom);
        last = last.min(len);

        let start = first.saturating_sub(overscan_rows);
        let end = (last + overscan_rows).min(len);
        let max_rows = (viewport_height.max(0.0) / MIN_ROW_HEIGHT).ceil() as usize
            + overscan_rows.saturating_mul(2)
            + 2;
        let end = end.min(start + max_rows);
        start..end
    }

    /// Scroll offset that puts `index` at the top of the viewport.
    pub fn scroll_top_for_block(&self, index: usize) -> Option<f32> {
        self.offsets().get(index).copied()
    }

    /// Row index at `offset`, clamped to the last row at or beyond the end.
    pub fn block_at_offset(&self, offset: f32) -> Option<usize> {
        if self.heights.is_empty() {
            return None;
        }
        if offset < 0.0 {
            return None;
        }
        let total = self.total_height();
        if offset >= total {
            return Some(self.heights.len() - 1);
        }
        let offsets = self.offsets();
        let index = offsets.partition_point(|start| *start <= offset);
        Some(index.saturating_sub(1))
    }

    /// A stable anchor offset for `index` (row top + half its height).
    pub fn anchor_offset(&self, index: usize) -> Option<f32> {
        let offset = *self.offsets().get(index)?;
        let height = *self.heights.get(index)?;
        Some(offset + height / 2.0)
    }

    /// Replace the height of one row.
    pub fn set_height(&mut self, index: usize, height: f32) -> bool {
        if let Some(slot) = self.heights.get_mut(index) {
            *slot = height.max(MIN_ROW_HEIGHT);
            true
        } else {
            false
        }
    }
}

/// Deterministic height estimate for a row.
///
/// See the module docs for the exact formula. The result never goes below
/// [`MIN_ROW_HEIGHT`].
pub fn estimate_height(kind: &BlockKind, content_len: usize) -> f32 {
    let base = match kind {
        BlockKind::Paragraph | BlockKind::Quote | BlockKind::ListItem => 24.0,
        BlockKind::Heading(_) => 32.0,
        BlockKind::Code { .. } => 22.0,
        BlockKind::Mermaid => 180.0,
        BlockKind::Math { display: true } => 48.0,
        BlockKind::Math { display: false } => 24.0,
        BlockKind::Divider => 12.0,
    };
    let lines = (content_len as f32 / 80.0).ceil().max(1.0);
    (base + (lines - 1.0) * 18.0).max(MIN_ROW_HEIGHT)
}

#[cfg(test)]
mod tests;
