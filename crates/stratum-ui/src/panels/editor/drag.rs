//! Block drag-and-drop for the native editor.
//!
//! A drag starts only on a row's indent gutter — everything left of
//! [`row_text_left(depth)`](crate::panels::editor::rows::row_text_left) — so
//! pressing inside the text keeps the normal caret and text-selection
//! behaviour. While the pointer is down the view resolves a [`DropTarget`]; the
//! move itself is delegated to [`stratum_editor::EditorState::move_block`],
//! which records one reversible history entry. Escape cancels an in-flight drag.
//!
//! Every helper except [`drop_indicator`] is a pure function so the drag
//! geometry and target resolution can be tested without a window.

use gpui_kit::component::ActiveTheme;
use gpui_kit::*;
use stratum_editor::{BlockId, DecoratedRow, EditorState, VirtualLayout};

use crate::panels::editor::rows::{row_text_left, ROW_DEPTH_INDENT, ROW_WIDTH};
use crate::panels::editor::EditorView;

/// Pointer travel in window pixels before a press becomes a drag.
pub const DRAG_THRESHOLD_PX: f32 = 4.0;

/// The part of a row a pointer is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropZone {
    Above,
    Below,
    Into,
}

/// A resolved drop: insert the dragged block into `parent` at `index`
/// (`index` is counted after removing the dragged block, see
/// [`stratum_editor::MoveTarget`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropTarget {
    pub parent: Option<BlockId>,
    pub index: usize,
    /// Row the indicator is anchored to.
    pub row_index: usize,
    pub zone: DropZone,
}

/// An in-flight drag started on a row gutter.
#[derive(Debug, Clone, Copy)]
pub struct DragState {
    pub block_id: BlockId,
    pub origin: Point<Pixels>,
    pub active: bool,
}

/// The row whose indent gutter contains a content-space point.
///
/// `content_x`/`content_y` are relative to the list content, i.e. already
/// corrected for the scroll offset. The gutter is everything left of
/// [`row_text_left(depth)`](crate::panels::editor::rows::row_text_left), so a
/// press there never starts a text selection.
pub fn gutter_row_at(
    rows: &[DecoratedRow],
    layout: &VirtualLayout,
    content_x: f32,
    content_y: f32,
) -> Option<usize> {
    if content_x < 0.0 {
        return None;
    }
    let index = layout.block_at_offset(content_y)?;
    let row = rows.get(index)?;
    if content_x < row_text_left(row.depth) {
        Some(index)
    } else {
        None
    }
}

/// Above = top quarter of the row, Below = bottom quarter, Into = the rest.
pub fn drop_zone(row_height: f32, offset_in_row: f32) -> DropZone {
    let quarter = row_height * 0.25;
    if offset_in_row < quarter {
        DropZone::Above
    } else if offset_in_row >= row_height * 0.75 {
        DropZone::Below
    } else {
        DropZone::Into
    }
}

/// Resolve a drop target.
///
/// Returns None when the drop would be a no-op, would create a cycle
/// (destination is the dragged block or one of its descendants), or the hovered
/// row is the dragged block itself for Above/Below.
pub fn drop_target_for(
    state: &EditorState,
    rows: &[DecoratedRow],
    row_index: usize,
    zone: DropZone,
    dragged: BlockId,
) -> Option<DropTarget> {
    let hovered = rows.get(row_index)?.block_id;
    let dragged_block = state.document().block(dragged)?;

    let parent = match zone {
        DropZone::Above | DropZone::Below => {
            if hovered == dragged {
                return None;
            }
            state.document().block(hovered)?.parent_id
        }
        DropZone::Into => Some(hovered),
    };

    if let Some(parent_id) = parent {
        // `ancestors()` includes the parent itself, so this rejects both
        // `parent == dragged` and any descendant destination.
        if state
            .document()
            .tree()
            .ancestors(parent_id)
            .iter()
            .any(|ancestor| ancestor.id == dragged)
        {
            return None;
        }
    }

    // Resolve the destination group in document order and drop the dragged
    // block from it: the index must count the siblings that remain.
    let group: Vec<BlockId> = match parent {
        Some(parent_id) => state
            .document()
            .tree()
            .children(parent_id)
            .into_iter()
            .map(|block| block.id)
            .collect(),
        None => state
            .document()
            .tree()
            .roots()
            .into_iter()
            .map(|block| block.id)
            .collect(),
    };
    let filtered: Vec<BlockId> = group.into_iter().filter(|id| *id != dragged).collect();

    let index = match zone {
        DropZone::Above | DropZone::Below => {
            let position = filtered.iter().position(|id| *id == hovered)?;
            if zone == DropZone::Above {
                position
            } else {
                position + 1
            }
        }
        DropZone::Into => filtered.len(),
    };
    let to_left = index.checked_sub(1).map(|slot| filtered[slot]);

    // Suppress a drop that would leave the block exactly where it already is.
    if dragged_block.parent_id == parent && dragged_block.left_id == to_left {
        return None;
    }

    Some(DropTarget {
        parent,
        index,
        row_index,
        zone,
    })
}

/// Indicator geometry in content coordinates: `(x, y, width)`.
///
/// `x` follows the insertion depth (hovered depth for siblings, hovered depth +
/// 1 for Into); `y` is the row's top edge (Above) or bottom edge (Below/Into).
pub fn drop_indicator_rect(
    rows: &[DecoratedRow],
    layout: &VirtualLayout,
    target: &DropTarget,
) -> Option<(f32, f32, f32)> {
    let row = rows.get(target.row_index)?;
    let x = match target.zone {
        DropZone::Into => row_text_left(row.depth) + ROW_DEPTH_INDENT,
        DropZone::Above | DropZone::Below => row_text_left(row.depth),
    };
    let top = *layout.offsets().get(target.row_index)?;
    let height = *layout.heights().get(target.row_index)?;
    let y = match target.zone {
        DropZone::Above => top,
        DropZone::Below | DropZone::Into => top + height,
    };
    let width = (ROW_WIDTH - 12.0 - x).max(40.0);
    Some((x, y, width))
}

/// Whether the pointer has travelled past [`DRAG_THRESHOLD_PX`].
pub fn drag_has_started(origin: Point<Pixels>, current: Point<Pixels>) -> bool {
    let dx = f32::from(current.x) - f32::from(origin.x);
    let dy = f32::from(current.y) - f32::from(origin.y);
    dx * dx + dy * dy > DRAG_THRESHOLD_PX * DRAG_THRESHOLD_PX
}

/// Resolve a drop target for a window-space pointer position.
///
/// The point is converted into list content coordinates with the same arithmetic
/// as [`crate::panels::editor::rows::row_index_at_point`]. Returns None outside
/// the list bounds.
pub(crate) fn drop_target_at_point(
    view: &EditorView,
    position: Point<Pixels>,
) -> Option<DropTarget> {
    let drag = view.drag_state()?;
    let viewport = view.scroll.bounds();
    if position.x < viewport.origin.x
        || position.x > viewport.origin.x + viewport.size.width
        || position.y < viewport.origin.y
        || position.y > viewport.origin.y + viewport.size.height
    {
        return None;
    }
    let offset = view.scroll.offset();
    let content_y = f32::from(position.y) - f32::from(viewport.origin.y) - f32::from(offset.y);
    let index = view.layout.block_at_offset(content_y)?;
    let height = *view.layout.heights().get(index)?;
    let top = *view.layout.offsets().get(index)?;
    let zone = drop_zone(height, content_y - top);
    drop_target_for(&view.state, &view.rows, index, zone, drag.block_id)
}

/// Render the drop indicator bar for the resolved target.
pub(crate) fn drop_indicator(view: &EditorView, cx: &mut Context<EditorView>) -> AnyElement {
    let Some(target) = view.drop_target() else {
        return div().into_any_element();
    };
    let Some((x, y, width)) = drop_indicator_rect(&view.rows, &view.layout, target) else {
        return div().into_any_element();
    };
    let theme = cx.theme();
    let offset = view.scroll.offset();
    div()
        .absolute()
        .left(px(x + f32::from(offset.x)))
        .top(px(y + f32::from(offset.y)))
        .w(px(width))
        .h(px(2.0))
        .rounded(px(1.0))
        .bg(theme.colors.primary)
        .into_any_element()
}

#[cfg(test)]
mod tests;
