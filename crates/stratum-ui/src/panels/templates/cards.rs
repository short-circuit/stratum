//! Template cards, placeholder chips and the Apply action.
//!
//! Split out of [`super::TemplatesView::render`] to keep the render function
//! small. Each card shows the template name, its description, the `{{name}}`
//! placeholders derived from the template content (via the shared
//! `stratum-app` helper) and an Apply button. Applying with an empty target is
//! a no-op, matching the shipped panel.

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_app::template::template_variables;
use stratum_i18n::t_global;

use super::TemplatesView;

/// Render the template cards or the empty state.
pub fn template_cards(view: &TemplatesView, cx: &mut Context<TemplatesView>) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let templates = view.templates().to_vec();
    if templates.is_empty() {
        return div()
            .id("templates-empty")
            .debug_selector(|| "templates-empty".to_string())
            .text_size(px(12.0))
            .text_color(muted)
            .child(t_global(
                "templates.no_templates_yet_save_a_page_as_template_to_get_started",
            ))
            .into_any_element();
    }
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .children(
            templates
                .into_iter()
                .enumerate()
                .map(|(index, template)| template_card(template, index, cx)),
        )
        .into_any_element()
}

/// Render one template card with its placeholder chips and Apply action.
fn template_card(
    template: crate::provider::TemplateRow,
    index: usize,
    cx: &mut Context<TemplatesView>,
) -> AnyElement {
    let theme = cx.theme();
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let border = theme.colors.border;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;
    let variables = template_variables(&template.content);
    let name = template.name.clone();
    let description = template.description.clone();
    div()
        .id(format!("template-card-{index}"))
        .debug_selector(move || format!("template-card-{index}"))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .p(px(12.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(border)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(foreground)
                        .child(template.name.clone()),
                )
                .child(
                    div()
                        .id(format!("template-apply-{index}"))
                        .debug_selector(move || format!("template-apply-{index}"))
                        .px(px(10.0))
                        .py(px(3.0))
                        .rounded(px(6.0))
                        .bg(accent)
                        .text_color(accent_foreground)
                        .text_size(px(11.0))
                        .cursor_pointer()
                        .child(t_global("templates.apply"))
                        .on_click(cx.listener(move |view, _, window, cx| {
                            view.apply(&name, window, cx);
                        })),
                ),
        )
        .when_some(description, |element, description| {
            element.child(
                div()
                    .text_size(px(11.0))
                    .text_color(muted)
                    .child(description),
            )
        })
        .when(!variables.is_empty(), |element| {
            element.child(div().flex().flex_wrap().gap(px(4.0)).children(
                variables.into_iter().map(|variable| {
                    div()
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .bg(muted.opacity(0.16))
                        .text_size(px(10.0))
                        .text_color(muted)
                        .child(format!("{{{{{variable}}}}}"))
                }),
            ))
        })
        .into_any_element()
}
