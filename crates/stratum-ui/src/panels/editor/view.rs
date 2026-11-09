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

use std::rc::Rc;
use std::time::Instant;

use gpui_kit::component::{v_virtual_list, ActiveTheme};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::panels::editor::context_menu::context_menu_popup;
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
        let scroll = self.scroll.clone();
        let rows = v_virtual_list(
            entity.clone(),
            "editor-rows",
            sizes,
            move |view, range, window, cx| {
                // Asset rendering (math/Mermaid) is budgeted separately: it is
                // measured here but excluded from the row-materialization figure
                // the 55 fps gate consumes, because one diagram render can
                // legitimately exceed the per-frame budget and is cached after
                // the first paint.
                let assets_started = Instant::now();
                view.ensure_assets_for_range(range.clone());
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

        div()
            .id("editor-root")
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if input::handle_key_down(this, event, window, cx) {
                    cx.stop_propagation();
                }
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
            .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                input::handle_click(this, event, window, cx);
            }))
            .child(badge_strip(self, cx))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .overflow_hidden()
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
                    .when(show_context_menu, |element| {
                        element.child(context_menu_popup(self, cx))
                    }),
            )
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
            })
    }
}
