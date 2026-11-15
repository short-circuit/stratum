//! Theme tab: interface language, dark mode, palette colours and font size.
//!
//! Every edit repaints the shell immediately through
//! [`SettingsView::sync_theme_from_form`]; a half-typed colour keeps the last
//! valid palette until the value parses.

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::super::controls::{self, Palette};
use super::super::SettingsView;

/// Shipped primary-colour presets.
const PRIMARY_PRESETS: [&str; 8] = [
    "#f97316", "#ef4444", "#3b82f6", "#8b5cf6", "#10b981", "#f59e0b", "#ec4899", "#06b6d4",
];

/// Shipped secondary-colour presets.
const SECONDARY_PRESETS: [&str; 7] = [
    "#6b7280", "#78716c", "#a1a1aa", "#71717a", "#52525b", "#3f3f46", "#27272a",
];

/// Shipped settings locale set.
const LOCALES: [&str; 8] = ["en", "de", "it", "fr", "es", "zh", "ja", "el"];

/// Render the theme tab.
pub(super) fn render(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let mut locales = div().flex().flex_wrap().gap(px(5.0));
    for code in LOCALES {
        let key = format!("settings.langs.{code}");
        let selected = view.form.settings.locale == code;
        locales = locales.child(controls::chip(
            format!("settings-locale-{code}"),
            t_global(&key),
            selected,
            palette,
            cx.listener(move |view, _, _, cx| view.change_locale(code, cx)),
        ));
    }

    controls::section(t_global("settings.theme"), palette)
        .child(controls::caption(
            t_global("settings.language_label"),
            palette,
        ))
        .child(locales)
        .child(controls::toggle_row(
            "settings-dark-mode",
            t_global("settings.dark_mode"),
            None,
            view.form.settings.theme.dark_mode,
            palette,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_flag("dark_mode", *checked, cx);
            }),
        ))
        .child(color_editor(
            view,
            "theme_primary",
            t_global("settings.primary_buttons_accents"),
            &PRIMARY_PRESETS,
            palette,
            cx,
        ))
        .child(color_editor(
            view,
            "theme_secondary",
            t_global("settings.secondary_backgrounds_borders"),
            &SECONDARY_PRESETS,
            palette,
            cx,
        ))
        .child(
            div()
                .flex()
                .items_end()
                .gap(px(10.0))
                .child(div().w(px(160.0)).child(controls::labeled_input(
                    view,
                    super::super::form::KEY_THEME_FONT_SIZE,
                    t_global("settings.font_size"),
                    palette,
                )))
                .child(controls::hint(
                    format!("{} 12–28", t_global("settings.font_size")),
                    palette,
                )),
        )
}

/// A colour editor: hex input plus preset swatches.
fn color_editor(
    view: &SettingsView,
    field: &'static str,
    label: String,
    presets: &[&str],
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let mut swatches = div().flex().flex_wrap().gap(px(4.0));
    for preset in presets {
        let preset = preset.to_string();
        let selected = match field {
            "theme_primary" => view.form.settings.theme.primary_color == preset,
            "theme_secondary" => view.form.settings.theme.secondary_color == preset,
            _ => false,
        };
        let swatch = crate::theme::ThemeConfig::default().to_hsla(&preset);
        swatches = swatches.child(
            div()
                .id(format!("settings-swatch-{field}-{preset}"))
                .debug_selector({
                    let id = format!("settings-swatch-{field}-{preset}");
                    move || id.clone()
                })
                .w(px(18.0))
                .h(px(18.0))
                .rounded(px(4.0))
                .bg(swatch)
                .border_2()
                .border_color(if selected {
                    palette.foreground
                } else {
                    palette.border
                })
                .cursor_pointer()
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.set_theme_color(field, &preset, cx);
                })),
        );
    }
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(controls::caption(label, palette))
        .child(
            div()
                .flex()
                .items_end()
                .gap(px(8.0))
                .child(div().w(px(160.0)).child(controls::labeled_input(
                    view,
                    field,
                    t_global("settings.hex_color"),
                    palette,
                )))
                .child(swatches),
        )
}
