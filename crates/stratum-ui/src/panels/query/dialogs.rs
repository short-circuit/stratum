//! Save/rename and delete dialogs for the query panel.
//!
//! [`SavedDialog`] is the window-free dialog model the view stores; the
//! renderers reproduce the shipped `SavedQueryDialogs` and `ConfirmDialog`
//! surfaces. Validation and the provider calls stay on [`QueryView`].

use gpui_kit::component::input::Input;
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::QueryView;

/// Whether the open dialog saves a new query or renames an existing one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SavedDialogKind {
    /// Save the current buffer under a new name.
    Save,
    /// Rename an existing saved query.
    Rename,
}

/// The save/rename dialog state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedDialog {
    /// Which action the dialog performs.
    pub kind: SavedDialogKind,
    /// The name field value.
    pub name: String,
    /// The saved query being renamed, when `kind` is [`SavedDialogKind::Rename`].
    pub target: Option<String>,
    /// The validation/provider error, if any.
    pub error: Option<String>,
}

impl SavedDialog {
    /// Build a save dialog with an empty name.
    pub fn save() -> Self {
        Self {
            kind: SavedDialogKind::Save,
            name: String::new(),
            target: None,
            error: None,
        }
    }

    /// Build a rename dialog prefilled with the target name.
    pub fn rename(name: &str) -> Self {
        Self {
            kind: SavedDialogKind::Rename,
            name: name.to_string(),
            target: Some(name.to_string()),
            error: None,
        }
    }

    /// The dialog title key.
    fn title_key(&self) -> &'static str {
        match self.kind {
            SavedDialogKind::Save => "query.save_query_label",
            SavedDialogKind::Rename => "query.rename_saved_query",
        }
    }

    /// The dialog submit label key.
    fn submit_key(&self) -> &'static str {
        match self.kind {
            SavedDialogKind::Save => "query.save",
            SavedDialogKind::Rename => "query.rename",
        }
    }

    /// The explanatory caption key.
    fn caption_key(&self) -> &'static str {
        match self.kind {
            SavedDialogKind::Save => {
                "query.saved_queries_are_stored_in_your_vault_and_sync_across_devic"
            }
            SavedDialogKind::Rename => "query.renamed_saved_queries_sync_with_your_vault",
        }
    }
}

/// Render the open save/rename dialog.
pub fn dialog(view: &QueryView, cx: &mut Context<QueryView>) -> impl IntoElement {
    let theme = cx.theme();
    let Some(model) = view.dialog.as_ref() else {
        return div();
    };
    let title = t_global(model.title_key());
    let submit = t_global(model.submit_key());
    let caption = t_global(model.caption_key());
    let error = model.error.clone();
    let renaming = match model.kind {
        SavedDialogKind::Rename => model
            .target
            .clone()
            .map(|target| format!("{}{}\u{201d}.", t_global("query.renaming"), target)),
        SavedDialogKind::Save => None,
    };
    let input = view.dialog_input().cloned();
    overlay(
        theme.colors.background,
        theme.colors.secondary,
        theme.colors.border,
        420.0,
        save_fields(
            DialogLabels {
                title,
                submit,
                caption,
            },
            error,
            renaming,
            input,
            DialogColors {
                muted: theme.colors.muted_foreground,
                danger: theme.colors.danger,
                accent: theme.colors.accent,
                accent_foreground: theme.colors.accent_foreground,
                foreground: theme.colors.foreground,
            },
            cx,
        ),
    )
}

/// Render the delete confirmation.
pub fn delete_dialog(view: &QueryView, cx: &mut Context<QueryView>) -> impl IntoElement {
    let theme = cx.theme();
    let name = view.delete_target().unwrap_or_default().to_string();
    let message = format!(
        "{}{}\u{201d}? {}",
        t_global("query.delete_label"),
        name,
        t_global("common.this_cannot_be_undone")
    );
    overlay(
        theme.colors.background,
        theme.colors.secondary,
        theme.colors.border,
        400.0,
        delete_fields(
            message,
            DialogColors {
                muted: theme.colors.muted_foreground,
                danger: theme.colors.danger,
                accent: theme.colors.accent,
                accent_foreground: theme.colors.accent_foreground,
                foreground: theme.colors.foreground,
            },
            cx,
        ),
    )
}

/// The modal backdrop and panel frame shared by both dialogs.
fn overlay(
    background: Hsla,
    panel: Hsla,
    border: Hsla,
    width: f32,
    content: impl IntoElement,
) -> Div {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(background.opacity(0.6))
        .child(
            div()
                .w(px(width))
                .p(px(16.0))
                .rounded(px(8.0))
                .bg(panel)
                .border_1()
                .border_color(border)
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(content),
        )
}

/// The localized dialog labels.
struct DialogLabels {
    title: String,
    submit: String,
    caption: String,
}

/// The dialog color set.
struct DialogColors {
    muted: Hsla,
    danger: Hsla,
    accent: Hsla,
    accent_foreground: Hsla,
    foreground: Hsla,
}

/// The save/rename dialog fields.
fn save_fields(
    labels: DialogLabels,
    error: Option<String>,
    renaming: Option<String>,
    input: Option<Entity<gpui_kit::component::input::InputState>>,
    colors: DialogColors,
    cx: &mut Context<QueryView>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors.foreground)
                .child(labels.title),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(colors.muted)
                .child(t_global("query.name")),
        )
        .when_some(input, |element, input| {
            element.child(Input::new(&input).w_full())
        })
        .when_some(renaming, |element, message| {
            element.child(
                div()
                    .text_size(px(10.0))
                    .text_color(colors.muted)
                    .child(message),
            )
        })
        .when_some(error, |element, message| {
            element.child(
                div()
                    .text_size(px(11.0))
                    .text_color(colors.danger)
                    .child(message),
            )
        })
        .child(
            div()
                .text_size(px(10.0))
                .text_color(colors.muted)
                .child(labels.caption),
        )
        .child(action_row(
            ("query-dialog-cancel", t_global("query.cancel")),
            ("query-dialog-submit", labels.submit),
            &colors,
            cx,
            |view, cx| view.dismiss_dialog(cx),
            |view, cx| view.submit_dialog(cx),
        ))
}

/// The delete confirmation fields.
fn delete_fields(
    message: String,
    colors: DialogColors,
    cx: &mut Context<QueryView>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors.foreground)
                .child(t_global("query.delete_saved_query")),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(colors.muted)
                .child(message),
        )
        .child(action_row(
            ("query-delete-cancel", t_global("common.cancel")),
            ("query-delete-confirm", t_global("common.delete")),
            &colors,
            cx,
            |view, cx| view.cancel_delete(cx),
            |view, cx| view.confirm_delete(cx),
        ))
}

/// The cancel/submit button pair shared by both dialogs.
fn action_row(
    cancel: (&'static str, String),
    submit: (&'static str, String),
    colors: &DialogColors,
    cx: &mut Context<QueryView>,
    on_cancel: impl Fn(&mut QueryView, &mut Context<QueryView>) + 'static,
    on_submit: impl Fn(&mut QueryView, &mut Context<QueryView>) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_end()
        .gap(px(8.0))
        .child(dialog_button(
            cancel.0,
            cancel.1,
            colors.muted.opacity(0.15),
            colors.muted,
            cx,
            on_cancel,
        ))
        .child(dialog_button(
            submit.0,
            submit.1,
            colors.accent,
            colors.accent_foreground,
            cx,
            on_submit,
        ))
}

/// One dialog action button.
fn dialog_button(
    id: &'static str,
    label: String,
    background: Hsla,
    color: Hsla,
    cx: &mut Context<QueryView>,
    action: impl Fn(&mut QueryView, &mut Context<QueryView>) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .flex()
        .items_center()
        .justify_center()
        .px(px(12.0))
        .py(px(5.0))
        .rounded(px(6.0))
        .bg(background)
        .text_color(color)
        .text_size(px(12.0))
        .cursor_pointer()
        .child(label)
        .on_click(cx.listener(move |view, _, _, cx| action(view, cx)))
}
