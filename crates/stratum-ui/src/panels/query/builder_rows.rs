//! Visual filter builder rows and generated-query preview.
//!
//! The builder mirrors the shipped `BuilderMode` / `ConditionRow`: one row per
//! [`stratum_app::query::builder::FilterSpec`] with a combiner toggle, field and
//! operator choosers, a value input, move controls and remove. All query
//! emission/validation comes from `stratum_app::query::builder`; this module
//! only renders and dispatches the model transitions.

use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::model::PanelState;
use super::QueryView;

/// Render the builder tab.
pub fn builder_section(view: &QueryView, cx: &mut Context<QueryView>) -> impl IntoElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let foreground = theme.colors.foreground;
    let danger = theme.colors.danger;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;
    let border = theme.colors.border;

    let state = view.state();
    let rows = state.rows();

    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .when(state.parse_notice(), |element| element.child(notice(muted)))
        .child(toolbar(muted, accent, accent_foreground, cx))
        .child(header_row(muted))
        .when(rows.is_empty(), |element| element.child(empty_hint(muted)))
        .children((0..rows.len()).map(|index| row(view, index, foreground, muted, border, cx)))
        .child(generated_preview(
            view, foreground, muted, danger, accent, cx,
        ))
}

/// The notice shown when the raw buffer could not be parsed into rows.
fn notice(muted: Hsla) -> impl IntoElement {
    div()
        .px(px(10.0))
        .py(px(6.0))
        .rounded(px(6.0))
        .bg(muted.opacity(0.12))
        .text_size(px(11.0))
        .text_color(muted)
        .child(t_global("query.builder_not_parseable"))
}

/// The builder hint and the add-condition control.
fn toolbar(
    muted: Hsla,
    accent: Hsla,
    accent_foreground: Hsla,
    cx: &mut Context<QueryView>,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_size(px(11.0))
                .text_color(muted)
                .child(t_global("query.builder_hint")),
        )
        .child(
            div()
                .id("query-add-condition")
                .debug_selector(|| "query-add-condition".to_string())
                .px(px(10.0))
                .py(px(4.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(accent)
                .text_color(accent_foreground)
                .text_size(px(11.0))
                .cursor_pointer()
                .child(t_global("query.add_condition"))
                .on_click(cx.listener(|view, _, _, cx| {
                    view.state_mut().add_row();
                    cx.notify();
                })),
        )
}

/// The empty-builder hint.
fn empty_hint(muted: Hsla) -> impl IntoElement {
    div()
        .text_size(px(11.0))
        .text_color(muted)
        .child(t_global("query.builder_empty"))
}

/// The generated-query preview with the copy-to-editor action.
fn generated_preview(
    view: &QueryView,
    foreground: Hsla,
    muted: Hsla,
    danger: Hsla,
    accent: Hsla,
    cx: &mut Context<QueryView>,
) -> impl IntoElement {
    let generated = view.state().generated();
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .p(px(10.0))
        .rounded(px(6.0))
        .bg(muted.opacity(0.10))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if generated.error.is_some() {
                            danger
                        } else {
                            muted
                        })
                        .child(t_global("query.generated_query")),
                )
                .when(generated.error.is_none(), |element| {
                    element.child(
                        div()
                            .id("query-copy")
                            .debug_selector(|| "query-copy".to_string())
                            .text_size(px(10.0))
                            .text_color(accent)
                            .cursor_pointer()
                            .child(t_global("query.copy_to_editor"))
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.copy_to_editor(window, cx);
                            })),
                    )
                }),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(if generated.error.is_some() {
                    danger
                } else {
                    foreground
                })
                .child(match generated.error {
                    Some(error) => t_global(error.key()),
                    None => generated.datalog,
                }),
        )
}

/// The column labels above the builder rows.
fn header_row(muted: Hsla) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .text_size(px(10.0))
        .text_color(muted)
        .child(div().w(px(58.0)).child(t_global("query.combiner")))
        .child(div().w(px(120.0)).child(t_global("query.field")))
        .child(div().w(px(110.0)).child(t_global("query.op")))
        .child(div().flex_1().child(t_global("query.value")))
        .child(div().w(px(150.0)))
}

/// Render one condition row.
fn row(
    view: &QueryView,
    index: usize,
    foreground: Hsla,
    muted: Hsla,
    border: Hsla,
    cx: &mut Context<QueryView>,
) -> impl IntoElement {
    let Some(spec) = view.state().rows().get(index) else {
        return div();
    };
    let id = spec.id.clone();
    let field_label = t_global(spec.field.label_key());
    let op_label = t_global(spec.op.label_key());
    let combiner_label = t_global(match spec.combiner {
        stratum_app::query::builder::Combiner::And => "query.and",
        stratum_app::query::builder::Combiner::Or => "query.or",
    });
    let needs_value = spec.op.needs_value();
    let input = view.row_input(&id).cloned();

    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .p(px(6.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(border)
        .child(combiner_control(index, combiner_label, muted, cx))
        .child(field_control(index, field_label, foreground, muted, cx))
        .child(op_control(index, op_label, foreground, muted, cx))
        .child(value_control(input, needs_value, muted))
        .child(row_actions(index, muted, cx))
}

/// The AND/OR combiner toggle; disabled on the first row.
fn combiner_control(
    index: usize,
    label: String,
    muted: Hsla,
    cx: &mut Context<QueryView>,
) -> impl IntoElement {
    button(
        ButtonSpec {
            id: format!("query-combiner-{index}"),
            label,
            width: 58.0,
            color: muted,
            background: muted.opacity(0.15),
            enabled: index > 0,
        },
        cx,
        move |view, cx| {
            view.state_mut().toggle_combiner(index);
            cx.notify();
        },
    )
}

/// The field chooser; each click cycles the field.
fn field_control(
    index: usize,
    label: String,
    foreground: Hsla,
    muted: Hsla,
    cx: &mut Context<QueryView>,
) -> impl IntoElement {
    button(
        ButtonSpec {
            id: format!("query-field-{index}"),
            label,
            width: 120.0,
            color: foreground,
            background: muted.opacity(0.15),
            enabled: true,
        },
        cx,
        move |view, cx| {
            view.state_mut().cycle_field(index);
            cx.notify();
        },
    )
}

/// The operator chooser; each click cycles the operator.
fn op_control(
    index: usize,
    label: String,
    foreground: Hsla,
    muted: Hsla,
    cx: &mut Context<QueryView>,
) -> impl IntoElement {
    button(
        ButtonSpec {
            id: format!("query-op-{index}"),
            label,
            width: 110.0,
            color: foreground,
            background: muted.opacity(0.15),
            enabled: true,
        },
        cx,
        move |view, cx| {
            view.state_mut().cycle_op(index);
            cx.notify();
        },
    )
}

/// The value cell: an input for value ops, an em dash otherwise.
fn value_control(
    input: Option<Entity<InputState>>,
    needs_value: bool,
    muted: Hsla,
) -> impl IntoElement {
    div()
        .flex_1()
        .when_some(input.filter(|_| needs_value), |element, input| {
            element.child(Input::new(&input))
        })
        .when(!needs_value, |element| {
            element.child(div().text_size(px(11.0)).text_color(muted).child("—"))
        })
}

/// The move-up/move-down/remove controls for a row.
fn row_actions(index: usize, muted: Hsla, cx: &mut Context<QueryView>) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(2.0))
        .child(icon_action(
            format!("query-move-up-{index}"),
            t_global("query.move_up"),
            muted,
            index > 0,
            cx,
            move |view, cx| {
                view.state_mut().move_row(index, -1);
                cx.notify();
            },
        ))
        .child(icon_action(
            format!("query-move-down-{index}"),
            t_global("query.move_down"),
            muted,
            true,
            cx,
            move |view, cx| {
                view.state_mut().move_row(index, 1);
                cx.notify();
            },
        ))
        .child(icon_action(
            format!("query-remove-{index}"),
            t_global("query.remove_condition"),
            muted,
            true,
            cx,
            move |view, cx| {
                view.state_mut().remove_row(index);
                cx.notify();
            },
        ))
}

/// The static appearance of a compact builder button.
struct ButtonSpec {
    id: String,
    label: String,
    width: f32,
    color: Hsla,
    background: Hsla,
    enabled: bool,
}

/// A compact text button.
fn button(
    spec: ButtonSpec,
    cx: &mut Context<QueryView>,
    action: impl Fn(&mut QueryView, &mut Context<QueryView>) + 'static,
) -> impl IntoElement {
    let ButtonSpec {
        id,
        label,
        width,
        color,
        background,
        enabled,
    } = spec;
    let selector = id.clone();
    div()
        .id(id)
        .debug_selector(move || selector.clone())
        .w(px(width))
        .flex()
        .items_center()
        .justify_center()
        .px(px(6.0))
        .py(px(3.0))
        .rounded(px(4.0))
        .bg(background)
        .text_size(px(10.0))
        .text_color(color)
        .when(enabled, |element| {
            element
                .cursor_pointer()
                .on_click(cx.listener(move |view, _, _, cx| action(view, cx)))
        })
        .child(label)
}

/// A small labelled action button.
fn icon_action(
    id: String,
    label: String,
    color: Hsla,
    enabled: bool,
    cx: &mut Context<QueryView>,
    action: impl Fn(&mut QueryView, &mut Context<QueryView>) + 'static,
) -> impl IntoElement {
    let selector = id.clone();
    div()
        .id(id)
        .debug_selector(move || selector.clone())
        .px(px(4.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .text_size(px(9.0))
        .text_color(color)
        .when(enabled, |element| {
            element
                .cursor_pointer()
                .on_click(cx.listener(move |view, _, _, cx| action(view, cx)))
        })
        .child(label)
}

/// Re-export the model accessor used by the view.
impl QueryView {
    /// Mutable access to the builder/editor state for row actions.
    pub fn state_mut(&mut self) -> &mut PanelState {
        &mut self.state
    }
}
