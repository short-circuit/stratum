//! Status chips and message banners for the native Settings panel.
//!
//! These helpers are shared by the tab bodies and the panel root, so they live
//! beside [`super::controls`] and reuse its palette snapshot.

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::controls::{ChipTone, Palette};
use super::SettingsView;

/// A status chip.
pub(super) fn status_chip(label: String, tone: ChipTone, palette: &Palette) -> impl IntoElement {
    let color = match tone {
        ChipTone::Neutral => palette.muted,
        ChipTone::Success => palette.success,
        ChipTone::Danger => palette.danger,
        ChipTone::Warning => palette.warning,
        ChipTone::Info => palette.info,
    };
    div()
        .px(px(8.0))
        .py(px(3.0))
        .rounded(px(6.0))
        .bg(color.opacity(0.15))
        .text_color(color)
        .text_size(px(9.0))
        .font_weight(FontWeight::SEMIBOLD)
        .child(label)
}

/// A success or error banner for the current status message.
pub(super) fn message_banner(
    message: String,
    is_error: bool,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let color = if is_error {
        palette.danger
    } else {
        palette.success
    };
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .p(px(10.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(color)
        .bg(color.opacity(0.1))
        .text_size(px(10.0))
        .text_color(color)
        .child(message)
        .child(div().flex_1())
        .child(
            div()
                .id("settings-dismiss-message")
                .debug_selector(|| "settings-dismiss-message".to_string())
                .cursor_pointer()
                .text_color(palette.muted)
                .child(t_global("importers.close"))
                .on_click(cx.listener(|view, _, _, cx| view.clear_message(cx))),
        )
}

/// A persistent error banner for a failed settings load.
pub(super) fn error_banner(message: String, palette: &Palette) -> impl IntoElement {
    div()
        .p(px(10.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(palette.danger)
        .bg(palette.danger.opacity(0.1))
        .text_size(px(10.0))
        .text_color(palette.danger)
        .child(message)
}
