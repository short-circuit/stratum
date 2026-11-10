//! Journal view state accessors and mode switching.
//!
//! These methods are the read-only surface the shell, header and contract tests
//! use, plus the read-mode switch shared by today and every loaded past day.

use super::*;

impl JournalView {
    /// Today's `YYYY-MM-DD` date.
    pub fn today(&self) -> &str {
        &self.today
    }

    /// The date currently selected in the header/calendar.
    pub fn target_date(&self) -> &str {
        &self.target_date
    }

    /// Whether the calendar popup is open.
    pub fn calendar_open(&self) -> bool {
        self.calendar_open
    }

    /// Whether the journal is in read mode.
    pub fn read_mode(&self) -> bool {
        self.read_mode
    }

    /// The current error message, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Whether an ensure/repair request is running.
    pub fn loading(&self) -> bool {
        self.loading
    }

    /// The voice-capture entity.
    pub fn voice(&self) -> &Entity<VoiceCapture> {
        &self.voice
    }

    /// The live today editor entity, if loaded.
    pub fn today_editor(&self) -> Option<&Entity<EditorView>> {
        self.today_editor.as_ref()
    }

    /// The live editor for a stacked past section, if loaded.
    pub fn section_editor(&self, index: usize) -> Option<&Entity<EditorView>> {
        self.sections
            .get(index)
            .and_then(|section| section.editor.as_ref())
    }

    /// The `YYYY-MM-DD` date for a stacked past section.
    pub fn section_date(&self, index: usize) -> Option<&str> {
        self.sections
            .get(index)
            .map(|section| section.date.as_str())
    }

    /// The number of stacked past sections.
    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    /// The current stack plan, for contract tests and diagnostics.
    pub fn plan(&self) -> &StackPlan {
        &self.plan
    }

    /// The current journal dates, newest first.
    pub fn journal_dates(&self) -> &[String] {
        &self.journal_dates
    }

    /// A pending navigation target produced by a child editor.
    pub fn pending_navigation(&self) -> Option<&str> {
        self.pending_navigation.as_deref()
    }

    /// Clear the pending navigation after the shell has consumed it.
    pub fn clear_pending_navigation(&mut self) {
        self.pending_navigation = None;
    }

    /// Switch between edit and read mode for today and every loaded past day.
    pub fn set_read_mode(&mut self, read_mode: bool, cx: &mut Context<Self>) {
        if self.read_mode == read_mode {
            return;
        }
        self.read_mode = read_mode;
        if let Some(editor) = &self.today_editor {
            editor.update(cx, |view, cx| view.set_read_mode(read_mode, cx));
        }
        for section in &self.sections {
            if let Some(editor) = &section.editor {
                editor.update(cx, |view, cx| view.set_read_mode(read_mode, cx));
            }
        }
        cx.notify();
    }

    /// The calendar's view month/year.
    pub fn view_month(&self) -> (i32, u32) {
        (self.view_year, self.view_month)
    }

    /// Scroll offset of the journal container in content pixels.
    pub(super) fn scroll_offset(&self) -> f32 {
        self.scroll.offset().y / px(1.0)
    }
}
