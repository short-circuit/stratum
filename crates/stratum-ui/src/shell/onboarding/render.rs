//! Rendering for the onboarding wizard.
//!
//! Draws the modal overlay, the per-step body, the error/status rows and the
//! navigation buttons. State and provider calls live in [`super`].

use gpui_kit::component::input::Input;
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::{t_global, tr_global};

use super::{Shell, Step};

/// Render the onboarding overlay.
pub fn overlay(shell: &Shell, cx: &mut Context<Shell>) -> AnyElement {
    let theme = cx.theme();
    let scrim = theme.colors.background.opacity(0.75);
    let surface = theme.colors.secondary;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let danger = theme.colors.danger;
    let onboarding = shell.onboarding();
    let step = onboarding.step();
    let total = 4;

    let mut body = div().flex().flex_col().gap(px(10.0));
    match step {
        Step::Welcome => {
            body = body
                .child(
                    div()
                        .text_size(px(20.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(foreground)
                        .child(t_global("onboarding.welcome_title")),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(muted)
                        .child(t_global("onboarding.welcome_subtitle")),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_size(px(10.5))
                                .text_color(muted)
                                .child(t_global("settings.vault_path")),
                        )
                        .when_some(onboarding.vault_input().cloned(), |element, input| {
                            element.child(
                                div()
                                    .id("onboarding-vault-input")
                                    .debug_selector(|| "onboarding-vault-input".to_string())
                                    .child(Input::new(&input)),
                            )
                        })
                        .child(nav_button(
                            "onboarding-browse",
                            t_global("onboarding.choose_vault_folder"),
                            false,
                            cx,
                            |shell, _window, cx| shell.onboarding_browse_vault(cx),
                        )),
                );
        }
        Step::Templates => {
            body = body.child(toggle_row(
                "onboarding-templates",
                t_global("onboarding.start_with_templates"),
                t_global("onboarding.templates_hint"),
                onboarding.install_templates(),
                cx,
                |shell, cx| shell.onboarding_toggle_templates(cx),
            ));
        }
        Step::Sample => {
            body = body.child(toggle_row(
                "onboarding-sample",
                t_global("onboarding.sample_vault"),
                t_global("onboarding.sample_vault_hint"),
                onboarding.install_sample(),
                cx,
                |shell, cx| shell.onboarding_toggle_sample(cx),
            ));
        }
        Step::Finish => {
            body = body
                .child(
                    div()
                        .text_size(px(20.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(foreground)
                        .child(t_global("onboarding.finish_title")),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(muted)
                        .child(t_global("onboarding.finish_hint")),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(muted)
                        .child(t_global("onboarding.open_quick_start")),
                );
        }
    }

    if let Some(error) = onboarding.error() {
        body = body.child(
            div()
                .id("onboarding-error")
                .debug_selector(|| "onboarding-error".to_string())
                .text_size(px(11.0))
                .text_color(danger)
                .child(error.to_string()),
        );
    }
    if let Some(status) = onboarding.status() {
        body = body.child(
            div()
                .id("onboarding-status")
                .debug_selector(|| "onboarding-status".to_string())
                .text_size(px(10.5))
                .text_color(muted)
                .child(status.to_string()),
        );
    }

    let mut actions = div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .child(div().text_size(px(10.5)).text_color(muted).child(tr_global(
            "onboarding.step",
            &[
                ("current", &step.number().to_string()),
                ("total", &total.to_string()),
            ],
        )));
    let mut buttons = div().flex().items_center().gap(px(6.0));
    if step.previous().is_some() {
        buttons = buttons.child(nav_button(
            "onboarding-back",
            t_global("onboarding.back"),
            false,
            cx,
            |shell, _window, cx| shell.onboarding_back(cx),
        ));
    }
    if step == Step::Finish {
        buttons = buttons.child(nav_button(
            "onboarding-finish",
            t_global("onboarding.finish"),
            true,
            cx,
            |shell, _window, cx| shell.onboarding_finish(cx),
        ));
    } else {
        buttons = buttons.child(nav_button(
            "onboarding-next",
            t_global("onboarding.next"),
            true,
            cx,
            |shell, _window, cx| shell.onboarding_next(cx),
        ));
    }
    actions = actions.child(buttons);

    div()
        .id("onboarding-overlay")
        .debug_selector(|| "onboarding-overlay".to_string())
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(scrim)
        .child(
            div()
                .w(px(440.0))
                .p(px(20.0))
                .rounded(px(10.0))
                .bg(surface)
                .border_1()
                .border_color(border)
                .flex()
                .flex_col()
                .gap(px(14.0))
                .child(body)
                .child(actions),
        )
        .into_any_element()
}

/// Render a labelled toggle row.
fn toggle_row(
    id: &str,
    label: String,
    hint: String,
    active: bool,
    cx: &mut Context<Shell>,
    action: impl Fn(&mut Shell, &mut Context<Shell>) + 'static,
) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.colors.muted_foreground;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let accent = theme.colors.accent;
    let id = id.to_string();
    let debug_id = id.clone();
    div()
        .id(id)
        .debug_selector(move || debug_id.clone())
        .flex()
        .items_start()
        .gap(px(10.0))
        .p(px(10.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(if active { accent } else { border })
        .cursor_pointer()
        .role(Role::CheckBox)
        .aria_label(label.clone())
        .child(
            div()
                .w(px(14.0))
                .h(px(14.0))
                .rounded(px(3.0))
                .border_1()
                .border_color(if active { accent } else { border })
                .bg(if active { accent } else { border.opacity(0.25) }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(foreground)
                        .child(label),
                )
                .child(div().text_size(px(10.5)).text_color(muted).child(hint)),
        )
        .on_click(cx.listener(move |shell, _, _, cx| action(shell, cx)))
        .into_any_element()
}

/// One wizard action button.
fn nav_button(
    id: impl Into<String>,
    label: String,
    primary: bool,
    cx: &mut Context<Shell>,
    action: impl Fn(&mut Shell, &mut Window, &mut Context<Shell>) + 'static,
) -> AnyElement {
    let theme = cx.theme();
    let id = id.into();
    let debug_id = id.clone();
    let (background, color) = if primary {
        (theme.colors.accent, theme.colors.accent_foreground)
    } else {
        (
            theme.colors.muted_foreground.opacity(0.15),
            theme.colors.foreground,
        )
    };
    div()
        .id(id)
        .debug_selector(move || debug_id.clone())
        .flex()
        .items_center()
        .justify_center()
        .px(px(12.0))
        .py(px(6.0))
        .rounded(px(6.0))
        .bg(background)
        .text_color(color)
        .text_size(px(11.0))
        .cursor_pointer()
        .role(Role::Button)
        .aria_label(label.clone())
        .child(label)
        .on_click(cx.listener(move |shell, _, window, cx| action(shell, window, cx)))
        .into_any_element()
}
