//! Save-status model for the native editor.
//!
//! [`SaveState`] tracks the persistence lifecycle independently of gpui so the
//! contract tests can exercise every transition without opening a window. The
//! view schedules a debounced save after each edit and reports the state through
//! the save chip; the label keys are `editor.saving` and `settings.saved`.
//!
//! A failed write keeps the document dirty so the next edit (or an explicit
//! retry) can try again.

use chrono::{DateTime, Utc};

/// The persistence lifecycle of the open document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveStatus {
    /// No unsaved edits.
    Clean,
    /// Edits exist but no save is running yet.
    Dirty,
    /// A save is in progress.
    Saving,
    /// The last save completed successfully.
    Saved,
    /// The last save failed; the document stays dirty.
    Failed(String),
}

/// Save bookkeeping for one editor session.
#[derive(Debug, Clone)]
pub struct SaveState {
    status: SaveStatus,
    last_saved: Option<DateTime<Utc>>,
}

impl Default for SaveState {
    fn default() -> Self {
        Self::new()
    }
}

impl SaveState {
    /// Create a clean save state.
    pub fn new() -> Self {
        Self {
            status: SaveStatus::Clean,
            last_saved: None,
        }
    }

    /// The current status.
    pub fn status(&self) -> &SaveStatus {
        &self.status
    }

    /// The timestamp of the last successful save.
    pub fn last_saved(&self) -> Option<DateTime<Utc>> {
        self.last_saved
    }

    /// Record an edit that needs saving.
    pub fn mark_dirty(&mut self) {
        if !matches!(self.status, SaveStatus::Saving) {
            self.status = SaveStatus::Dirty;
        }
    }

    /// Mark the beginning of a write.
    pub fn begin_save(&mut self) {
        self.status = SaveStatus::Saving;
    }

    /// Record a successful write.
    pub fn finish_save(&mut self, now: DateTime<Utc>) {
        self.status = SaveStatus::Saved;
        self.last_saved = Some(now);
    }

    /// Record a failed write; the document remains dirty.
    pub fn fail(&mut self, message: impl Into<String>) {
        self.status = SaveStatus::Failed(message.into());
    }

    /// Whether the document needs a write.
    pub fn needs_save(&self) -> bool {
        matches!(self.status, SaveStatus::Dirty | SaveStatus::Failed(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    #[test]
    fn clean_state_does_not_need_saving() {
        let state = SaveState::new();
        assert_eq!(state.status(), &SaveStatus::Clean);
        assert!(!state.needs_save());
        assert_eq!(state.last_saved(), None);
    }

    #[test]
    fn failed_write_stays_dirty() {
        let mut state = SaveState::new();
        state.mark_dirty();
        state.begin_save();
        state.fail("permission denied");
        assert_eq!(
            state.status(),
            &SaveStatus::Failed("permission denied".into())
        );
        assert!(state.needs_save());
    }

    #[test]
    fn successful_write_records_a_timestamp() {
        let mut state = SaveState::new();
        state.mark_dirty();
        state.begin_save();
        let now = Utc::now();
        state.finish_save(now);
        assert_eq!(state.status(), &SaveStatus::Saved);
        assert_eq!(state.last_saved(), Some(now));
        assert!(!state.needs_save());
    }

    #[test]
    fn dirty_edits_do_not_interrupt_an_active_save() {
        let mut state = SaveState::new();
        state.begin_save();
        state.mark_dirty();
        assert_eq!(state.status(), &SaveStatus::Saving);
    }
}
