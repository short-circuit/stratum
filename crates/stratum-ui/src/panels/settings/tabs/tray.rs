//! Tray tab: tray icon, close-to-tray, quick-capture shortcut, target and inbox
//! path.

use gpui_kit::prelude::*;
use stratum_i18n::t_global;

use super::super::controls::{self, Palette};
use super::super::SettingsView;

/// Render the tray tab.
pub(super) fn render(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let target = view.form.settings.tray.capture_target.clone();
    let mut section = controls::section(t_global("settings.tray"), palette)
        .child(controls::toggle_row(
            "settings-tray-enabled",
            t_global("settings.tray_enabled"),
            None,
            view.form.settings.tray.enabled,
            palette,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_flag("tray_enabled", *checked, cx);
            }),
        ))
        .child(controls::toggle_row(
            "settings-tray-close",
            t_global("settings.tray_close_to_tray"),
            None,
            view.form.settings.tray.close_to_tray,
            palette,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_flag("tray_close_to_tray", *checked, cx);
            }),
        ))
        .child(controls::labeled_input(
            view,
            "tray_shortcut",
            t_global("settings.tray_shortcut"),
            palette,
        ))
        .child(controls::caption(
            t_global("settings.tray_capture_target"),
            palette,
        ))
        .child(controls::chip_row(
            view,
            "tray_capture_target",
            &target,
            &[
                ("settings.tray_capture_target_journal", "journal"),
                ("settings.tray_capture_target_inbox", "inbox"),
            ],
            palette,
            cx,
        ));
    if target == "inbox" {
        section = section.child(controls::labeled_input(
            view,
            "tray_inbox_path",
            t_global("settings.tray_inbox_path"),
            palette,
        ));
    }
    section
}
