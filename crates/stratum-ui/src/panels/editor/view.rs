//! `Render` implementation for the native block editor.
//!
//! The view is a virtualized list of [`DecoratedRow`] values. Rows come from
//! [`crate::panels::editor::build_rows_for_state`]; math/Mermaid assets are
//! rendered lazily for the visible window only, with a one-asset-per-frame
//! budget so a viewport full of diagrams cannot stall the 55 fps work budget.
//!
//! The input handler is registered on a paint-phase canvas that covers the
//! list; the list itself owns scrolling, so the canvas never intercepts mouse
//! events (it has no hitbox).

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

use gpui_kit::component::{v_virtual_list, ActiveTheme};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::panels::editor::asset_state::drop_target_block;
use crate::panels::editor::attachments::attachments_panel;
use crate::panels::editor::context_menu::context_menu_popup;
use crate::panels::editor::drag;
use crate::panels::editor::input;
use crate::panels::editor::overlay::{badge_strip, hover_preview, overlay_popup, save_chip};
use crate::panels::editor::rows::{row_element, ROW_WIDTH};
use crate::panels::editor::{EditorView, OverlayState};

impl Render for EditorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Reset the lazy asset budget once per frame. The virtual list may call
        // the row builder more than once while measuring, so the budget lives on
        // the view rather than in the closure.
        self.asset_budget = 1;
        let theme = cx.theme().clone();
        let sizes: Rc<Vec<Size<Pixels>>> = Rc::new(
            self.layout
                .heights()
                .iter()
                .map(|height| size(px(ROW_WIDTH), px(*height)))
                .collect(),
        );
        let entity = cx.entity();
        let drop_entity = entity.clone();
        let scroll = self.scroll.clone();
        let rows = v_virtual_list(
            entity.clone(),
            "editor-rows",
            sizes,
            move |view, range, window, cx| {
                // Asset rendering (math/Mermaid/attachment thumbnails) is
                // budgeted separately: it is measured here but excluded from
                // the row-materialization figure the 55 fps gate consumes,
                // because one diagram or thumbnail decode can legitimately
                // exceed the per-frame budget and is cached after the first
                // paint.
                let assets_started = Instant::now();
                view.ensure_assets_for_range(range.clone());
                view.ensure_thumbnails_for_range(range.clone());
                let assets_ms = assets_started.elapsed().as_secs_f64() * 1000.0;
                let started = Instant::now();
                let visible: Vec<(usize, stratum_editor::DecoratedRow)> = range
                    .clone()
                    .filter_map(|index| view.rows.get(index).cloned().map(|row| (index, row)))
                    .collect();
                let mut out = Vec::with_capacity(visible.len());
                for (index, row) in visible {
                    out.push(row_element(view, index, &row, window, cx));
                }
                view.metrics.record_assets(assets_ms);
                view.metrics
                    .record(started.elapsed().as_secs_f64() * 1000.0, range.len());
                out
            },
        )
        .track_scroll(&scroll)
        .flex_1();

        let focus = self.focus.clone();
        let input_entity = entity.clone();
        let input_canvas = canvas(
            move |bounds, _window, _cx| bounds,
            move |bounds, _, window, cx| {
                window.handle_input(
                    &focus,
                    ElementInputHandler::new(bounds, input_entity.clone()),
                    cx,
                );
            },
        )
        .absolute()
        .inset_0();

        let show_overlay = !self.read_mode && !matches!(self.overlay, OverlayState::None);
        let show_preview = !self.read_mode && self.hover_preview.is_some();
        let show_context_menu = !self.read_mode && self.context_menu.is_some();

        let auto_height = self.auto_height;
        let content_height = self.total_height();
        let root = div()
            .id("editor-root")
            .relative()
            .flex()
            .flex_col()
            .bg(theme.colors.background)
            .when(auto_height, |element| element.w_full())
            .when(!auto_height, |element| element.size_full())
            .text_color(theme.colors.foreground)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if input::handle_key_down(this, event, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .on_key_up(cx.listener(|this, event: &KeyUpEvent, _window, cx| {
                input::handle_key_up(this, event, cx);
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    input::handle_mouse_down(this, event, window, cx);
                    cx.stop_propagation();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    input::handle_right_mouse_down(this, event, window, cx);
                    cx.stop_propagation();
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                input::handle_mouse_move(this, event, window, cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, window, cx| {
                    input::handle_mouse_up(this, event, window, cx);
                }),
            )
            .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                input::handle_click(this, event, window, cx);
            }))
            .can_drop(|_, _, _| true)
            .on_drop::<ExternalPaths>(move |paths, window, cx| {
                // External file drops insert attachment references into the
                // row under the pointer, falling back to the caret block.
                let position = window.mouse_position();
                let dropped: Vec<PathBuf> = paths.0.iter().cloned().collect();
                drop_entity.update(cx, |view, cx| {
                    let block_id = drop_target_block(view, position);
                    view.start_import_paths(dropped, block_id, cx);
                });
            })
            .child(badge_strip(self, cx))
            .child({
                let mut container = div().relative().overflow_hidden();
                container = if auto_height {
                    // A stacked journal editor contributes its full content
                    // height; an empty page still reserves room for the empty
                    // message so the section is visible.
                    let height = if self.rows.is_empty() {
                        120.0
                    } else {
                        content_height
                    };
                    container.h(px(height))
                } else {
                    container.flex_1()
                };
                container
                    .when(self.rows.is_empty(), |element| {
                        element.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size_full()
                                .text_size(px(13.0))
                                .text_color(theme.colors.muted_foreground)
                                .child(t_global("page.prose_empty")),
                        )
                    })
                    .when(!self.rows.is_empty(), |element| {
                        element.child(rows).child(input_canvas)
                    })
                    .when(
                        self.drag_state().is_some_and(|drag| drag.active)
                            && self.drop_target().is_some(),
                        |element| element.child(drag::drop_indicator(self, cx)),
                    )
                    .when(show_context_menu, |element| {
                        element.child(context_menu_popup(self, cx))
                    })
            })
            // Journal sections stack editors without the page-level attachment
            // panel, matching the shipped JournalPanel (which never renders
            // AttachmentPanel). The page editor keeps it.
            .when(!auto_height, |element| {
                element.child(attachments_panel(self, cx))
            })
            .when(!self.read_mode, |element| {
                element.child(save_chip(self, cx))
            })
            .when(show_overlay, |element| {
                element.child(overlay_popup(self, cx))
            })
            .when(show_preview, |element| {
                element.child(
                    div()
                        .absolute()
                        .right(px(16.0))
                        .top(px(56.0))
                        .w(px(260.0))
                        .rounded(px(8.0))
                        .bg(theme.colors.background)
                        .border_1()
                        .border_color(theme.colors.border)
                        .p(px(8.0))
                        .child(hover_preview(self, cx)),
                )
            });
        root
    }
}
