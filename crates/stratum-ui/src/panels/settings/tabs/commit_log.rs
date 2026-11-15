//! Shared collapsible commit-log renderer for the native Settings surface.
//!
//! The shipped Settings page exposes the recent-commit table under Developer.
//! Keeping the markup here means the Developer tab owns the section while the
//! table itself stays a single implementation.

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::super::controls::Palette;
use super::super::SettingsView;

/// Render the collapsible recent-commit table.
pub(super) fn render(
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
