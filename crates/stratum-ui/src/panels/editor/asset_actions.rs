//! Attachment actions for the native block editor.
//!
//! This module owns the `EditorView` side of the attachment feature: the
//! import/delete/thumbnail orchestration, the platform file picker and the
//! background file reads. The pure models (row labels, cache, task selection)
//! live in [`crate::panels::editor::asset_state`]; the persistence lives behind
//! [`ShellData`]. Splitting the actions from the models keeps both files under
//! the repository file-length limit.
//!
//! Imports never touch the UI thread: file reads and provider calls run on the
//! gpui background executor and the result is applied back on the entity.
//! Provider errors are passed through unchanged so the panel can translate the
//! `file_too_large` sentinel.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::*;
use stratum_editor::AttachmentRow;
use stratum_i18n::t_global;
use uuid::Uuid;

use crate::panels::editor::asset_state::{next_thumbnail_task, ThumbnailTask};
use crate::panels::editor::EditorView;
use crate::provider::ShellData;

/// Import every dropped file path through the provider.
///
/// Runs on the background executor: the provider reads the file and applies the
/// upload policy in the service layer, so an 8 MiB drop never round-trips
/// through base64 in the shell. The first failure aborts the batch and is
/// returned unchanged so the panel can map the size sentinel.
fn import_paths_blocking(
    provider: Arc<dyn ShellData>,
    paths: Vec<PathBuf>,
) -> Result<Vec<AttachmentRow>, String> {
    let mut rows = Vec::with_capacity(paths.len());
    for path in paths {
        rows.push(provider.import_asset_path(&path.to_string_lossy())?);
    }
    Ok(rows)
}

impl EditorView {
    /// Reload the attachment list of the open page from the provider.
    ///
    /// Providers without asset support return an empty list; the fixture
    /// provider returns the imported rows whose references appear in the saved
    /// page content.
    pub(crate) fn refresh_attachments(&mut self) {
        self.attachments = self.provider.page_attachments(&self.path);
    }

    /// The attachment record for a vault-relative asset path, if known.
    pub fn attachment_row(&self, ref_path: &str) -> Option<&AttachmentRow> {
        self.attachments.iter().find(|row| row.ref_path == ref_path)
    }

    /// Lazily decode thumbnails for the visible range.
    ///
    /// The shared [`EditorView::asset_budget`] caps provider decodes at one new
    /// thumbnail per frame; cached thumbnails are attached without spending
    /// budget. A failed decode leaves the row's `thumbnail` empty so the chip
    /// fallback stays visible.
    pub(crate) fn ensure_thumbnails_for_range(&mut self, range: std::ops::Range<usize>) {
        while self.asset_budget > 0 {
            let Some(task) = next_thumbnail_task(
                &self.rows,
                &self.thumbnails,
                range.clone(),
                self.asset_budget,
            ) else {
                break;
            };
            match task {
                ThumbnailTask::Cached { index, thumbnail } => {
                    if let Some(row) = self.rows.get_mut(index) {
                        row.thumbnail = Some(thumbnail);
                    }
                }
                ThumbnailTask::Fetch { index, path } => {
                    self.asset_budget = self.asset_budget.saturating_sub(1);
                    if let Some(thumbnail) = self.provider.read_asset_thumbnail(&path) {
                        self.thumbnails.insert(&path, thumbnail);
                        if let Some(row) = self.rows.get_mut(index) {
                            row.thumbnail = self.thumbnails.get(&path);
                        }
                    }
                }
            }
        }
    }

    /// Import files dropped onto the editor.
    ///
    /// `block_id` is the drop target row (or the caret block); the imported
    /// references are inserted there once the background import completes.
    pub fn start_import_paths(
        &mut self,
        paths: Vec<PathBuf>,
        block_id: Option<Uuid>,
        cx: &mut Context<Self>,
    ) {
        if paths.is_empty() {
            return;
        }
        self.begin_import(cx);
        let provider = self.provider.clone();
        cx.spawn(
            async move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move { import_paths_blocking(provider, paths) })
                    .await;
                let _ = this.update(cx, |view, cx| view.finish_import(result, block_id, cx));
            },
        )
        .detach();
    }

    /// Import pasted bytes (a clipboard image) as an attachment.
    ///
    /// The MIME is passed through so the service upload policy treats the
    /// payload as an image and compresses it when it exceeds the size limit.
    pub fn start_import_bytes(
        &mut self,
        bytes: Vec<u8>,
        name: String,
        mime: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        if bytes.is_empty() {
            return;
        }
        self.begin_import(cx);
        let provider = self.provider.clone();
        let mime = mime.map(str::to_string);
        let block_id = self.state.caret().map(|caret| caret.block_id);
        cx.spawn(
            async move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(
                        async move { provider.import_asset_bytes(bytes, &name, mime.as_deref()) },
                    )
                    .await
                    .map(|row| vec![row]);
                let _ = this.update(cx, |view, cx| view.finish_import(result, block_id, cx));
            },
        )
        .detach();
    }

    /// Open the platform file picker and import the selected files.
    ///
    /// The picker result arrives asynchronously; a cancelled or failed prompt
    /// leaves the attachment state untouched.
    pub(crate) fn attach_files(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some(t_global("asset.attach").into()),
        });
        cx.spawn(
            async move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
                let paths = match receiver.await {
                    Ok(Ok(Some(paths))) => paths,
                    _ => Vec::new(),
                };
                if paths.is_empty() {
                    return;
                }
                let _ = this.update(cx, |view, cx| {
                    let block_id = view.state.caret().map(|caret| caret.block_id);
                    view.start_import_paths(paths, block_id, cx);
                });
            },
        )
        .detach();
    }

    /// Two-step delete: the first click arms `ref_path`, the second deletes it.
    ///
    /// Arming another row moves the pending target, matching the shipped
    /// inline confirmation instead of a modal dialog.
    pub fn click_delete(&mut self, ref_path: &str, cx: &mut Context<Self>) {
        if self.pending_delete.as_deref() == Some(ref_path) {
            self.delete_attachment(ref_path, cx);
        } else {
            self.pending_delete = Some(ref_path.to_string());
            cx.notify();
        }
    }

    /// Delete an attachment through the provider and reload the panel.
    pub fn delete_attachment(&mut self, ref_path: &str, cx: &mut Context<Self>) {
        self.pending_delete = None;
        self.attachment_error = None;
        match self.provider.delete_asset(ref_path) {
            Ok(()) => {
                self.refresh_attachments();
                self.rebuild_rows();
            }
            Err(error) => self.attachment_error = Some(error),
        }
        cx.notify();
    }

    /// Mark the start of an import and clear the previous error.
    fn begin_import(&mut self, cx: &mut Context<Self>) {
        self.attachment_busy = true;
        self.attachment_error = None;
        self.pending_delete = None;
        cx.notify();
    }

    /// Apply a finished import to the document and the panel.
    ///
    /// The caret is moved to the drop target before the references are
    /// inserted, then the document is marked dirty and the normal debounced
    /// save is scheduled. Imported rows are listed even when the reference
    /// could not be inserted (for example an empty document without a caret).
    fn finish_import(
        &mut self,
        result: Result<Vec<AttachmentRow>, String>,
        block_id: Option<Uuid>,
        cx: &mut Context<Self>,
    ) {
        self.attachment_busy = false;
        match result {
            Ok(rows) => {
                if let Some(block_id) = block_id {
                    let end = self
                        .state
                        .document()
                        .block(block_id)
                        .map(|block| block.content.len());
                    if let Some(end) = end {
                        let _ = self.state.set_caret(block_id, end);
                    }
                }
                for row in &rows {
                    let _ = self.state.insert_asset_reference(&row.ref_path);
                }
                self.rebuild_rows();
                self.save.mark_dirty();
                self.schedule_save(cx);
                self.refresh_attachments();
            }
            Err(error) => self.attachment_error = Some(error),
        }
        cx.notify();
    }
}
