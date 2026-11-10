//! Note menu for the page header.
//!
//! The menu mirrors the shipped desktop `page.note` popup: an actions
//! subheader, reindex, normalize, move/rename and delete. The item table is a
//! pure model so the contract test can pin the labels and order without a
//! window; the renderer only dispatches to [`super::PageView`].

use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use super::PageView;

/// One Note menu action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteAction {
    /// Reindex the page from disk.
    Reindex,
    /// Normalize the page through the block parser.
    Normalize,
    /// Open the move/rename dialog.
    Move,
    /// Open the delete confirmation.
    Delete,
}

/// The shipped Note menu order.
pub const NOTE_MENU_ACTIONS: [NoteAction; 4] = [
    NoteAction::Reindex,
    NoteAction::Normalize,
    NoteAction::Move,
    NoteAction::Delete,
];

impl NoteAction {
    /// The translation key for this action's label.
    pub fn label_key(self) -> &'static str {
        match self {
            NoteAction::Reindex => "page.reindex_note",
            NoteAction::Normalize => "page.normalize_file",
            NoteAction::Move => "sidebar.move_rename",
            NoteAction::Delete => "page.delete_page",
        }
    }

    /// Whether the action renders in the danger color.
    pub fn is_destructive(self) -> bool {
        matches!(self, NoteAction::Delete)
    }
}

impl PageView {
    /// Close the menu and run the selected action.
    pub fn select_action(&mut self, action: NoteAction, cx: &mut Context<Self>) {
        self.close_menu(cx);
        match action {
            NoteAction::Reindex => self.reindex(cx),
            NoteAction::Normalize => self.normalize(cx),
            NoteAction::Move => self.open_move_dialog(cx),
            NoteAction::Delete => self.open_delete_dialog(cx),
        }
    }
}

/// Render the Note menu popup.
pub fn note_menu(view: &PageView, cx: &mut Context<PageView>) -> impl IntoElement {
    let theme = cx.theme();
    let panel = theme.colors.secondary;
    let border = theme.colors.border;
    let foreground = theme.colors.foreground;
    let muted = theme.colors.muted_foreground;
    let danger = theme.colors.danger;

    let mut menu = div()
        .absolute()
        .top(px(52.0))
        .right(px(12.0))
        .w(px(220.0))
        .py(px(4.0))
        .rounded(px(8.0))
        .bg(panel)
        .border_1()
        .border_color(border)
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(10.0))
                .py(px(3.0))
                .text_size(px(10.0))
                .text_color(muted)
                .child(t_global("page.actions")),
        );

    for action in NOTE_MENU_ACTIONS {
        let disabled = view.reindexing() && action == NoteAction::Reindex;
        let color = if action.is_destructive() {
            danger
        } else {
            foreground
        };
        menu = menu.child(
            div()
                .id(format!("page-note-{}", action.label_key()))
                .px(px(10.0))
                .py(px(5.0))
                .text_size(px(12.0))
                .text_color(color.opacity(if disabled { 0.5 } else { 1.0 }))
                .cursor_pointer()
                .child(t_global(action.label_key()))
                .on_click(cx.listener(move |view, _, _, cx| {
                    if disabled {
                        return;
                    }
                    view.select_action(action, cx);
                })),
        );
    }

    menu
}

#[cfg(test)]
mod tests {
    use super::*;
    // `gpui_kit::*` re-exports gpui's `test` attribute macro, which shadows
    // the built-in attribute. Import the built-in explicitly so these plain
    // unit tests stay cheap (no GPUI app context is created).
    use ::core::prelude::v1::test;
    use stratum_i18n::{t, Locale};

    #[test]
    fn note_menu_actions_are_localized_in_every_catalog() {
        for locale in Locale::ALL {
            for action in NOTE_MENU_ACTIONS {
                let key = action.label_key();
                assert_ne!(t(locale, key), key, "{key} missing in {}", locale.code());
                assert!(super::super::PAGE_I18N_KEYS.contains(&key));
            }
        }
    }

    #[test]
    fn only_delete_is_destructive() {
        for action in NOTE_MENU_ACTIONS {
            assert_eq!(action.is_destructive(), action == NoteAction::Delete);
        }
    }
}
