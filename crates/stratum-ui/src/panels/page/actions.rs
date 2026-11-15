//! Page menu actions: reindex, normalize, delete and toast plumbing.
//!
//! Each action runs the provider call on the background executor, then updates
//! the page metadata/editor or records a close request for the shell. The
//! methods are thin state transitions so the contract tests can exercise the
//! pure pieces without a window.

use std::sync::Arc;

use stratum_i18n::t_global;

use super::*;

impl PageView {
    /// Reindex the page from disk into SQLite and the search index.
    pub fn reindex(&mut self, cx: &mut Context<Self>) {
        if self.reindexing {
            return;
        }
        self.reindexing = true;
        self.error = None;
        let provider = Arc::clone(&self.provider);
        let path = self.path.clone();
        cx.spawn(async move |this: WeakEntity<PageView>, cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move { provider.reindex_page(&path) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.reindexing = false;
                match result {
                    Ok(summary) => {
                        view.reload_page_info();
                        view.reload_editor(cx);
                        if summary.failed > 0 {
                            view.error = Some(format!(
                                "{} {}",
                                t_global("page.reindex_failed"),
                                summary.errors.join(", ")
                            ));
                        } else {
                            view.toast = Some(format!(
                                "{} {}",
                                t_global("page.reindexed_page"),
                                summary.succeeded
                            ));
                        }
                    }
                    Err(message) => {
                        view.error = Some(format!("{} {message}", t_global("page.reindex_failed")));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Normalize the page file through the block parser.
    pub fn normalize(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        let provider = Arc::clone(&self.provider);
        let path = self.path.clone();
        cx.spawn(async move |this: WeakEntity<PageView>, cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move { provider.normalize_page(&path) })
                .await;
            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(()) => {
                        view.reload_page_info();
                        view.reload_editor(cx);
                    }
                    Err(message) => {
                        view.error =
                            Some(format!("{} {message}", t_global("page.normalize_failed")));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Open the delete confirmation.
    pub fn open_delete_dialog(&mut self, cx: &mut Context<Self>) {
        self.delete_open = true;
        self.delete_busy = false;
        cx.notify();
    }

    /// Dismiss the delete confirmation.
    pub fn cancel_delete(&mut self, cx: &mut Context<Self>) {
        self.delete_open = false;
        self.delete_busy = false;
        cx.notify();
    }

    /// Delete the page and request the shell to clear the route.
    pub fn confirm_delete(&mut self, cx: &mut Context<Self>) {
        if self.delete_busy {
            return;
        }
        self.delete_open = false;
        self.delete_busy = true;
        self.error = None;
        let provider = Arc::clone(&self.provider);
        let path = self.path.clone();
        cx.spawn(async move |this: WeakEntity<PageView>, cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move { provider.delete_page(&path) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.delete_busy = false;
                match result {
                    Ok(()) => {
                        view.close_request = Some(PageClose::Deleted);
                    }
                    Err(message) => {
                        view.error = Some(format!("{} {message}", t_global("page.delete_failed")));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}
