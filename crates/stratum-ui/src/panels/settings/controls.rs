//! Reusable styled controls for the native Settings panel.
//!
//! The shipped page leans on MUI for layout, alerts, chips and toggles. The
//! native surface reproduces the same structure with plain gpui elements so the
//! tab bodies stay declarative and every control keeps one implementation.
//! Buttons and chips are custom divs (matching the rest of `stratum-ui`) rather
//! than the component-library button, which keeps the palette and hit areas
//! consistent with the graph, query and plugins panels.
//!
//! [`Palette`] snapshots the theme colours once per render so helpers never
//! borrow the context while a listener is being created.

use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, Textarea};
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::{SettingsTab, SettingsView};

/// Theme colours used by the settings controls.
#[derive(Clone, Copy, Debug)]
pub(super) struct Palette {
    /// Accent/primary colour.
    pub accent: Hsla,
    /// Foreground colour on the accent.
    pub accent_foreground: Hsla,
    /// Panel background.
    pub background: Hsla,
    /// Border colour.
    pub border: Hsla,
    /// Primary text colour.
    pub foreground: Hsla,
    /// Secondary text colour.
    pub muted: Hsla,
    /// Error colour.
    pub danger: Hsla,
    /// Success colour.
    pub success: Hsla,
    /// Warning colour.
    pub warning: Hsla,
    /// Informational colour.
    pub info: Hsla,
}

/// Snapshot the active theme palette.
pub(super) fn palette(cx: &mut Context<SettingsView>) -> Palette {
    let theme = cx.theme();
    Palette {
        accent: theme.colors.accent,
        accent_foreground: theme.colors.accent_foreground,
        background: theme.colors.background,
        border: theme.colors.border,
        foreground: theme.colors.foreground,
        muted: theme.colors.muted_foreground,
        danger: theme.colors.danger,
        success: theme.colors.success,
        warning: theme.colors.warning,
        info: theme.colors.info,
    }
}

/// Semantic colour for a status chip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ChipTone {
    /// Neutral/disabled.
    Neutral,
    /// Healthy or successful state.
    Success,
    /// Error or destructive state.
    Danger,
    /// Warning state.
    Warning,
    /// Informational state.
    Info,
}

/// The panel title bar with the Save and Reload actions.
pub(super) fn header(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let saving = view.busy() == Some("save");
    div()
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(24.0))
        .py(px(12.0))
        .border_b_1()
        .border_color(palette.border)
        .child(
            div()
                .text_size(px(18.0))
                .font_weight(FontWeight::BOLD)
                .text_color(palette.foreground)
                .child(t_global("mobile_nav.settings")),
        )
        .child(div().flex_1())
        .child(button(
            "settings-reload",
            t_global("common.refresh"),
            false,
            false,
            palette,
            cx.listener(|view, _, _, cx| view.reload(cx)),
        ))
        .child(button(
            "settings-save",
            if saving {
                t_global("settings.saving")
            } else {
                t_global("settings.save")
            },
            saving,
            true,
            palette,
            cx.listener(|view, _, _, cx| view.save(cx)),
        ))
}

/// The shipped eight-tab selector.
pub(super) fn tab_bar(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let mut row = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(6.0))
        .pb(px(6.0));
    for tab in SettingsTab::ALL {
        row = row.child(tab_chip(view.tab(), tab, palette, cx));
    }
    row
}

/// One tab chip.
fn tab_chip(
    active: SettingsTab,
    tab: SettingsTab,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let (bg, fg) = if active == tab {
        (palette.accent, palette.accent_foreground)
    } else {
        (palette.background, palette.muted)
    };
    div()
        .id(format!("settings-tab-{}", tab.id()))
        .debug_selector(move || format!("settings-tab-{}", tab.id()))
        .px(px(12.0))
        .py(px(5.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(palette.border)
        .bg(bg)
        .text_color(fg)
        .text_size(px(11.0))
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .child(t_global(tab.label_key()))
        .on_click(cx.listener(move |view, _, window, cx| {
            view.set_tab(tab, window, cx);
        }))
}

/// A settings section container with a title.
pub(super) fn section(title: String, palette: &Palette) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .p(px(16.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(palette.border)
        .bg(palette.background)
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(palette.foreground)
                .child(title),
        )
}

/// A muted caption or field label.
pub(super) fn caption(text: String, palette: &Palette) -> Div {
    div()
        .text_size(px(10.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(palette.muted)
        .child(text)
}

/// A muted help line under a field.
pub(super) fn hint(text: String, palette: &Palette) -> Div {
    div()
        .text_size(px(9.0))
        .text_color(palette.muted)
        .child(text)
}

/// The localized validation message for `key`, if the last save failed on it.
pub(super) fn field_error(
    view: &SettingsView,
    key: &'static str,
    palette: &Palette,
) -> Option<AnyElement> {
    let error_key = view.form.errors.get(key)?;
    Some(
        div()
            .text_size(px(9.0))
            .text_color(palette.danger)
            .child(t_global(error_key))
            .into_any_element(),
    )
}

/// A labelled single-line input bound to the settings form.
pub(super) fn labeled_input(
    view: &SettingsView,
    key: &'static str,
    label: String,
    palette: &Palette,
) -> impl IntoElement {
    let error = field_error(view, key, palette);
    div()
        .flex()
        .flex_col()
        .gap(px(3.0))
        .child(
            div()
                .text_size(px(10.0))
                .text_color(palette.muted)
                .child(label),
        )
        .child(
            div()
                .id(format!("settings-field-{key}"))
                .debug_selector(move || format!("settings-field-{key}"))
                .when_some(view.input(key), |element, input| {
                    element.child(Input::new(input).appearance(false).w_full())
                }),
        )
        .when_some(error, |element, error| element.child(error))
}

/// An unlabelled single-line input bound to the settings form.
///
/// Used where the shipped page renders a section caption followed by
/// placeholder-only fields, so the native surface does not repeat the caption.
pub(super) fn plain_input(
    view: &SettingsView,
    key: &'static str,
    palette: &Palette,
) -> impl IntoElement {
    let error = field_error(view, key, palette);
    div()
        .flex()
        .flex_col()
        .gap(px(3.0))
        .child(
            div()
                .id(format!("settings-field-{key}"))
                .debug_selector(move || format!("settings-field-{key}"))
                .when_some(view.input(key), |element, input| {
                    element.child(Input::new(input).appearance(false).w_full())
                }),
        )
        .when_some(error, |element, error| element.child(error))
}

/// A labelled multiline textarea bound to the settings form.
pub(super) fn labeled_textarea(
    view: &SettingsView,
    key: &'static str,
    label: String,
    palette: &Palette,
) -> impl IntoElement {
    let error = field_error(view, key, palette);
    div()
        .flex()
        .flex_col()
        .gap(px(3.0))
        .child(
            div()
                .text_size(px(10.0))
                .text_color(palette.muted)
                .child(label),
        )
        .child(
            div()
                .id(format!("settings-textarea-{key}"))
                .debug_selector(move || format!("settings-textarea-{key}"))
                .when_some(view.textarea(key), |element, textarea| {
                    element.child(Textarea::new(textarea).h(px(64.0)).w_full())
                }),
        )
        .when_some(error, |element, error| element.child(error))
}

/// A labelled checkbox row.
pub(super) fn toggle_row(
    id: &'static str,
    label: String,
    description: Option<String>,
    checked: bool,
    palette: &Palette,
    on_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            Checkbox::new(id)
                .checked(checked)
                .label(label)
                .on_click(on_change),
        )
        .when_some(description, |element, description| {
            element.child(
                div()
                    .text_size(px(9.0))
                    .text_color(palette.muted)
                    .child(description),
            )
        })
}

/// A selected/selectable chip used for enumerated settings.
pub(super) fn chip(
    id: String,
    label: String,
    selected: bool,
    palette: &Palette,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let (bg, fg) = if selected {
        (palette.accent, palette.accent_foreground)
    } else {
        (palette.background, palette.muted)
    };
    div()
        .id(id.clone())
        .debug_selector(move || id)
        .px(px(10.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(palette.border)
        .bg(bg)
        .text_color(fg)
        .text_size(px(10.0))
        .cursor_pointer()
        .child(label)
        .on_click(on_click)
}

/// A horizontal row of enumerated-setting chips.
pub(super) fn chip_row(
    _view: &SettingsView,
    field: &'static str,
    selected: &str,
    options: &[(&'static str, &'static str)],
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let mut row = div().flex().flex_wrap().items_center().gap(px(5.0));
    for (label_key, value) in options {
        let value = value.to_string();
        let selected = selected == value;
        row = row.child(chip(
            format!("settings-chip-{field}-{value}"),
            t_global(label_key),
            selected,
            palette,
            cx.listener(move |view, _, _, cx| view.set_choice(field, &value, cx)),
        ));
    }
    row
}

/// A styled action button.
///
/// `id` accepts an owned string so conflict rows can give each Accept button a
/// unique debug selector while the rest of the panel keeps static ids.
pub(super) fn button(
    id: impl Into<String>,
    label: String,
    disabled: bool,
    primary: bool,
    palette: &Palette,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let (bg, fg) = if disabled {
        (
            palette.accent.opacity(0.35),
            palette.accent_foreground.opacity(0.6),
        )
    } else if primary {
        (palette.accent, palette.accent_foreground)
    } else {
        (palette.accent.opacity(0.12), palette.accent_foreground)
    };
    button_with_colors(id.into(), label, disabled, bg, fg, on_click)
}

/// A styled action button using a semantic tone (danger/warning/etc.).
pub(super) fn toned_button(
    id: impl Into<String>,
    label: String,
    disabled: bool,
    color: Hsla,
    palette: &Palette,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let bg = if disabled { color.opacity(0.35) } else { color };
    button_with_colors(
        id.into(),
        label,
        disabled,
        bg,
        palette.accent_foreground,
        on_click,
    )
}

/// Shared button body.
fn button_with_colors(
    id: String,
    label: String,
    disabled: bool,
    bg: Hsla,
    fg: Hsla,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let debug_id = id.clone();
    div()
        .id(id)
        .debug_selector(move || debug_id)
        .px(px(12.0))
        .py(px(5.0))
        .rounded(px(6.0))
        .bg(bg)
        .text_color(fg)
        .text_size(px(11.0))
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .child(label)
        .when(!disabled, |element| element.on_click(on_click))
}

pub(super) use super::banners::{error_banner, message_banner, status_chip};
