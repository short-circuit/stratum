//! Notifications tab: block-reminder notifications and sound.

use gpui_kit::prelude::*;
use stratum_i18n::t_global;

use super::super::controls::{self, Palette};
use super::super::SettingsView;

/// Render the notifications tab.
pub(super) fn render(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    controls::section(t_global("settings.notifications.title"), palette)
        .child(controls::toggle_row(
            "settings-reminders-enabled",
            t_global("settings.notifications.enable"),
            None,
            view.form.settings.reminders.enabled,
            palette,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_flag("reminders_enabled", *checked, cx);
            }),
        ))
        .child(controls::toggle_row(
            "settings-reminders-sound",
            t_global("settings.notifications.sound"),
            Some(t_global("settings.notifications.sound_desc")),
            view.form.settings.reminders.sound,
            palette,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_flag("reminders_sound", *checked, cx);
            }),
        ))
}
