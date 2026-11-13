//! Vault tab: vault path, page-list layout, the inline importer and the four
//! export rows.
//!
//! The shipped page opens the import dialog from this tab; the native surface
//! renders the same importer inline so the shell does not need a second window.

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::super::controls::{self, Palette};
use super::super::form::ExportKind;
use super::super::SettingsView;

/// Render the vault tab.
pub(super) fn render(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(
            controls::section(t_global("settings.vault"), palette)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .w(px(480.0))
                        .child(controls::labeled_input(
                            view,
                            "vault_path",
                            t_global("settings.vault_path"),
                            palette,
                        ))
                        .child(div().flex().gap(px(6.0)).child(controls::button(
                            "settings-browse-vault",
                            t_global("settings.browse"),
                            false,
                            false,
                            palette,
                            cx.listener(|view, _, _, cx| view.browse_vault(cx)),
                        ))),
                )
                .child(controls::caption(t_global("settings.pages_view"), palette))
                .child(controls::chip_row(
                    view,
                    "pages_view",
                    &view.form.settings.layout.pages_view,
                    &[
                        ("settings.pages_view_flat", "flat"),
                        ("settings.pages_view_tree", "tree"),
                    ],
                    palette,
                    cx,
                )),
        )
        .child(import_section(view, palette, cx))
        .child(export_section(view, palette, cx))
}

/// The inline vault importer.
fn import_section(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let format = view.form.import_format.label();
    let format_key = format!("importers.format_{format}");
    let mut section = controls::section(t_global("importers.import_notes"), palette)
        .child(controls::caption(
            t_global("importers.select_source_folder"),
            palette,
        ))
        .child(controls::labeled_input(
            view,
            "import_path",
            t_global("importers.source_folder"),
            palette,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(controls::button(
                    "settings-browse-import",
                    t_global("importers.browse"),
                    false,
                    false,
                    palette,
                    cx.listener(|view, _, _, cx| view.browse_import(cx)),
                ))
                .child(controls::caption(t_global("importers.format"), palette))
                .child(controls::chip_row(
                    view,
                    "import_format",
                    format,
                    &[
                        ("importers.format_auto", "auto"),
                        ("importers.format_obsidian", "obsidian"),
                        ("importers.format_logseq", "logseq"),
                        ("importers.format_generic", "generic"),
                    ],
                    palette,
                    cx,
                ))
                .child(controls::status_chip(
                    t_global(&format_key),
                    controls::ChipTone::Info,
                    palette,
                )),
        )
        .child(controls::toggle_row(
            "settings-import-dry-run",
            t_global("importers.preview"),
            None,
            view.form.import_dry_run,
            palette,
            cx.listener(|view, checked: &bool, _, cx| {
                view.set_flag("import_dry_run", *checked, cx);
            }),
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(controls::button(
                    "settings-import-preview",
                    t_global("importers.preview"),
                    view.import_busy(),
                    false,
                    palette,
                    cx.listener(|view, _, _, cx| view.preview_import(cx)),
                ))
                .child(controls::button(
                    "settings-import-run",
                    if view.import_busy() {
                        t_global("importers.importing")
                    } else {
                        t_global("importers.import")
                    },
                    view.import_busy(),
                    true,
                    palette,
                    cx.listener(|view, _, _, cx| view.run_import(cx)),
                ))
                .child(controls::button(
                    "settings-import-clear",
                    t_global("importers.close"),
                    false,
                    false,
                    palette,
                    cx.listener(|view, _, _, cx| view.clear_import(cx)),
                )),
        );

    if let Some(preview) = view.import_preview() {
        section = section.child(preview_body(preview, palette));
    }
    if let Some(report) = view.import_report() {
        section = section.child(report_body(report, palette));
    }
    section
}

/// The import preview summary.
fn preview_body(preview: &crate::provider::ImportPreview, palette: &Palette) -> impl IntoElement {
    let format_key = format!("importers.format_{}", preview.format.label());
    let mut body = div()
        .flex()
        .flex_col()
        .gap(px(3.0))
        .p(px(8.0))
        .rounded(px(6.0))
        .bg(palette.background)
        .child(
            div()
                .text_size(px(9.0))
                .text_color(palette.muted)
                .child(format!(
                    "{}: {}",
                    t_global("importers.source_folder"),
                    preview.source_path
                )),
        )
        .child(
            div()
                .text_size(px(9.0))
                .text_color(palette.muted)
                .child(format!(
                    "{}: {}",
                    t_global("importers.format"),
                    t_global(&format_key)
                )),
        );
    if preview.total_pages == 0 && preview.total_assets == 0 {
        body = body.child(
            div()
                .text_size(px(10.0))
                .text_color(palette.muted)
                .child(t_global("importers.preview_empty")),
        );
        return body;
    }
    body = body.child(
        div()
            .text_size(px(10.0))
            .text_color(palette.foreground)
            .child(format!(
                "{} ({}) · {} ({})",
                t_global("importers.pages"),
                preview.total_pages,
                t_global("importers.assets"),
                preview.total_assets
            )),
    );
    if !preview.duplicates.is_empty() {
        body = body.child(list_block(
            t_global("importers.duplicates"),
            &preview.duplicates,
            palette,
        ));
    }
    if !preview.conflicts.is_empty() {
        body = body.child(list_block(
            t_global("importers.conflicts"),
            &preview.conflicts,
            palette,
        ));
    }
    body
}

/// The import report summary.
fn report_body(report: &crate::provider::ImportReport, palette: &Palette) -> impl IntoElement {
    let mut body = div()
        .flex()
        .flex_col()
        .gap(px(3.0))
        .p(px(8.0))
        .rounded(px(6.0))
        .bg(palette.background)
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(palette.foreground)
                .child(t_global("importers.report_title")),
        )
        .child(
            div()
                .text_size(px(10.0))
                .text_color(palette.foreground)
                .child(stratum_i18n::tr_global(
                    "importers.report_summary",
                    &[
                        ("pages", report.pages_imported.to_string().as_str()),
                        ("assets", report.assets_imported.to_string().as_str()),
                        ("duplicates", report.duplicates_skipped.to_string().as_str()),
                        ("conflicts", report.conflicts_skipped.to_string().as_str()),
                    ],
                )),
        );
    for error in &report.errors {
        body = body.child(
            div()
                .text_size(px(9.0))
                .text_color(palette.danger)
                .child(error.clone()),
        );
    }
    body.child(
        div()
            .text_size(px(9.0))
            .text_color(palette.muted)
            .child(t_global("importers.undo_note")),
    )
}

/// A labelled list of skipped paths.
fn list_block(title: String, entries: &[String], palette: &Palette) -> impl IntoElement {
    let mut block = div().flex().flex_col().gap(px(1.0)).child(
        div()
            .text_size(px(10.0))
            .text_color(palette.foreground)
            .child(title),
    );
    for entry in entries {
        block = block.child(
            div()
                .text_size(px(9.0))
                .text_color(palette.muted)
                .child(entry.clone()),
        );
    }
    block
}

/// The four export rows.
fn export_section(
    view: &SettingsView,
    palette: &Palette,
    cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    let vault = view.form.settings.vault_path.clone();
    let mut section = controls::section(t_global("export.section_label"), palette)
        .child(controls::caption(vault, palette));
    for kind in ExportKind::ALL {
        let path = view.form.exports.get(&kind).cloned().unwrap_or_default();
        let pdf_disabled = kind == ExportKind::Pdf && view.pdf_page().is_none();
        let busy = view.busy_export() == Some(kind);
        let mut row = div()
            .flex()
            .items_end()
            .gap(px(6.0))
            .child(div().flex_1().child(controls::labeled_input(
                view,
                kind.input_key(),
                t_global("export.output_path"),
                palette,
            )))
            .child(controls::button(
                export_browse_id(kind),
                t_global("export.browse"),
                false,
                false,
                palette,
                cx.listener(move |view, _, _, cx| view.browse_export(kind, cx)),
            ))
            .child(controls::button(
                export_run_id(kind),
                if busy {
                    t_global("export.exporting")
                } else {
                    t_global(kind.label_key())
                },
                pdf_disabled || busy || path.trim().is_empty(),
                true,
                palette,
                cx.listener(move |view, _, _, cx| view.run_export(kind, cx)),
            ))
            .child(controls::button(
                export_reveal_id(kind),
                t_global("export.open_folder"),
                path.trim().is_empty(),
                false,
                palette,
                cx.listener(move |view, _, _, cx| view.reveal_export(kind, cx)),
            ));
        if pdf_disabled {
            row = row.child(controls::hint(t_global("export.no_current_page"), palette));
        }
        section = section.child(row);
    }
    section
}

/// Stable debug id for an export browse button.
fn export_browse_id(kind: ExportKind) -> &'static str {
    match kind {
        ExportKind::Html => "settings-export-browse-html",
        ExportKind::Json => "settings-export-browse-json",
        ExportKind::Pdf => "settings-export-browse-pdf",
        ExportKind::Zip => "settings-export-browse-zip",
    }
}

/// Stable debug id for an export reveal button.
fn export_reveal_id(kind: ExportKind) -> &'static str {
    match kind {
        ExportKind::Html => "settings-export-reveal-html",
        ExportKind::Json => "settings-export-reveal-json",
        ExportKind::Pdf => "settings-export-reveal-pdf",
        ExportKind::Zip => "settings-export-reveal-zip",
    }
}

/// Stable debug id for an export action button.
fn export_run_id(kind: ExportKind) -> &'static str {
    match kind {
        ExportKind::Html => "settings-export-run-html",
        ExportKind::Json => "settings-export-run-json",
        ExportKind::Pdf => "settings-export-run-pdf",
        ExportKind::Zip => "settings-export-run-zip",
    }
}
