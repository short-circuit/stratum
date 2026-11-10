//! Move/rename dialog and path validation.
//!
//! The shipped `MovePageDialog` trims the input, appends `.md` when the
//! extension is missing and rejects empty paths, paths outside `pages/` and
//! targets that already exist. [`validate_move`] is the pure model; the dialog
//! renderer binds it to a gpui input and the provider move call.

use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::*;
use crate::provider::normalize_move_path;

/// A move-path validation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveError {
    /// The input is empty.
    Empty,
    /// The input is outside the `pages/` namespace.
    NotInPages,
    /// A different page already uses the target path.
    Exists,
}

impl MoveError {
    /// The translation key for this validation failure.
    pub fn label_key(self) -> &'static str {
        match self {
            MoveError::Empty => "sidebar.path_must_not_be_empty",
            MoveError::NotInPages => "sidebar.path_must_start_with_pages",
            MoveError::Exists => "sidebar.page_already_exists",
        }
    }
}

/// Validate a user-entered move path against the current page and the existing
/// page paths.
///
/// Returns the normalized target path on success. The current path is allowed
/// as a target so submitting without changes is a no-op.
pub fn validate_move(raw: &str, current: &str, existing: &[String]) -> Result<String, MoveError> {
    let target = normalize_move_path(raw);
    if target.is_empty() {
        return Err(MoveError::Empty);
    }
    if !target.starts_with("pages/") {
        return Err(MoveError::NotInPages);
    }
    if target != current && existing.iter().any(|path| path == &target) {
        return Err(MoveError::Exists);
    }
    Ok(target)
}

impl PageView {
    /// Open the move/rename dialog.
    pub fn open_move_dialog(&mut self, cx: &mut Context<Self>) {
        self.move_open = true;
        self.move_input = None;
        self.move_error = None;
        self.move_provider_error = None;
        self.move_busy = false;
        cx.notify();
    }

    /// Close the move/rename dialog.
    pub fn close_move_dialog(&mut self, cx: &mut Context<Self>) {
        self.move_open = false;
        self.move_input = None;
        self.move_error = None;
        self.move_provider_error = None;
        self.move_busy = false;
        cx.notify();
    }

    /// Create the input state when the dialog opens.
    pub(super) fn sync_move_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.move_open || self.move_input.is_some() {
            return;
        }
        let path = self.path.clone();
        self.move_input = Some(cx.new(move |cx| InputState::new(window, cx).default_value(path)));
    }

    /// Validate the input and run the provider move.
    pub fn submit_move(&mut self, cx: &mut Context<Self>) {
        if self.move_busy {
            return;
        }
        let Some(input) = self.move_input.clone() else {
            return;
        };
        let raw = input.read(cx).value().to_string();
        let existing: Vec<String> = self
            .provider
            .snapshot()
            .recents
            .into_iter()
            .map(|page| page.path)
            .collect();
        let target = match validate_move(&raw, &self.path, &existing) {
            Ok(target) => target,
            Err(error) => {
                self.move_error = Some(error);
                self.move_provider_error = None;
                cx.notify();
                return;
            }
        };
        if target == self.path {
            self.close_move_dialog(cx);
            return;
        }

        self.move_busy = true;
        self.move_error = None;
        self.move_provider_error = None;
        let provider = Arc::clone(&self.provider);
        let old = self.path.clone();
        let task_old = old.clone();
        cx.spawn(async move |this: WeakEntity<PageView>, cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move { provider.move_page(&task_old, &target) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.move_busy = false;
                match result {
                    Ok(new_path) => {
                        view.close_request = Some(PageClose::Moved {
                            old: old.clone(),
                            new: new_path,
                        });
                        view.move_open = false;
                        view.move_input = None;
                        view.move_error = None;
                        view.move_provider_error = None;
                    }
                    Err(message) => {
                        view.move_provider_error = Some(message);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}

/// Render the move/rename dialog.
pub fn move_dialog(view: &PageView, cx: &mut Context<PageView>) -> impl IntoElement {
    let theme = cx.theme();
    let background = theme.colors.background;
    let panel = theme.colors.secondary;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let danger = theme.colors.danger;
    let accent = theme.colors.accent;
    let accent_foreground = theme.colors.accent_foreground;

    let error = view
        .move_error
        .map(|error| t_global(error.label_key()))
        .or_else(|| view.move_provider_error.clone());

    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(background.opacity(0.6))
        .child(
            div()
                .w(px(420.0))
                .p(px(16.0))
                .rounded(px(8.0))
                .bg(panel)
                .border_1()
                .border_color(border)
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(
                    div()
                        .text_size(px(14.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(foreground)
                        .child(t_global("sidebar.move_page_title")),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(muted)
                        .child(t_global("sidebar.new_path")),
                )
                .when_some(view.move_input.clone(), |element, input| {
                    element.child(Input::new(&input).w_full())
                })
                .when_some(error, |element, message| {
                    element.child(div().text_size(px(11.0)).text_color(danger).child(message))
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(8.0))
                        .child(dialog_button(
                            "page-move-cancel",
                            t_global("sidebar.cancel"),
                            muted.opacity(0.15),
                            muted,
                            cx,
                            |view, cx| view.close_move_dialog(cx),
                        ))
                        .child(dialog_button(
                            "page-move-submit",
                            t_global("sidebar.move"),
                            accent,
                            accent_foreground,
                            cx,
                            |view, cx| view.submit_move(cx),
                        )),
                ),
        )
}

/// One dialog action button.
fn dialog_button(
    id: &'static str,
    label: String,
    background: Hsla,
    color: Hsla,
    cx: &mut Context<PageView>,
    action: impl Fn(&mut PageView, &mut Context<PageView>) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;
    use stratum_i18n::{t, Locale};

    #[test]
    fn move_validation_trims_and_appends_the_extension() {
        assert_eq!(
            validate_move("  pages/New  ", "pages/Old.md", &[]),
            Ok("pages/New.md".to_string())
        );
        assert_eq!(
            validate_move("pages/New.md", "pages/Old.md", &[]),
            Ok("pages/New.md".to_string())
        );
        assert_eq!(
            validate_move("pages/Old.md", "pages/Old.md", &[]),
            Ok("pages/Old.md".to_string())
        );
    }

    #[test]
    fn move_validation_rejects_empty_foreign_and_existing_targets() {
        assert_eq!(
            validate_move("   ", "pages/Old.md", &[]),
            Err(MoveError::Empty)
        );
        assert_eq!(
            validate_move("notes/New.md", "pages/Old.md", &[]),
            Err(MoveError::NotInPages)
        );
        let existing = vec!["pages/New.md".to_string()];
        assert_eq!(
            validate_move("pages/New.md", "pages/Old.md", &existing),
            Err(MoveError::Exists)
        );
    }

    #[test]
    fn move_errors_are_localized_in_every_catalog() {
        for locale in Locale::ALL {
            for error in [MoveError::Empty, MoveError::NotInPages, MoveError::Exists] {
                let key = error.label_key();
                assert_ne!(t(locale, key), key, "{key} missing in {}", locale.code());
                assert!(super::super::PAGE_I18N_KEYS.contains(&key));
            }
        }
    }
}
