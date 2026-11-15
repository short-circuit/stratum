//! Cloud/folder backup section inside the Sync tab.
//!
//! Mirrors the shipped `BackupSettingsPanel`: enable, target mode, folder and
//! WebDAV credentials, connection probe, interval, exclude list, manual run and
//! the last-run ledger.

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::super::controls::{self, ChipTone, Palette};
use super::super::SettingsView;

/// The shipped backup target modes.
const MODES: [(&str, &str); 3] = [
    ("settings.backup_mode_folder", "folder"),
    ("settings.backup_mode_webdav", "webdav"),
    ("settings.backup_mode_both", "both"),
];

/// Render the backup section.
pub(super) fn render(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let backup = &view.form.settings.sync.backup;
    let mode = backup.mode.clone();
    let uses_folder = matches!(mode.as_str(), "folder" | "both");
    let uses_webdav = matches!(mode.as_str(), "webdav" | "both");
    let mut section = controls::section(t_global("settings.backup"), palette)
        .child(controls::toggle_row(
            "settings-backup-enabled",
            t_global("settings.backup_enable"),
            None,
            backup.enabled,
            palette,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_flag("backup_enabled", *checked, cx);
            }),
        ))
        .child(controls::caption(t_global("settings.backup_mode"), palette))
        .child(controls::chip_row(
            view,
            "backup_mode",
            &mode,
            &MODES,
            palette,
            cx,
        ));
    if uses_folder {
        section = section.child(controls::labeled_input(
            view,
            "backup_folder_path",
            t_global("settings.backup_folder_path"),
            palette,
        ));
    }
    if uses_webdav {
        section = section
            .child(controls::labeled_input(
                view,
                "backup_webdav_url",
                t_global("settings.backup_webdav_url"),
                palette,
            ))
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap(px(6.0))
                    .child(div().flex_1().child(controls::labeled_input(
                        view,
                        "backup_webdav_username",
                        t_global("settings.backup_webdav_username"),
                        palette,
                    )))
                    .child(div().flex_1().child(controls::labeled_input(
                        view,
                        "backup_webdav_password",
                        t_global("settings.backup_webdav_password"),
                        palette,
                    ))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(controls::button(
                        "settings-webdav-test",
                        if view.webdav_testing() {
                            t_global("settings.testing")
                        } else {
                            t_global("settings.backup_test_connection")
                        },
                        view.webdav_testing(),
                        false,
                        palette,
                        cx.listener(|view, _, _, cx| view.test_webdav(cx)),
                    ))
                    .when_some(view.webdav_result().map(str::to_string), |row, result| {
                        let tone = if result == "ok" {
                            ChipTone::Success
                        } else {
                            ChipTone::Danger
                        };
                        let label = if result == "ok" {
                            t_global("settings.backup_test_ok")
                        } else {
                            result
                        };
                        row.child(controls::status_chip(label, tone, palette))
                    }),
            );
    }
    section = section
        .child(div().w(px(200.0)).child(controls::labeled_input(
            view,
            super::super::form::KEY_BACKUP_INTERVAL,
            t_global("settings.backup_interval_minutes"),
            palette,
        )))
        .child(controls::labeled_textarea(
            view,
            super::super::form::KEY_BACKUP_EXCLUDE,
            t_global("settings.backup_exclude"),
            palette,
        ))
        .child(controls::hint(
            t_global("settings.backup_exclude_help"),
            palette,
        ))
        .child(div().flex().items_center().child(controls::button(
            "settings-backup-run",
            if view.backup_running() {
                t_global("settings.backup_running")
            } else {
                t_global("settings.backup_run_now")
            },
            view.backup_running(),
            true,
            palette,
            cx.listener(|view, _, _, cx| view.run_backup(cx)),
        )));
    if let Some(status) = view.backup_status() {
        section = section
            .child(
                div()
                    .text_size(px(9.0))
                    .text_color(palette.muted)
                    .child(format!(
                        "{} {}",
                        t_global("settings.backup_last_run"),
                        status.last_run.clone().unwrap_or_else(|| "—".to_string())
                    )),
            )
            .child(
                div()
                    .text_size(px(9.0))
                    .text_color(palette.muted)
                    .child(format!(
                        "{} {}",
                        t_global("settings.backup_files_copied"),
                        status.files_copied.unwrap_or(0)
                    )),
            )
            .child(
                div()
                    .text_size(px(9.0))
                    .text_color(palette.muted)
                    .child(format!(
                        "{} {}",
                        t_global("settings.backup_files_deleted"),
                        status.files_deleted.unwrap_or(0)
                    )),
            );
        if status.last_success == Some(true) && status.last_error.is_none() {
            section = section.child(controls::status_chip(
                t_global("settings.backup_no_errors"),
                ChipTone::Success,
                palette,
            ));
        }
        if let Some(error) = status.last_error.clone() {
            section = section.child(
                div()
                    .text_size(px(9.0))
                    .text_color(palette.danger)
                    .child(format!("{} {error}", t_global("settings.backup_failed"))),
            );
        }
    }
    section.child(controls::hint(
        t_global("settings.backup_manual_note"),
        palette,
    ))
}
