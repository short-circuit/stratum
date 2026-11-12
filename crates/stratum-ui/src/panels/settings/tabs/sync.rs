//! Sync tab: sync mode, remote/branch, SSH key, auto-commit/sync schedules,
//! manual sync with inline passphrase, the commit log and the backup panel.

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::super::controls::{self, ChipTone, Palette};
use super::super::SettingsView;

/// The shipped sync modes.
const MODES: [(&str, &str); 4] = [
    ("settings.sync_mode_manual", "manual"),
    ("settings.auto_commit", "auto_commit"),
    ("settings.auto_sync", "auto_sync"),
    ("settings.sync_mode_background", "background"),
];

/// Commit-template placeholders offered by the shipped page.
const PLACEHOLDERS: [&str; 5] = [
    "{datetime}",
    "{editedfiles}",
    "{newfiles}",
    "{deletedfiles}",
    "{count}",
];

/// Render the sync tab.
pub(super) fn render(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let mode = view.form.settings.sync.mode.clone();
    let scheduled = matches!(mode.as_str(), "auto_commit" | "auto_sync" | "background");
    let body = controls::section(t_global("settings.sync"), palette)
        .child(mode_section(view, palette, cx))
        .child(remote_section(view, palette))
        .child(ssh_section(view, palette))
        .when(scheduled, |element| {
            element.child(commit_section(view, palette, cx))
        })
        .when(
            matches!(mode.as_str(), "auto_sync" | "background"),
            |element| element.child(sync_interval_section(view, palette)),
        )
        .child(controls_section(view, palette, cx))
        .child(commit_log_section(view, palette, cx))
        .child(super::backup::render(view, palette, cx));
    body
}

/// Sync-mode chips and their help line.
fn mode_section(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let mode = view.form.settings.sync.mode.clone();
    let help_key = match mode.as_str() {
        "auto_commit" => "settings.auto_commit_mode_help",
        "auto_sync" => "settings.automatic_commits_periodic_push_pull_to_remote",
        "background" => "settings.full_background_sync_commits_push_pull_happen_automatica",
        _ => "settings.sync_only_when_you_click_the_sync_button_no_automatic_commit",
    };
    controls::caption(t_global("settings.sync_mode"), palette)
        .child(controls::chip_row(
            view,
            "sync_mode",
            &mode,
            &MODES,
            palette,
            cx,
        ))
        .child(controls::hint(t_global(help_key), palette))
}

/// Remote URL and branch fields.
fn remote_section(view: &SettingsView, palette: &Palette) -> impl IntoElement {
    controls::caption(t_global("settings.remote_branch"), palette).child(
        div()
            .flex()
            .items_end()
            .gap(px(6.0))
            .child(
                div()
                    .flex_1()
                    .child(controls::plain_input(view, "sync_remote_url", palette)),
            )
            .child(
                div()
                    .w(px(120.0))
                    .child(controls::plain_input(view, "sync_branch", palette)),
            ),
    )
}

/// SSH key path with the shipped Set/Agent chip.
fn ssh_section(view: &SettingsView, palette: &Palette) -> impl IntoElement {
    let configured = view
        .form
        .settings
        .sync
        .ssh_key_path
        .as_deref()
        .map(|path| !path.trim().is_empty())
        .unwrap_or(false);
    controls::caption(t_global("settings.ssh_key_path"), palette)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(div().flex_1().child(controls::plain_input(
                    view,
                    "sync_ssh_key_path",
                    palette,
                )))
                .child(controls::status_chip(
                    if configured {
                        t_global("settings.ssh_key_set")
                    } else {
                        t_global("settings.ssh_key_agent")
                    },
                    if configured {
                        ChipTone::Success
                    } else {
                        ChipTone::Neutral
                    },
                    palette,
                )),
        )
        .child(controls::hint(
            t_global("settings.leave_empty_to_use_ssh_agent"),
            palette,
        ))
}

/// Auto-commit interval, template, placeholders and preview.
fn commit_section(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let template = view.form.settings.sync.commit_template.clone();
    let preview = render_template(&template);
    let mut placeholders = div().flex().flex_wrap().gap(px(4.0));
    for placeholder in PLACEHOLDERS {
        let value = placeholder.to_string();
        placeholders = placeholders.child(controls::chip(
            format!("settings-placeholder-{placeholder}"),
            placeholder.to_string(),
            false,
            palette,
            cx.listener(move |view, _, _, cx| {
                view.insert_template_placeholder(&value, cx);
            }),
        ));
    }
    controls::caption(t_global("settings.auto_commit_settings"), palette)
        .child(div().w(px(200.0)).child(controls::labeled_input(
            view,
            super::super::form::KEY_SYNC_COMMIT_INTERVAL,
            t_global("settings.commit_interval_seconds"),
            palette,
        )))
        .child(controls::labeled_input(
            view,
            "sync_commit_template",
            t_global("settings.commit_message_template"),
            palette,
        ))
        .child(placeholders)
        .child(
            div()
                .p(px(6.0))
                .rounded(px(4.0))
                .bg(palette.border.opacity(0.35))
                .text_size(px(9.0))
                .text_color(palette.muted)
                .child(format!("{} {preview}", t_global("settings.preview"))),
        )
}

/// Auto-sync interval field.
fn sync_interval_section(view: &SettingsView, palette: &Palette) -> impl IntoElement {
    controls::caption(t_global("settings.auto_sync_settings"), palette).child(
        div().w(px(200.0)).child(controls::labeled_input(
            view,
            super::super::form::KEY_SYNC_SYNC_INTERVAL,
            t_global("settings.pull_push_interval_seconds"),
            palette,
        )),
    )
}

/// Sync-now, status chip, scheduler and the last-sync line.
fn controls_section(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let mode = view.form.settings.sync.mode.clone();
    let scheduled = matches!(mode.as_str(), "auto_commit" | "auto_sync" | "background");
    let mut row = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(6.0))
        .child(controls::button(
            "settings-sync-now",
            if view.busy() == Some("sync") {
                t_global("settings.syncing")
            } else {
                t_global("settings.sync_now")
            },
            view.busy() == Some("sync"),
            true,
            palette,
            cx.listener(|view, _, _, cx| view.sync_now(cx)),
        ));
    if let Some(status) = view.sync_status() {
        let (label, tone) = sync_status_label(status);
        row = row.child(controls::status_chip(label, tone, palette));
        if scheduled {
            row = row.child(controls::button(
                "settings-start-scheduler",
                t_global("settings.start_scheduler"),
                false,
                false,
                palette,
                cx.listener(|view, _, _, cx| view.start_scheduler(cx)),
            ));
        }
    } else if scheduled {
        row = row.child(controls::button(
            "settings-start-scheduler",
            t_global("settings.start_scheduler"),
            false,
            false,
            palette,
            cx.listener(|view, _, _, cx| view.start_scheduler(cx)),
        ));
    }

    let mut section = row;
    if let Some(status) = view.sync_status() {
        let mut meta = div().flex().gap(px(10.0)).items_center();
        if let Some(branch) = status.branch.clone() {
            meta = meta.child(
                div()
                    .text_size(px(9.0))
                    .text_color(palette.muted)
                    .child(branch),
            );
        }
        if let Some(last_sync) = status.last_sync_time.clone() {
            meta = meta.child(
                div()
                    .text_size(px(9.0))
                    .text_color(palette.muted)
                    .child(format!("{} {last_sync}", t_global("settings.last_sync"))),
            );
        }
        section = section.child(meta);
    }
    if view.passphrase_prompt() {
        section = section.child(
            div()
                .flex()
                .items_end()
                .gap(px(6.0))
                .child(div().w(px(240.0)).child(controls::labeled_input(
                    view,
                    "passphrase",
                    t_global("common.ssh_key_passphrase"),
                    palette,
                )))
                .child(controls::button(
                    "settings-passphrase-submit",
                    t_global("common.submit"),
                    false,
                    true,
                    palette,
                    cx.listener(|view, _, _, cx| view.submit_passphrase(cx)),
                ))
                .child(controls::button(
                    "settings-passphrase-cancel",
                    t_global("common.cancel"),
                    false,
                    false,
                    palette,
                    cx.listener(|view, _, _, cx| view.dismiss_passphrase(cx)),
                )),
        );
    }
    section
}

/// The localized sync status chip.
fn sync_status_label(status: &crate::provider::SyncStatusDto) -> (String, ChipTone) {
    match status.status.as_str() {
        "ok" => {
            let suffix = if status.ahead > 0 || status.behind > 0 {
                format!(" +{}/-{}", status.ahead, status.behind)
            } else {
                String::new()
            };
            (format!("OK{suffix}"), ChipTone::Success)
        }
        "conflicts" => (
            format!(
                "{}{})",
                t_global("settings.conflicts"),
                status.conflicts.len()
            ),
            ChipTone::Danger,
        ),
        "no_repo" => (t_global("settings.no_repo"), ChipTone::Warning),
        other => (other.to_string(), ChipTone::Neutral),
    }
}

/// Collapsible recent-commit table.
fn commit_log_section(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let open = view.commits_open();
    let mut section = div().flex().flex_col().gap(px(4.0)).child(
        div()
            .id("settings-toggle-commits")
            .debug_selector(|| "settings-toggle-commits".to_string())
            .flex()
            .items_center()
            .gap(px(4.0))
            .cursor_pointer()
            .text_size(px(11.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(palette.muted)
            .child(format!(
                "{} {}",
                if open { "▼" } else { "▶" },
                t_global("settings.recent_commits")
            ))
            .on_click(cx.listener(|view, _, _, cx| view.toggle_commits(cx))),
    );
    if !open {
        return section;
    }
    if view.commits().is_empty() {
        return section.child(
            div()
                .text_size(px(10.0))
                .text_color(palette.muted)
                .child(t_global("settings.no_commits_yet")),
        );
    }
    let mut table = div().flex().flex_col().gap(px(2.0)).child(
        div()
            .flex()
            .gap(px(6.0))
            .border_b_1()
            .border_color(palette.border)
            .pb(px(2.0))
            .child(column_header("Hash", 70.0, palette))
            .child(column_header("Author", 110.0, palette))
            .child(column_header("Message", 0.0, palette))
            .child(column_header("Date", 150.0, palette)),
    );
    for entry in view.commits() {
        let hash = entry.hash.chars().take(7).collect::<String>();
        table = table.child(
            div()
                .flex()
                .gap(px(6.0))
                .text_size(px(9.0))
                .child(div().w(px(70.0)).text_color(palette.accent).child(hash))
                .child(
                    div()
                        .w(px(110.0))
                        .text_color(palette.foreground)
                        .child(entry.author.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .text_color(palette.foreground)
                        .child(entry.message.clone()),
                )
                .child(
                    div()
                        .w(px(150.0))
                        .text_color(palette.muted)
                        .child(entry.timestamp.clone()),
                ),
        );
    }
    section = section.child(table);
    section
}

/// One commit-table column header.
fn column_header(label: &str, width: f32, palette: &Palette) -> impl IntoElement {
    let mut header = div()
        .text_size(px(9.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(palette.muted);
    header = if width > 0.0 {
        header.w(px(width))
    } else {
        header.flex_1()
    };
    header.child(label.to_string())
}

/// Render the shipped commit-template preview sample.
fn render_template(template: &str) -> String {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    template
        .replace("{datetime}", &now)
        .replace("{editedfiles}", "3")
        .replace("{newfiles}", "1")
        .replace("{deletedfiles}", "0")
        .replace("{count}", "4")
}
