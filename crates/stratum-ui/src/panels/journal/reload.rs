//! Journal reload, repair and stack synchronisation.
//!
//! The stack plan and its editor entities are rebuilt here after an ensure or
//! repair request, after voice insertion, and while the user scrolls. Editors
//! created for a stacked section use the editor auto-height mode so each day
//! contributes its full content height.

use super::*;

impl JournalView {
    /// Retry the ensure-today-journal flow.
    pub fn retry(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        self.loading = true;
        let provider = self.provider.clone();
        cx.spawn(
            async move |this: WeakEntity<JournalView>, cx: &mut AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move { provider.ensure_today_journal() })
                    .await;
                let _ = this.update(cx, |view, cx| {
                    view.loading = false;
                    match result {
                        Ok(path) => view.finish_reload(path, cx),
                        Err(message) => {
                            view.error = Some(format!(
                                "{} {message}",
                                t_global("journal.failed_to_create_journal")
                            ));
                            cx.notify();
                        }
                    }
                });
            },
        )
        .detach();
    }

    /// Repair the database, then ensure today's journal.
    pub fn repair(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        self.loading = true;
        let provider = self.provider.clone();
        cx.spawn(
            async move |this: WeakEntity<JournalView>, cx: &mut AsyncApp| {
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        provider.repair_db()?;
                        provider.ensure_today_journal()
                    })
                    .await;
                let _ = this.update(cx, |view, cx| {
                    view.loading = false;
                    match result {
                        Ok(path) => view.finish_reload(path, cx),
                        Err(message) => {
                            view.error = Some(format!(
                                "{} {message}",
                                t_global("journal.failed_to_create_journal")
                            ));
                            cx.notify();
                        }
                    }
                });
            },
        )
        .detach();
    }

    /// Rebuild the plan and today's editor after an ensure/repair success.
    fn finish_reload(&mut self, _path: String, cx: &mut Context<Self>) {
        self.journal_dates = self.provider.journal_dates();
        self.plan = StackPlan::build(&self.journal_dates, &self.today, DEFAULT_VISIBLE_WINDOW);
        let existing = self.take_section_editors();
        self.rebuild_sections(existing);
        self.reload_today_editor(cx);
        self.sync_stack(0.0, cx);
        cx.notify();
    }

    /// Recreate today's editor so newly inserted blocks become visible.
    pub(super) fn reload_today_editor(&mut self, cx: &mut Context<Self>) {
        let provider = self.provider.clone();
        let path = journal_path(&self.today);
        let read_mode = self.read_mode;
        self.today_editor = Some(cx.new(move |cx| {
            let mut editor = EditorView::new(path, provider, read_mode, cx);
            editor.set_auto_height(true, cx);
            editor
        }));
    }

    /// Load the stacked sections required for `scroll_offset`.
    ///
    /// Sections are created once; placeholders remain until their editor
    /// entity exists. Near the end of the current window the plan grows by the
    /// shipped 25-day page size.
    pub fn sync_stack(&mut self, scroll_offset: f32, cx: &mut Context<Self>) {
        let required = self.plan.load_indices(scroll_offset);
        for index in required {
            if index == 0 {
                continue;
            }
            let Some(section) = self.sections.get(index - 1) else {
                continue;
            };
            if section.editor.is_some() {
                continue;
            }
            let provider = self.provider.clone();
            let path = section.path.clone();
            let read_mode = self.read_mode;
            let editor = cx.new(move |cx| {
                let mut editor = EditorView::new(path, provider, read_mode, cx);
                editor.set_auto_height(true, cx);
                editor
            });
            if let Some(section) = self.sections.get_mut(index - 1) {
                section.editor = Some(editor);
            }
        }
        if scroll_offset + stack::PRELOAD_MARGIN >= self.plan.total_height()
            && self.plan.visible() < self.plan.total()
        {
            self.plan.grow(DEFAULT_VISIBLE_WINDOW);
        }
    }

    /// Scroll the section for `date` into view.
    ///
    /// The plan index resolves the target day; an unknown date falls back to
    /// today, matching the header's default selection.
    pub(super) fn focus_date(&mut self, date: &str, cx: &mut Context<Self>) {
        let index = self.plan.index_of(date).unwrap_or(0);
        self.focus_section(index, cx);
    }

    /// Request a centred scroll for one section.
    ///
    /// Today resets the container to the top. A past day first grows the plan
    /// and materialises its editor, then records the pending scroll so the next
    /// frame that has layout bounds can centre it.
    pub(super) fn focus_section(&mut self, index: usize, cx: &mut Context<Self>) {
        if index == 0 {
            self.scroll.set_offset(point(px(0.0), px(0.0)));
            self.pending_scroll = None;
            return;
        }
        if index >= self.plan.visible() {
            let delta = index + 1 - self.plan.visible();
            self.plan.grow(delta);
        }
        self.sync_stack(self.plan.offset_of(index).unwrap_or(0.0), cx);
        self.pending_scroll = Some(index);
    }

    /// Apply a pending section scroll once layout bounds are available.
    ///
    /// Returns `false` while the target section has no layout bounds yet or the
    /// viewport has not been measured; the caller asks for another frame so the
    /// scroll lands as soon as the section is laid out. gpui clamps the offset
    /// to the content bounds during prepaint, so no manual clamping is needed.
    pub(super) fn apply_pending_scroll(&mut self) -> bool {
        let Some(index) = self.pending_scroll else {
            return true;
        };
        let Some(item) = self.scroll.bounds_for_item(index) else {
            return false;
        };
        let viewport = self.scroll.bounds();
        if viewport.size.height <= px(0.0) {
            return false;
        }
        let viewport_top = viewport.top() / px(1.0);
        let viewport_height = viewport.size.height / px(1.0);
        let item_top = item.top() / px(1.0);
        let item_height = item.size.height / px(1.0);
        let target = viewport_top + (viewport_height - item_height) / 2.0 - item_top;
        self.scroll.set_offset(point(px(0.0), px(target)));
        self.pending_scroll = None;
        true
    }

    /// Copy a child editor's pending navigation into the journal view.
    pub(super) fn sync_child_navigation(&mut self, cx: &mut Context<Self>) {
        if self.pending_navigation.is_some() {
            return;
        }
        let mut editors: Vec<Entity<EditorView>> = self
            .today_editor
            .iter()
            .chain(
                self.sections
                    .iter()
                    .filter_map(|section| section.editor.as_ref()),
            )
            .cloned()
            .collect();
        for editor in editors.drain(..) {
            let target = editor.update(cx, |view, _| {
                view.pending_navigation().map(|target| target.to_string())
            });
            if let Some(target) = target {
                editor.update(cx, |view, _| view.clear_pending_navigation());
                self.pending_navigation = Some(target);
                return;
            }
        }
    }

    /// Take the live editors keyed by path before rebuilding the section list.
    pub(super) fn take_section_editors(&mut self) -> HashMap<String, Entity<EditorView>> {
        self.sections
            .drain(..)
            .filter_map(|section| section.editor.map(|editor| (section.path, editor)))
            .collect()
    }

    /// Build section placeholders for the current plan, reusing live editors.
    pub(super) fn rebuild_sections(&mut self, mut existing: HashMap<String, Entity<EditorView>>) {
        self.sections = self
            .plan
            .sections()
            .iter()
            .skip(1)
            .map(|section| SectionState {
                date: section.date.clone(),
                path: section.path.clone(),
                editor: existing.remove(&section.path),
            })
            .collect();
    }
}
