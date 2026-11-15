//! Render implementation for the native graph panel.
//!
//! The panel paints a cached software-rendered frame instead of running a
//! per-frame simulation. This module owns the title header, the loading, empty
//! and failure states, the toolbar/settings composition and the node-cap banner;
//! the cached frame is produced by [`super::GraphView::ensure_frame`] before the
//! body is built.

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::canvas;
use super::model::banner_text;
use super::settings;
use super::toolbar;
use super::{GraphStatus, GraphView};

impl Render for GraphView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.advance_tween(window);
        self.poll_layout(cx);
        // A worker-backed layout only completes between frames; keep painting
        // the previous image while asking for the next frame to poll it again.
        if self.pending_layout.is_some() {
            window.request_animation_frame();
        }
        self.ensure_frame();
        let theme = cx.theme();
        let background = theme.colors.background;
        let foreground = theme.colors.foreground;
        let border = theme.colors.border;
        let muted = theme.colors.muted_foreground;
        let banner = self.filtered.as_ref().and_then(banner_text);
        let software_fallback = self.software_renderer();
        let status = self.status.clone();
        let has_frame = self.image.is_some();
        let body: AnyElement = match status {
            GraphStatus::Failed(message) => {
                let danger = theme.colors.danger;
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size_full()
                    .px(px(24.0))
                    .child(div().text_size(px(12.0)).text_color(danger).child(message))
                    .into_any_element()
            }
            GraphStatus::Empty => empty_state(cx).into_any_element(),
            GraphStatus::Loading | GraphStatus::Idle => div()
                .flex()
                .items_center()
                .justify_center()
                .size_full()
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(muted)
                        .child(t_global("common.loading")),
                )
                .into_any_element(),
            GraphStatus::Ready => {
                if has_frame {
                    div()
                        .size_full()
                        .child(canvas::graph_canvas(self, cx))
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size_full()
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(muted)
                                .child(t_global("common.loading")),
                        )
                        .into_any_element()
                }
            }
        };
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(background)
            .text_color(foreground)
            .child(header(cx))
            .child(toolbar::toolbar(self, cx))
            .when(self.settings_open, |element| {
                element.child(settings::settings(self, cx))
            })
            .when_some(banner, |element, text| {
                element.child(
                    div()
                        .mx(px(16.0))
                        .my(px(6.0))
                        .px(px(10.0))
                        .py(px(4.0))
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(border)
                        .text_size(px(10.0))
                        .text_color(muted)
                        .child(text),
                )
            })
            .when(software_fallback, |element| {
                element.child(
                    div()
                        .mx(px(16.0))
                        .my(px(6.0))
                        .px(px(10.0))
                        .py(px(4.0))
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(border)
                        .text_size(px(10.0))
                        .text_color(muted)
                        .child(t_global("graph.fallback_notice_webgl")),
                )
            })
            .child(div().flex_1().overflow_hidden().child(body))
    }
}

/// The panel title header.
fn header(cx: &mut Context<GraphView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(10.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .child(
            div()
                .text_size(px(16.0))
                .font_weight(FontWeight::BOLD)
                .child(t_global("mobile_nav.graph")),
        )
}

/// The localized empty-state hint.
fn empty_state(cx: &mut Context<GraphView>) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .size_full()
        .gap(px(6.0))
        .px(px(24.0))
        .child(
            div()
                .text_size(px(15.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child(t_global("graph.no_graph_data_yet")),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(theme.colors.muted_foreground)
                .child(t_global(
                    "graph.create_pages_with_wiki_links_to_build_connections",
                )),
        )
}
