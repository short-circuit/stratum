//! Page surface for the native shell.
//!
//! [`PageView`] reproduces the shipped desktop page panel: a header with the
//! page title, path chip, voice/attach/read-mode controls and the Note menu
//! (reindex, normalize, move, delete), followed by the block editor, the
//! backlinks card and the related-note suggestions card.
//!
//! The module is split so every file stays under the repository limit and the
//! pure models are testable without a window:
//!
//! - [`header`] — the page header row.
//! - [`menu`] — the Note menu items and popup.
//! - [`move_dialog`] — move/rename dialog plus path validation.
//! - [`delete_dialog`] — delete confirmation dialog.
//! - [`backlinks`] — linked references, unlinked mentions and the snippet card.
//! - [`suggestions`] — related-note suggestions state and card.
//! - [`actions`] — async reindex/normalize/delete flows and toast plumbing.
//! - [`state`] — accessors, read-mode forwarding and editor reloads.
//!
//! [`PAGE_I18N_KEYS`] is the release gate list for every string this surface
//! renders.

use std::sync::Arc;

use gpui_kit::component::input::InputState;
use gpui_kit::component::ActiveTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::panels::editor::EditorView;
use crate::panels::voice::VoiceCapture;
use crate::provider::{BacklinkRow, BacklinkSnippet, PageInfo, ShellData};

pub mod actions;
pub mod backlinks;
pub mod delete_dialog;
pub mod header;
pub mod menu;
pub mod move_dialog;
mod state;
pub mod suggestions;

pub use suggestions::SuggestionsState;

/// Every translation key the page surface renders.
///
/// The contract test walks this list against all eight catalogs, so adding a
/// visible string without a catalog entry fails the build instead of leaking a
/// raw key into the UI.
pub const PAGE_I18N_KEYS: &[&str] = &[
    "asset.attach",
    "backlink_snippet_body.backlinked_note",
    "backlink_snippet_body.could_not_load_the_backlinked_snippet",
    "backlinks.backlinked_snippet",
    "backlinks.backlinks",
    "backlinks.close_snippet",
    "backlinks.linked_references",
    "backlinks.no_backlinks_found",
    "backlinks.unlinked_mentions",
    "common.cancel",
    "common.loading",
    "page.actions",
    "page.delete",
    "page.delete_failed",
    "page.delete_page",
    "page.edit_mode",
    "page.normalize_failed",
    "page.normalize_file",
    "page.note",
    "page.read_mode",
    "page.reindex_failed",
    "page.reindex_note",
    "page.reindexed_page",
    "page.voice_memo_dictation",
    "sidebar.cancel",
    "sidebar.move",
    "sidebar.move_page_success",
    "sidebar.move_page_title",
    "sidebar.move_rename",
    "sidebar.new_path",
    "sidebar.page_already_exists",
    "sidebar.path_must_not_be_empty",
    "sidebar.path_must_start_with_pages",
    "suggested_connections.add_wiki_link_to_this_page",
    "suggested_connections.click_find_to_discover_related_notes",
    "suggested_connections.failed_to_add_link",
    "suggested_connections.failed_to_find_connections",
    "suggested_connections.find",
    "suggested_connections.no_connections_found",
    "suggested_connections.relevance",
    "suggested_connections.searching",
    "suggested_connections.see_also",
    "suggested_connections.suggested_connections",
];

/// A page-panel close request consumed by the shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageClose {
    /// The page was deleted; the shell clears the route.
    Deleted,
    /// The page was moved; the shell opens the new path and shows a toast.
    Moved {
        /// Previous vault-relative path.
        old: String,
        /// Stored vault-relative path after the move.
        new: String,
    },
}

/// Deterministic start state for the page panel.
///
/// The capture harness uses this to open the Note menu, expand the cards and
/// highlight a block on open without synthesising pointer events.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PageStart {
    /// Start in read mode.
    pub read_mode: bool,
    /// Block id to reveal after the editor is built.
    pub highlight: Option<String>,
    /// Start with the Note menu open.
    pub menu_open: bool,
    /// Start with the backlinks and suggestions cards expanded.
    pub cards_open: bool,
}

/// The page panel entity.
pub struct PageView {
    provider: Arc<dyn ShellData>,
    path: String,
    read_mode: bool,
    info: Option<PageInfo>,
    editor: Option<Entity<EditorView>>,
    voice: Entity<VoiceCapture>,
    menu_open: bool,
    move_open: bool,
    move_input: Option<Entity<InputState>>,
    move_error: Option<move_dialog::MoveError>,
    move_provider_error: Option<String>,
    move_busy: bool,
    delete_open: bool,
    delete_busy: bool,
    reindexing: bool,
    backlinks: Vec<BacklinkRow>,
    backlinks_open: bool,
    suggestions: SuggestionsState,
    suggestions_open: bool,
    snippet: Option<BacklinkSnippet>,
    snippet_error: Option<String>,
    error: Option<String>,
    toast: Option<String>,
    pending_navigation: Option<String>,
    close_request: Option<PageClose>,
}

impl PageView {
    /// Open `path` through `provider` and build the page surface.
    ///
    /// The editor is created up front so a reveal request can scroll to its
    /// block before the first paint. A missing page starts as an empty editor.
    pub fn new(
        provider: Arc<dyn ShellData>,
        path: impl Into<String>,
        start: PageStart,
        cx: &mut Context<Self>,
    ) -> Self {
        let path = path.into();
        let info = provider.page_info(&path);
        let backlinks = provider.backlinks(&path);
        let read_mode = start.read_mode;

        let editor_path = path.clone();
        let editor_provider = Arc::clone(&provider);
        let editor = cx.new(move |cx| EditorView::new(editor_path, editor_provider, read_mode, cx));
        if let Some(block_id) = &start.highlight {
            editor.update(cx, |view, _| {
                view.reveal_block(block_id);
            });
        }

        let voice_path = path.clone();
        let voice_provider = Arc::clone(&provider);
        let voice = cx.new(move |cx| VoiceCapture::new(voice_path, voice_provider, cx));

        Self {
            provider,
            path,
            read_mode,
            info,
            editor: Some(editor),
            voice,
            menu_open: start.menu_open,
            move_open: false,
            move_input: None,
            move_error: None,
            move_provider_error: None,
            move_busy: false,
            delete_open: false,
            delete_busy: false,
            reindexing: false,
            backlinks,
            backlinks_open: start.cards_open,
            suggestions: SuggestionsState::Idle,
            suggestions_open: start.cards_open,
            snippet: None,
            snippet_error: None,
            error: None,
            toast: None,
            pending_navigation: None,
            close_request: None,
        }
    }
}

impl Render for PageView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self
            .voice
            .update(cx, |voice, _| voice.take_reload_request())
        {
            self.reload_editor(cx);
        }
        self.sync_child_navigation(cx);
        self.sync_move_input(window, cx);

        let theme = cx.theme();
        let editor = self.editor.clone();

        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .child(header::header(self, cx))
            .when(self.error.is_some(), |element| {
                element.child(error_bar(self, cx))
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_hidden()
                    .when_some(editor, |element, editor| element.child(editor))
                    .when(self.editor.is_none(), |element| {
                        element.child(
                            div()
                                .p(px(16.0))
                                .text_size(px(12.0))
                                .child(t_global("common.loading")),
                        )
                    }),
            )
            .child(backlinks::backlinks_section(self, cx))
            .child(suggestions::suggestions_section(self, cx))
            .when(self.menu_open, |element| {
                element.child(menu::note_menu(self, cx))
            })
            .when(self.move_open, |element| {
                element.child(move_dialog::move_dialog(self, cx))
            })
            .when(self.delete_open, |element| {
                element.child(delete_dialog::delete_dialog(self, cx))
            })
    }
}

/// Render the dismissable page error bar.
fn error_bar(view: &PageView, cx: &mut Context<PageView>) -> impl IntoElement {
    let theme = cx.theme();
    let danger = theme.colors.danger;
    let muted = theme.colors.muted_foreground;
    let message = view.error().unwrap_or_default().to_string();

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .mx(px(12.0))
        .my(px(8.0))
        .px(px(10.0))
        .py(px(6.0))
        .rounded(px(6.0))
        .bg(danger.opacity(0.12))
        .child(div().text_size(px(12.0)).text_color(danger).child(message))
        .child(
            div()
                .id("page-error-dismiss")
                .text_size(px(11.0))
                .text_color(muted)
                .cursor_pointer()
                .child(t_global("common.cancel"))
                .on_click(cx.listener(|view, _, _, cx| {
                    view.error = None;
                    cx.notify();
                })),
        )
}
