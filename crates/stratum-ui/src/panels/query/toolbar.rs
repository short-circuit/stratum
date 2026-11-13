//! Query panel header, mode tabs, raw editor and run/save/reset actions.
//!
//! These controls mirror the shipped `QueryPanel.desktop` toolbar: the
//! Builder/Datalog tab pair, the multiline Datalog editor, Run, Save Query and
//! Reset. The tab switches and run/reset transitions live on [`QueryView`];
//! this module only lays the controls out.

use gpui_kit::component::input::Textarea;
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::model::QueryTab;
use super::QueryView;

/// Render the panel title and the Builder/Datalog tab pair.
pub fn header(view: &QueryView, cx: &mut Context<QueryView>) -> impl IntoElement {
    let theme = cx.theme();
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let border = theme.colors.border;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;
    let tab = view.tab();

    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(16.0))
        .py(px(10.0))
        .border_b_1()
        .border_color(border)
        .child(
            div()
                .text_size(px(16.0))
                .font_weight(FontWeight::BOLD)
                .text_color(foreground)
                .child(t_global("query.datalog_query")),
        )
        .child(tab_button(
            "query-tab-builder",
            t_global("query.builder_mode"),
            tab == QueryTab::Builder,
            TabColors {
                muted,
                accent,
                accent_foreground,
            },
            cx,
            |view, cx| view.switch_tab(QueryTab::Builder, cx),
        ))
        .child(tab_button(
            "query-tab-datalog",
            t_global("query.datalog_mode"),
            tab == QueryTab::Datalog,
            TabColors {
                muted,
                accent,
                accent_foreground,
            },
            cx,
            |view, cx| view.switch_tab(QueryTab::Datalog, cx),
        ))
}

/// Colors used by the Builder/Datalog tab controls.
struct TabColors {
    muted: Hsla,
    accent: Hsla,
    accent_foreground: Hsla,
}

/// One tab button; the active tab uses the accent fill.
fn tab_button(
    id: &'static str,
    label: String,
    active: bool,
    colors: TabColors,
    cx: &mut Context<QueryView>,
    action: impl Fn(&mut QueryView, &mut Context<QueryView>) + 'static,
) -> impl IntoElement {
    let TabColors {
        muted,
        accent,
        accent_foreground,
    } = colors;
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .px(px(10.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .text_size(px(11.0))
        .bg(if active { accent } else { muted.opacity(0.12) })
        .text_color(if active { accent_foreground } else { muted })
        .cursor_pointer()
        .child(label)
        .on_click(cx.listener(move |view, _, _, cx| action(view, cx)))
}

/// Render the raw Datalog editor.
pub fn editor_section(view: &QueryView, cx: &mut Context<QueryView>) -> impl IntoElement {
    let _ = cx;
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .when_some(view.editor().cloned(), |element, editor| {
            element.child(
                div()
                    .id("query-datalog-editor")
                    .debug_selector(|| "query-datalog-editor".to_string())
                    .child(Textarea::new(&editor).h(px(150.0)).w_full()),
            )
        })
}

/// Render the Run / Save Query / Reset action row.
pub fn actions(view: &QueryView, cx: &mut Context<QueryView>) -> impl IntoElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;
    let border = theme.colors.border;

    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(run_button(view, accent, accent_foreground, cx))
        .child(save_button(muted, border, cx))
        .child(div().flex_1())
        .child(reset_button(muted, cx))
}

/// The Run button; disabled while a query is running.
fn run_button(
    view: &QueryView,
    accent: Hsla,
    accent_foreground: Hsla,
    cx: &mut Context<QueryView>,
) -> impl IntoElement {
    let running = view.state().running();
    let run_label = if running {
        t_global("query.running")
    } else {
        t_global("query.run_query")
    };
    div()
        .id("query-run")
        .debug_selector(|| "query-run".to_string())
        .px(px(12.0))
        .py(px(5.0))
        .rounded(px(6.0))
        .bg(accent)
        .text_color(accent_foreground)
        .text_size(px(12.0))
        .when(!running, |element| {
            element
                .cursor_pointer()
                .on_click(cx.listener(|view, _, _, cx| view.run(cx)))
        })
        .child(run_label)
}

/// The Save Query button.
fn save_button(muted: Hsla, border: Hsla, cx: &mut Context<QueryView>) -> impl IntoElement {
    div()
        .id("query-save")
        .debug_selector(|| "query-save".to_string())
        .px(px(12.0))
        .py(px(5.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(border)
        .text_color(muted)
        .text_size(px(12.0))
        .cursor_pointer()
        .child(t_global("query.save_query"))
        .on_click(cx.listener(|view, _, _, cx| view.open_save_dialog(cx)))
}

/// The Reset action.
fn reset_button(muted: Hsla, cx: &mut Context<QueryView>) -> impl IntoElement {
    div()
        .id("query-reset")
        .debug_selector(|| "query-reset".to_string())
        .px(px(8.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .text_color(muted)
        .text_size(px(12.0))
        .cursor_pointer()
        .child(t_global("common.reset"))
        .on_click(cx.listener(|view, _, window, cx| view.reset(window, cx)))
}

/// Render the run failure message.
pub fn error_alert(error: String, cx: &mut Context<QueryView>) -> impl IntoElement {
    let theme = cx.theme();
    let danger = theme.colors.danger;
    let _ = cx;
    div()
        .px(px(10.0))
        .py(px(8.0))
        .rounded(px(6.0))
        .text_size(px(12.0))
        .text_color(danger)
        .child(error)
}
