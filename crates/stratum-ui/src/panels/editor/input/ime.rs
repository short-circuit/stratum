//! [`EntityInputHandler`] implementation for the native block editor.
//!
//! IME, dead keys and normal typing all arrive here as UTF-16 ranges; they are
//! mapped onto the engine's UTF-8 byte offsets before the edit is applied.

use std::ops::Range;

use gpui_kit::component::ActiveTheme;
use gpui_kit::*;
use stratum_editor::{byte_to_utf16, utf16_to_byte};

use crate::panels::editor::rows::{
    inline_offset, marker_prefix_len, row_font_size, row_index_at_point, row_text_left,
    source_offset_for_visible, visible_runs,
};
use crate::panels::editor::EditorView;

use super::{caret_text_prefix, refresh_overlay};

impl EntityInputHandler for EditorView {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let caret = self.state.caret()?;
        let content = self.state.document().block(caret.block_id)?.content.clone();
        let start = utf16_to_byte(&content, range.start)?;
        let end = utf16_to_byte(&content, range.end)?;
        *adjusted_range = Some(range);
        Some(content[start..end].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let caret = self.state.caret()?;
        let content = self.state.document().block(caret.block_id)?.content.clone();
        let head = byte_to_utf16(&content, caret.byte_offset)?;
        if let Some(selection) = self.state.selection() {
            if selection.anchor.block_id == selection.head.block_id {
                let anchor = byte_to_utf16(&content, selection.anchor.byte_offset)?;
                let reversed = anchor > head;
                let (start, end) = if reversed {
                    (head, anchor)
                } else {
                    (anchor, head)
                };
                return Some(UTF16Selection {
                    range: start..end,
                    reversed,
                });
            }
        }
        Some(UTF16Selection {
            range: head..head,
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.state
            .marked_range_utf16()
            .map(|(start, end)| start..end)
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.state.ime_commit().is_ok() {
            self.after_edit(cx);
        }
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let result = match range {
            Some(range) => self
                .state
                .ime_replace(range, text)
                .and_then(|()| self.state.ime_commit()),
            None => self.state.insert_text(text),
        };
        match result {
            Ok(()) => {
                refresh_overlay(self);
                self.after_edit(cx);
            }
            Err(error) => {
                self.save.fail(error.to_string());
                cx.notify();
            }
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range.unwrap_or_else(|| {
            self.state
                .caret()
                .map(|caret| {
                    let content = self
                        .state
                        .document()
                        .block(caret.block_id)
                        .map(|block| block.content.clone())
                        .unwrap_or_default();
                    let utf16 = byte_to_utf16(&content, caret.byte_offset).unwrap_or(0);
                    utf16..utf16
                })
                .unwrap_or(0..0)
        });
        if self.state.ime_replace(range, new_text).is_ok() {
            self.rebuild_rows();
            cx.notify();
        }
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        _element_bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        caret_text_prefix(self, window, cx).map(|(bounds, _)| bounds)
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<usize> {
        let index = row_index_at_point(self, point)?;
        let row = self.rows.get(index)?.clone();
        let content = self.state.document().block(row.block_id)?.content.clone();
        let theme = cx.theme();
        let (text, runs) = visible_runs(&row, theme);
        let font_size = row_font_size(&row, theme);
        let viewport = self.scroll.bounds();
        let text_x = f32::from(point.x)
            - f32::from(viewport.origin.x)
            - row_text_left(row.depth)
            - inline_offset(self, &row);
        let layout = window
            .text_system()
            .layout_line(&text, font_size, &runs, None);
        let visible_byte = layout
            .index_for_x(px(text_x.max(0.0)))
            .unwrap_or(text.len());
        let prefix = marker_prefix_len(&content, row.marker.map(|marker| marker.as_str()));
        let source = source_offset_for_visible(&row, visible_byte) + prefix;
        byte_to_utf16(&content, source)
    }
}
