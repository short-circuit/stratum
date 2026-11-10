//! Native block editor view for the gpui shell.
//!
//! [`EditorView`] is a thin renderer over [`stratum_editor`]: every editing
//! rule, inline-span parse, undo record, IME transition and virtualization
//! calculation lives in the headless engine. The view owns the document state,
//! the math/Mermaid caches, the virtual layout, save status, overlays and the
//! focus handle; it never manipulates block text directly.
//!
//! The module is split so each file stays under the repository limit:
//!
//! - [`view`] — the `Render` implementation and the input-handler canvas.
//! - [`rows`] — row, span and caret elements built from decorated rows.
//! - [`asset_rows`] — attachment thumbnail/file rows.
//! - [`asset_state`] — thumbnail budget, attachment list and drop-target
//!   resolution.
//! - [`asset_actions`] — import/delete/attach flows on the background executor.
//! - [`attachments`] — the attachment panel under the editor page.
//! - [`input`] — `EntityInputHandler`, key handling, clipboard and IME wiring.
//! - [`overlay`] — slash menu, wiki autocomplete, marker menu, save chip,
//!   link preview and dead-link popup.
//! - [`context_menu`] — the right-click block menu (read aloud, clipboard,
//!   marker and reminder actions).
//! - [`drag`] — gutter drag state, drop-target resolution and the indicator bar.
//! - [`save`] — save-status transitions and the debounced persistence helper.
//! - [`metrics`] — per-frame render measurements for the 10k perf capture.
//!
//! All user-facing strings are translation keys resolved through
//! `stratum_i18n`; [`EDITOR_VIEW_I18N_KEYS`] is the release gate list.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use gpui_kit::component::VirtualListScrollHandle;
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_editor::menus::{AiCall, LinkPreview};
use stratum_editor::{
    blocks_to_dto, dto_to_blocks, math_source, mermaid_source, render_page, AssetThumbnailCache,
    AttachmentRow, DecoratedRow, EditorDocument, EditorState, MathCache, MathOptions, MermaidCache,
    RenderMode, VirtualLayout,
};
use stratum_i18n::{t_global, tr_global};

use crate::provider::ShellData;

use self::drag::{DragState, DropTarget};

mod ai;
pub use ai::AiStatus;

pub mod asset_actions;
pub mod asset_rows;
pub mod asset_state;
pub mod attachments;
pub mod context_menu;
pub mod drag;
pub mod input;
mod metrics;
pub mod overlay;
pub mod rows;
pub mod save;
pub mod view;

pub use context_menu::{ContextAction, ContextMenuItem, ContextMenuState, CONTEXT_MENU_ITEMS};
pub use metrics::EditorMetrics;
pub use overlay::{detect_overlay, OverlayState};
pub use save::{SaveState, SaveStatus};

/// Every translation key the editor view renders.
///
/// The contract test walks this list against the shipped catalogs. Slash and
/// marker labels are covered by [`stratum_editor::EDITOR_I18N_KEYS`].
pub const EDITOR_VIEW_I18N_KEYS: &[&str] = &[
    "asset.attach",
    "asset.attachments",
    "asset.compressing",
    "asset.delete",
    "asset.delete_confirm",
    "asset.empty",
    "asset.file_too_large",
    "asset.upload_failed",
    "backlink_snippet_body.ctrl_click_to_navigate",
    "editor.create_page",
    "editor.saving",
    "editor.toggle_marker",
    "page.prose_empty",
    "reminders.chip.clear",
    "reminders.chip.title",
    "settings.saved",
];

/// The native block-editor view for one page.
pub struct EditorView {
    focus: FocusHandle,
    provider: Arc<dyn ShellData>,
    path: String,
    state: EditorState,
    rows: Vec<DecoratedRow>,
    layout: VirtualLayout,
    math: MathCache,
    mermaid: MermaidCache,
    scroll: VirtualListScrollHandle,
    read_mode: bool,
    save: SaveState,
    overlay: OverlayState,
    context_menu: Option<ContextMenuState>,
    drag: Option<DragState>,
    drop_target: Option<DropTarget>,
    metrics: EditorMetrics,
    pending_navigation: Option<String>,
    hover_preview: Option<LinkPreview>,
    ai_status: Option<AiStatus>,
    pending_ai: Option<AiCall>,
    save_generation: u64,
    asset_budget: usize,
    thumbnails: AssetThumbnailCache,
    /// Asset paths whose thumbnail decode failed; skipped until re-decoded.
    failed_thumbnails: HashSet<String>,
    attachments: Vec<AttachmentRow>,
    attachments_open: bool,
    attachment_busy: bool,
    attachment_error: Option<String>,
    pending_delete: Option<String>,
    last_pointer: Point<Pixels>,
    /// When set, the editor sizes itself to its content instead of filling the
    /// parent. Journal sections stack full-height editors inside one scroll
    /// container, so a fill-height editor would collapse its virtual list.
    auto_height: bool,
}

impl EditorView {
    /// Open `path` through `provider` and build the first layout.
    ///
    /// The provider is synchronous; the real provider blocks on its Tokio
    /// runtime, so this runs on the UI thread only for the initial page load.
    /// A missing or unreadable page starts as an empty document.
    pub fn new(
        path: impl Into<String>,
        provider: Arc<dyn ShellData>,
        read_mode: bool,
        cx: &mut Context<Self>,
    ) -> Self {
        let path = path.into();
        let blocks = provider.load_blocks(&path).unwrap_or_default();
        let document = EditorDocument::from_blocks(path.clone(), dto_to_blocks(blocks))
            .unwrap_or_else(|_| {
                EditorDocument::open(path.clone(), "").expect("empty document always opens")
            });
        let mut state = EditorState::new(document);
        if let Some(first) = state.document().ordered_ids().first().copied() {
            let _ = state.set_caret(first, 0);
        }
        let rows = build_rows_for_state(&state, read_mode);
        let layout = VirtualLayout::from_rows(&rows);
        let mut view = Self {
            focus: cx.focus_handle(),
            provider,
            path,
            state,
            rows,
            layout,
            math: MathCache::new(),
            mermaid: MermaidCache::new(),
            scroll: VirtualListScrollHandle::new(),
            read_mode,
            save: SaveState::new(),
            overlay: OverlayState::None,
            context_menu: None,
            drag: None,
            drop_target: None,
            metrics: EditorMetrics::default(),
            pending_navigation: None,
            hover_preview: None,
            ai_status: None,
            pending_ai: None,
            save_generation: 0,
            asset_budget: 1,
            thumbnails: AssetThumbnailCache::default(),
            failed_thumbnails: HashSet::new(),
            attachments: Vec::new(),
            attachments_open: true,
            attachment_busy: false,
            attachment_error: None,
            pending_delete: None,
            last_pointer: point(px(0.0), px(0.0)),
            auto_height: false,
        };
        view.refresh_attachments();
        view
    }

    /// The page path this editor was opened from.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Whether the view is in read mode.
    pub fn read_mode(&self) -> bool {
        self.read_mode
    }

    /// Switch between edit and read mode, rebuilding decorations.
    pub fn set_read_mode(&mut self, read_mode: bool, cx: &mut Context<Self>) {
        if self.read_mode == read_mode {
            return;
        }
        self.read_mode = read_mode;
        self.overlay = OverlayState::None;
        self.context_menu = None;
        self.drag = None;
        self.drop_target = None;
        self.rebuild_rows();
        cx.notify();
    }

    /// The number of rows in the current layout.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// The current virtual layout.
    pub fn layout(&self) -> &VirtualLayout {
        &self.layout
    }

    /// Whether the editor sizes itself to its content.
    pub fn auto_height(&self) -> bool {
        self.auto_height
    }

    /// Size the editor to its content instead of the parent height.
    ///
    /// The journal stack embeds several editors inside one scroll container, so
    /// each editor must contribute its full row height to the stack rather than
    /// filling a viewport it does not own.
    pub fn set_auto_height(&mut self, auto_height: bool, cx: &mut Context<Self>) {
        if self.auto_height == auto_height {
            return;
        }
        self.auto_height = auto_height;
        cx.notify();
    }

    /// The scroll handle used by the virtual list and the perf capture.
    pub fn scroll_handle(&self) -> &VirtualListScrollHandle {
        &self.scroll
    }

    /// The save status.
    pub fn save_state(&self) -> &SaveState {
        &self.save
    }

    /// The current overlay.
    pub fn overlay(&self) -> &OverlayState {
        &self.overlay
    }

    /// Create the page for the open dead-link overlay and request navigation.
    ///
    /// Used by the popup's click handler; Enter goes through the same
    /// [`input::overlay_ops::apply_dead_link`] path.
    pub(crate) fn create_dead_link(&mut self, cx: &mut Context<Self>) {
        if let OverlayState::DeadLink { target } = self.overlay.clone() {
            input::overlay_ops::apply_dead_link(self, &target, cx);
        }
    }

    /// The open block context menu, if any.
    pub fn context_menu(&self) -> Option<ContextMenuState> {
        self.context_menu
    }

    /// The in-flight gutter drag, if any.
    pub fn drag_state(&self) -> Option<DragState> {
        self.drag
    }

    /// The resolved drop target while dragging, if any.
    pub fn drop_target(&self) -> Option<&DropTarget> {
        self.drop_target.as_ref()
    }

    /// Per-frame render measurements.
    pub fn metrics(&self) -> &EditorMetrics {
        &self.metrics
    }

    /// A pending navigation target produced by Ctrl+click on a tag.
    pub fn pending_navigation(&self) -> Option<&str> {
        self.pending_navigation.as_deref()
    }

    /// Clear the pending navigation after the shell has consumed it.
    pub fn clear_pending_navigation(&mut self) {
        self.pending_navigation = None;
    }

    /// The document revision, used by the shell to detect refreshes.
    pub fn revision(&self) -> u64 {
        self.state.document().revision()
    }

    /// The decorated rows, for contract tests and diagnostics.
    pub fn rows(&self) -> &[DecoratedRow] {
        &self.rows
    }

    /// The current caret, if any.
    pub fn caret(&self) -> Option<stratum_editor::Caret> {
        self.state.caret()
    }

    /// Set the scroll offset in content pixels.
    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.scroll.set_offset(point(px(0.0), px(-offset.max(0.0))));
    }

    /// Total estimated content height.
    pub fn total_height(&self) -> f32 {
        self.layout.total_height()
    }

    /// Rebuild decorated rows from the current document and mode.
    pub(crate) fn rebuild_rows(&mut self) {
        self.rows = build_rows_for_state(&self.state, self.read_mode);
        self.layout = VirtualLayout::from_rows(&self.rows);
    }

    /// Lazily render math/Mermaid assets for the visible range.
    ///
    /// Rendering is limited to [`Self::asset_budget`] new assets per frame so a
    /// window full of diagrams cannot blow the 55 fps work budget. Failures
    /// leave the row's asset `None` and the decorated text spans stay visible.
    pub(crate) fn ensure_assets_for_range(&mut self, range: std::ops::Range<usize>) {
        for index in range {
            if self.asset_budget == 0 {
                break;
            }
            let Some(row) = self.rows.get(index) else {
                continue;
            };
            let Some(block) = self.state.document().block(row.block_id) else {
                continue;
            };
            let content = block.content.clone();
            let need_math = row.math.is_none();
            let need_mermaid = row.mermaid.is_none();
            let math = need_math.then(|| math_source(&content)).flatten();
            let mermaid = if need_mermaid {
                mermaid_source(&content).map(str::to_string)
            } else {
                None
            };
            if let Some(source) = math {
                if let Ok(image) = self.math.render(&source, &MathOptions::default()) {
                    if let Some(row) = self.rows.get_mut(index) {
                        row.math = Some(image);
                    }
                    self.asset_budget = self.asset_budget.saturating_sub(1);
                    continue;
                }
            }
            if let Some(source) = mermaid {
                if let Ok(svg) = self.mermaid.render(&source, "modern") {
                    if let Some(row) = self.rows.get_mut(index) {
                        row.mermaid = Some(svg);
                    }
                    self.asset_budget = self.asset_budget.saturating_sub(1);
                }
            }
        }
    }

    /// Mark the document dirty and schedule the debounced save.
    pub(crate) fn after_edit(&mut self, cx: &mut Context<Self>) {
        self.rebuild_rows();
        self.save.mark_dirty();
        self.schedule_save(cx);
        cx.notify();
    }

    /// Schedule a save after the debounce window.
    ///
    /// A generation counter makes only the newest scheduled save run, so a
    /// burst of keystrokes produces one write.
    pub(crate) fn schedule_save(&mut self, cx: &mut Context<Self>) {
        self.save_generation = self.save_generation.wrapping_add(1);
        let generation = self.save_generation;
        cx.spawn(
            async move |this: WeakEntity<EditorView>, cx: &mut AsyncApp| {
                cx.background_executor()
                    .timer(Duration::from_millis(400))
                    .await;
                let _ = this.update(cx, |view, cx| {
                    if view.save_generation == generation {
                        view.save.begin_save();
                        view.persist_now(cx);
                    }
                });
            },
        )
        .detach();
    }

    /// Persist the current document through the provider.
    pub(crate) fn persist_now(&mut self, cx: &mut Context<Self>) {
        let owned: Vec<_> = self
            .state
            .document()
            .blocks()
            .into_iter()
            .cloned()
            .collect();
        let dtos = blocks_to_dto(&owned);
        match self.provider.save_blocks(&self.path, dtos) {
            Ok(()) => {
                self.state.mark_clean();
                self.save.finish_save(Utc::now());
            }
            Err(error) => self.save.fail(error),
        }
        cx.notify();
    }

    /// The save-chip label.
    ///
    /// A running or failed AI request takes precedence over the save status so
    /// the chip reports the long-running service call instead of a stale save
    /// label.
    pub(crate) fn save_label(&self) -> String {
        if let Some(label) = self.ai_label() {
            return label;
        }
        match self.save.status() {
            SaveStatus::Failed(message) => message.clone(),
            SaveStatus::Saved => t_global("settings.saved"),
            _ => t_global("editor.saving"),
        }
    }

    /// The reminder-chip label for a row.
    pub(crate) fn reminder_label(chip: &stratum_editor::ReminderChip) -> String {
        let vars: Vec<(&str, &str)> = chip
            .vars
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        tr_global(chip.label_key, &vars)
    }
}

impl Focusable for EditorView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// Build decorated rows for a document and mode.
///
/// Exposed for the window-free contract tests; the view calls the same helper.
pub fn build_rows_for_state(state: &EditorState, read_mode: bool) -> Vec<DecoratedRow> {
    let focus = if read_mode { None } else { state.caret() };
    let mode = if !read_mode && focus.is_some() {
        RenderMode::FocusedLine
    } else {
        RenderMode::StageA
    };
    render_page(
        state.document(),
        focus.map(|caret| caret.block_id),
        mode,
        Utc::now(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;
    use stratum_i18n::{t, Locale};

    #[test]
    fn view_keys_are_present_in_every_catalog() {
        for locale in Locale::ALL {
            for key in EDITOR_VIEW_I18N_KEYS {
                assert_ne!(t(locale, key), *key, "{} missing in {}", key, locale.code());
            }
        }
    }

    #[test]
    fn save_state_transitions_are_monotonic() {
        let mut state = SaveState::new();
        assert_eq!(state.status(), &SaveStatus::Clean);
        state.mark_dirty();
        assert_eq!(state.status(), &SaveStatus::Dirty);
        state.begin_save();
        assert_eq!(state.status(), &SaveStatus::Saving);
        state.finish_save(Utc::now());
        assert_eq!(state.status(), &SaveStatus::Saved);
        state.fail("disk full");
        assert_eq!(state.status(), &SaveStatus::Failed("disk full".into()));
    }
}
