//! Page view state accessors and editor lifecycle.
//!
//! These methods are the read-only surface the shell, header and contract tests
//! use, plus the read-mode switch, editor reload and the navigation/snippet
//! plumbing shared by the backlinks card.

use super::*;

impl PageView {
    /// The vault-relative page path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Whether the page is in read mode.
    pub fn read_mode(&self) -> bool {
        self.read_mode
    }

    /// Page metadata, when the provider could read it.
    pub fn page_info(&self) -> Option<&PageInfo> {
        self.info.as_ref()
    }

    /// The live editor entity.
    pub fn editor(&self) -> Option<&Entity<EditorView>> {
        self.editor.as_ref()
    }

    /// The voice-capture entity.
    pub fn voice(&self) -> &Entity<VoiceCapture> {
        &self.voice
    }

    /// Whether the Note menu is open.
    pub fn menu_open(&self) -> bool {
        self.menu_open
    }

    /// Whether the move dialog is open.
    pub fn move_dialog_open(&self) -> bool {
        self.move_open
    }

    /// Whether the delete confirmation is open.
    pub fn delete_dialog_open(&self) -> bool {
        self.delete_open
    }

    /// Whether a reindex request is running.
    pub fn reindexing(&self) -> bool {
        self.reindexing
    }

    /// The current error message, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The current backlink rows.
    pub fn backlinks(&self) -> &[BacklinkRow] {
        &self.backlinks
    }

    /// Whether the backlinks card is expanded.
    pub fn backlinks_open(&self) -> bool {
        self.backlinks_open
    }

    /// The current related-note suggestions state.
    pub fn suggestions(&self) -> &SuggestionsState {
        &self.suggestions
    }

    /// Whether the suggestions card is expanded.
    pub fn suggestions_open(&self) -> bool {
        self.suggestions_open
    }

    /// The loaded backlink snippet, if any.
    pub fn snippet(&self) -> Option<&BacklinkSnippet> {
        self.snippet.as_ref()
    }

    /// The snippet load error, if any.
    pub fn snippet_error(&self) -> Option<&str> {
        self.snippet_error.as_deref()
    }

    /// A pending navigation target produced by a child editor or a backlink.
    pub fn take_pending_navigation(&mut self) -> Option<String> {
        self.pending_navigation.take()
    }

    /// A pending page close request consumed by the shell.
    pub fn take_close_request(&mut self) -> Option<PageClose> {
        self.close_request.take()
    }

    /// A success message the shell should push to the toaster.
    pub fn take_toast(&mut self) -> Option<String> {
        self.toast.take()
    }

    /// Switch between edit and read mode.
    pub fn set_read_mode(&mut self, read_mode: bool, cx: &mut Context<Self>) {
        if self.read_mode == read_mode {
            return;
        }
        self.read_mode = read_mode;
        if let Some(editor) = &self.editor {
            editor.update(cx, |view, cx| view.set_read_mode(read_mode, cx));
        }
        cx.notify();
    }

    /// Open or close the Note menu.
    pub fn toggle_menu(&mut self, cx: &mut Context<Self>) {
        self.menu_open = !self.menu_open;
        cx.notify();
    }

    /// Close the Note menu.
    pub fn close_menu(&mut self, cx: &mut Context<Self>) {
        if self.menu_open {
            self.menu_open = false;
            cx.notify();
        }
    }

    /// Expand or collapse both page cards.
    pub fn set_cards_open(&mut self, open: bool, cx: &mut Context<Self>) {
        self.backlinks_open = open;
        self.suggestions_open = open;
        cx.notify();
    }

    /// Toggle the backlinks card.
    pub fn toggle_backlinks(&mut self, cx: &mut Context<Self>) {
        self.backlinks_open = !self.backlinks_open;
        cx.notify();
    }

    /// Toggle the suggestions card.
    pub fn toggle_suggestions(&mut self, cx: &mut Context<Self>) {
        self.suggestions_open = !self.suggestions_open;
        cx.notify();
    }

    /// Request navigation to another page.
    pub fn navigate_to(&mut self, page: &str) {
        self.pending_navigation = Some(page.to_string());
    }

    /// Load the bounded snippet around a backlink anchor.
    pub fn load_snippet(&mut self, note_id: &str, anchor_id: &str, cx: &mut Context<Self>) {
        self.snippet = self.provider.backlink_snippet(note_id, anchor_id);
        self.snippet_error = if self.snippet.is_some() {
            None
        } else {
            Some(t_global(
                "backlink_snippet_body.could_not_load_the_backlinked_snippet",
            ))
        };
        cx.notify();
    }

    /// Dismiss the snippet card.
    pub fn clear_snippet(&mut self, cx: &mut Context<Self>) {
        if self.snippet.is_some() || self.snippet_error.is_some() {
            self.snippet = None;
            self.snippet_error = None;
            cx.notify();
        }
    }

    /// Recreate the editor so reloaded content becomes visible.
    pub(crate) fn reload_editor(&mut self, cx: &mut Context<Self>) {
        let provider = Arc::clone(&self.provider);
        let path = self.path.clone();
        let read_mode = self.read_mode;
        self.editor = Some(cx.new(move |cx| EditorView::new(path, provider, read_mode, cx)));
        cx.notify();
    }

    /// Reload page metadata after a reindex or normalize.
    pub(crate) fn reload_page_info(&mut self) {
        self.info = self.provider.page_info(&self.path);
    }

    /// Open the platform file picker through the editor's attachment flow.
    pub fn attach_files(&mut self, cx: &mut Context<Self>) {
        if let Some(editor) = &self.editor {
            editor.update(cx, |view, cx| view.attach_files(cx));
        }
    }

    /// Copy a child editor's pending navigation into the page view.
    pub(super) fn sync_child_navigation(&mut self, cx: &mut Context<Self>) {
        if self.pending_navigation.is_some() {
            return;
        }
        let Some(editor) = self.editor.clone() else {
            return;
        };
        let target = editor.update(cx, |view, _| view.pending_navigation().map(str::to_string));
        if let Some(target) = target {
            editor.update(cx, |view, _| view.clear_pending_navigation());
            self.pending_navigation = Some(target);
        }
    }
}
