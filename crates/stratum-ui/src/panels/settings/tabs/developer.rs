//! Developer tab: index maintenance, onboarding reset and starter templates.
//!
//! The shipped page shows a determinate progress bar driven by a Tauri event.
//! The native surface runs these operations synchronously through the provider
//! seam, so the button label carries the busy state and the status message
//! reports the outcome (the progress bar is indeterminate by construction).

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::super::controls::{self, ChipTone, Palette};
use super::super::SettingsView;

/// Render the developer tab.
pub(super) fn render(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let busy = view.busy();
    let fetching = matches!(busy, Some("reindex" | "repair" | "normalize"));
    controls::section(t_global("settings.developer"), palette)
        .child(controls::section(
            t_global("settings.developer_tools"),
            palette,
        ))
        .child(controls::hint(
            t_global("settings.re_sync_all_pages_from_disk_into_the_database"),
            palette,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(controls::toned_button(
                    "settings-reindex",
                    if busy == Some("reindex") {
                        t_global("settings.reindexing")
                    } else {
                        t_global("settings.reindex_all")
                    },
                    fetching,
                    palette.danger,
                    palette,
                    cx.listener(|view, _, _, cx| view.reindex(cx)),
                ))
                .child(controls::button(
                    "settings-repair",
                    if busy == Some("repair") {
                        t_global("settings.repairing")
                    } else {
                        t_global("settings.repair_db_from_disk")
                    },
                    fetching,
                    false,
                    palette,
                    cx.listener(|view, _, _, cx| view.repair(cx)),
                )),
        )
        .when(fetching, |element| {
            element.child(controls::status_chip(
                t_global("settings.loading_settings"),
                ChipTone::Info,
                palette,
            ))
        })
        .child(controls::hint(
            t_global("settings.parse_every_md_file_through_the_block_parser"),
            palette,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(controls::toned_button(
                    "settings-normalize",
                    if busy == Some("normalize") {
                        t_global("settings.normalizing")
                    } else {
                        t_global("settings.normalize_all_files")
                    },
                    fetching,
                    palette.warning,
                    palette,
                    cx.listener(|view, _, _, cx| view.normalize_all(cx)),
                )),
        )
        .child(controls::hint(
            t_global("settings.reset_onboarding_help"),
            palette,
        ))
        .child(div().flex().items_center().child(controls::button(
            "settings-reset-onboarding",
            t_global("settings.reset_onboarding"),
            false,
            false,
            palette,
            cx.listener(|view, _, _, cx| view.reset_onboarding(cx)),
        )))
        .child(controls::hint(
            t_global("settings.install_starter_templates_help"),
            palette,
        ))
        .child(div().flex().items_center().child(controls::button(
            "settings-install-templates",
            if busy == Some("templates") {
                t_global("onboarding.installing")
            } else {
                t_global("settings.install_starter_templates")
            },
            busy == Some("templates"),
            false,
            palette,
            cx.listener(|view, _, _, cx| view.install_templates(cx)),
        )))
        .child(super::commit_log::render(view, palette, cx))
}
