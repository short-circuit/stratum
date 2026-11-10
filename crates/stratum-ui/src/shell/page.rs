//! Page-panel lifecycle for the shell.
//!
//! Opening a page, forwarding read-mode changes and consuming the page's
//! navigation/toast/close requests live here so [`super::mod`] stays under the
//! repository file-length limit. The page entity owns its editor; the shell
//! keeps a cached editor handle only for its public accessor.

use std::sync::Arc;

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::tr_global;

use super::Shell;
use crate::panels::editor::EditorView;
use crate::panels::page::{PageClose, PageStart, PageView};
use crate::router::Panel;

impl Shell {
    /// Open a page in the page panel. The entity is created lazily by `render`.
    pub fn open_page(&mut self, path: impl Into<String>) {
        self.open_page_with_start(path.into(), PageStart::default());
    }

    /// Open a page and reveal `block_id` once the editor is built.
    ///
    /// Used by navigation that carries a block anchor (for example a backlink
    /// click). The highlight clears on the next edit.
    pub fn open_page_with_highlight(
        &mut self,
        path: impl Into<String>,
        block_id: impl Into<String>,
    ) {
        self.open_page_with_start(
            path.into(),
            PageStart {
                highlight: Some(block_id.into()),
                ..PageStart::default()
            },
        );
    }

    /// Open a page with a deterministic start state.
    fn open_page_with_start(&mut self, path: String, mut start: PageStart) {
        self.current_page = Some(path);
        self.active = Panel::Page;
        self.read_mode = false;
        start.read_mode = false;
        self.page_start = start;
        self.page = None;
        self.page_editor = None;
        self.page_read_mode = false;
    }

    /// The live page entity, if the page panel has created one.
    pub fn page_view(&self) -> Option<&Entity<PageView>> {
        self.page.as_ref()
    }

    /// The live editor entity owned by the page panel, if any.
    ///
    /// This delegates to the cached handle the page view exposes so existing
    /// callers of the shell API keep working after the page panel took over
    /// editor ownership.
    pub fn editor(&self) -> Option<&Entity<EditorView>> {
        self.page_editor.as_ref()
    }

    /// Set the initial page-panel state before the page entity exists.
    ///
    /// Called by the capture harness from `STRATUM_MENU`, `STRATUM_CARDS` and
    /// `STRATUM_HIGHLIGHT`. A `None` highlight preserves any highlight already
    /// requested through [`Shell::open_page_with_highlight`]. Recreating the page
    /// entity makes a later call take effect deterministically.
    pub fn set_page_start(&mut self, menu_open: bool, cards_open: bool, highlight: Option<String>) {
        self.page_start.menu_open = menu_open;
        self.page_start.cards_open = cards_open;
        if highlight.is_some() {
            self.page_start.highlight = highlight;
        }
        self.page = None;
        self.page_editor = None;
    }

    /// Create or refresh the page entity for the current page, forward shell
    /// read-mode changes and consume the page's navigation, toast and close
    /// requests.
    ///
    /// The page header owns its own read-mode toggle, so the shell only forwards
    /// a change when its own read-mode flag differs from the last value it
    /// pushed; otherwise the header toggle would be overwritten every frame.
    pub(super) fn sync_page(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.current_page.clone() else {
            self.page = None;
            self.page_editor = None;
            return;
        };
        let needs_new = match &self.page {
            Some(page) => page.read(cx).path() != path,
            None => true,
        };
        if needs_new {
            let provider = Arc::clone(&self.provider);
            let mut start = std::mem::take(&mut self.page_start);
            let read_mode = self.read_mode;
            start.read_mode = read_mode;
            self.page_read_mode = read_mode;
            self.page = Some(cx.new(move |cx| PageView::new(provider, path, start, cx)));
        } else if let Some(page) = &self.page {
            if self.read_mode != self.page_read_mode {
                let read_mode = self.read_mode;
                page.update(cx, |view, cx| view.set_read_mode(read_mode, cx));
                self.page_read_mode = read_mode;
            }
        }

        let page = self.page.clone();
        let Some(page) = page else {
            return;
        };

        let navigation = page.update(cx, |view, _| view.take_pending_navigation());
        if let Some(target) = navigation {
            self.handle_editor_navigation(&target);
        }

        let toast = page.update(cx, |view, _| view.take_toast());
        if let Some(message) = toast {
            self.push_toast(message);
        }

        let close = page.update(cx, |view, _| view.take_close_request());
        if let Some(close) = close {
            self.handle_page_close(close);
        }

        self.page_editor = self
            .page
            .as_ref()
            .and_then(|page| page.update(cx, |view, _| view.editor().cloned()));
    }

    /// Handle a page close request produced by the page panel.
    ///
    /// A deleted page clears the route and returns to the journal home, while a
    /// moved page shows the localized success toast and opens the new path.
    fn handle_page_close(&mut self, close: PageClose) {
        match close {
            PageClose::Deleted => {
                self.current_page = None;
                self.page = None;
                self.page_editor = None;
                self.page_start = PageStart::default();
                self.read_mode = false;
                self.active = Panel::Journal;
            }
            PageClose::Moved { old, new } => {
                self.push_toast(tr_global(
                    "sidebar.move_page_success",
                    &[("old", old.as_str()), ("new", new.as_str())],
                ));
                self.open_page(new);
            }
        }
    }
}
