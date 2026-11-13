//! Native Reminders panel.
//!
//! [`RemindersView`] ports the shipped desktop `RemindersPanel`: the overdue /
//! today / upcoming sections, the snooze/complete/open row actions, the due
//! dialog with quick choices and the shell block-open navigation. Provider
//! calls stay behind the [`crate::provider::PanelData`] seam; grouping and the
//! dialog state live in [`model`] and are unit-tested without a window.
//!
//! [`REMINDERS_I18N_KEYS`] is the release gate list for every string this
//! surface renders. Provider error text is passed through as data, matching the
//! shipped panel's untranslated alert body.

use std::sync::Arc;

use chrono::Local;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::panels::PanelRequests;
use crate::provider::{ReminderRow, ShellData};

pub mod dialog;
pub mod model;
pub mod view;

use model::{DueDialog, QuickDue};

/// Every translation key the Reminders surface renders.
///
/// The contract test walks this list against all eight catalogs, so adding a
/// visible string without a catalog entry fails the build instead of leaking a
/// raw key into the UI.
pub const REMINDERS_I18N_KEYS: &[&str] = &[
    "reminders.chip.title",
    "reminders.dialog.cancel",
    "reminders.dialog.change_reminder",
    "reminders.dialog.clear_reminder",
    "reminders.dialog.evening_1800",
    "reminders.dialog.in_1_hour",
    "reminders.dialog.save",
    "reminders.dialog.set_reminder",
    "reminders.dialog.tomorrow_0900",
    "reminders.humanize.due_date",
    "reminders.humanize.due_in_hours",
    "reminders.humanize.due_in_min",
    "reminders.humanize.now",
    "reminders.humanize.overdue_hours",
    "reminders.humanize.overdue_min",
    "reminders.humanize.overdue_now",
    "reminders.panel.complete",
    "reminders.panel.due",
    "reminders.panel.empty",
    "reminders.panel.open",
    "reminders.panel.overdue",
    "reminders.panel.snooze_10min",
    "reminders.panel.snooze_1h",
    "reminders.panel.title",
    "reminders.panel.today",
    "reminders.panel.upcoming",
];

/// Deterministic start state for the reminders panel.
///
/// `STRATUM_REMINDER_DIALOG=<block id>` opens the due dialog for that reminder
/// after construction so the capture harness can render it without a click.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemindersStart {
    /// Block id whose due dialog should open after construction.
    pub dialog_block: Option<String>,
}

/// The reminders panel entity.
pub struct RemindersView {
    provider: Arc<dyn ShellData>,
    rows: Vec<ReminderRow>,
    now: chrono::DateTime<Local>,
    load_error: Option<String>,
    dialog: Option<DueDialog>,
    dialog_input: Option<Entity<InputState>>,
    _dialog_subscription: Option<Subscription>,
    pending_open_block: Option<(String, String)>,
    pending_navigation: Option<String>,
    pending_toast: Option<String>,
}

impl RemindersView {
    /// Build the panel, load the reminder list and apply the start state.
    pub fn new(
        provider: Arc<dyn ShellData>,
        start: RemindersStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let now = Local::now();
        let (rows, load_error) = match provider.reminders() {
            Ok(rows) => (rows, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        let mut view = Self {
            provider,
            rows,
            now,
            load_error,
            dialog: None,
            dialog_input: None,
            _dialog_subscription: None,
            pending_open_block: None,
            pending_navigation: None,
            pending_toast: None,
        };
        if let Some(block_id) = start.dialog_block {
            if let Some(row) = view
                .rows
                .iter()
                .find(|row| row.block_id == block_id)
                .cloned()
            {
                view.open_dialog_for(&row, window, cx);
            }
        }
        view
    }

    /// The loaded reminder rows.
    pub fn rows(&self) -> &[ReminderRow] {
        &self.rows
    }

    /// The local clock used to bucket and humanize the rows.
    pub fn now(&self) -> chrono::DateTime<Local> {
        self.now
    }

    /// The reminder groups derived from the loaded rows.
    pub fn groups(&self) -> model::ReminderGroups {
        model::group(&self.rows, self.now)
    }

    /// The reminder-list load error, if any.
    pub fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    /// The open due dialog, if any.
    pub fn dialog(&self) -> Option<&DueDialog> {
        self.dialog.as_ref()
    }

    /// Whether the due dialog is open.
    pub fn dialog_open(&self) -> bool {
        self.dialog.is_some()
    }

    /// The due dialog input entity, if the dialog is open.
    pub fn dialog_input(&self) -> Option<&Entity<InputState>> {
        self.dialog_input.as_ref()
    }

    /// Reload the reminder list.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.now = Local::now();
        match self.provider.reminders() {
            Ok(rows) => {
                self.rows = rows;
                self.load_error = None;
            }
            Err(error) => {
                self.rows.clear();
                self.load_error = Some(error);
            }
        }
        cx.notify();
    }

    /// Snooze the reminder at `index` by `minutes`, then reload.
    pub fn snooze(&mut self, index: usize, minutes: u32, cx: &mut Context<Self>) {
        let Some(row) = self.rows.get(index).cloned() else {
            return;
        };
        match self
            .provider
            .snooze_reminder(&row.page_path, &row.block_id, minutes)
        {
            Ok(()) => {
                self.load_error = None;
                self.refresh(cx);
            }
            Err(error) => {
                self.load_error = Some(error);
                cx.notify();
            }
        }
    }

    /// Clear the reminder at `index` (the shipped "Done" action), then reload.
    pub fn complete(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(row) = self.rows.get(index).cloned() else {
            return;
        };
        match self.provider.clear_reminder(&row.page_path, &row.block_id) {
            Ok(()) => {
                self.load_error = None;
                self.refresh(cx);
            }
            Err(error) => {
                self.load_error = Some(error);
                cx.notify();
            }
        }
    }

    /// Request the shell open the reminder's page and reveal the block.
    pub fn open_row(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(row) = self.rows.get(index) else {
            return;
        };
        self.pending_open_block = Some((row.page_path.clone(), row.block_id.clone()));
        cx.notify();
    }

    /// Open the due dialog for the reminder at `index`.
    pub fn open_dialog(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.rows.get(index).cloned() else {
            return;
        };
        self.open_dialog_for(&row, window, cx);
    }

    /// Open the due dialog for a row.
    pub fn open_dialog_for(
        &mut self,
        row: &ReminderRow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dialog = DueDialog::for_row(row);
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(dialog.value.clone())
                .placeholder("YYYY-MM-DDTHH:MM")
        });
        let subscription =
            cx.subscribe(&input, |this: &mut Self, source, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    if let Some(dialog) = &mut this.dialog {
                        dialog.value = source.read(cx).value().to_string();
                        dialog.error = None;
                    }
                    cx.notify();
                }
            });
        self.dialog = Some(dialog);
        self.dialog_input = Some(input);
        self._dialog_subscription = Some(subscription);
        cx.notify();
    }

    /// Replace the dialog input value.
    pub fn dialog_set_value(
        &mut self,
        value: impl Into<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = value.into();
        let Some(dialog) = &mut self.dialog else {
            return;
        };
        dialog.set_value(value.clone());
        if let Some(input) = self.dialog_input.clone() {
            input.update(cx, |input, cx| input.set_value(value, window, cx));
        }
        cx.notify();
    }

    /// Apply a quick due choice to the dialog input.
    pub fn dialog_apply_quick(
        &mut self,
        quick: QuickDue,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let now = self.now;
        let Some(dialog) = &mut self.dialog else {
            return;
        };
        dialog.apply_quick(quick, now);
        let value = dialog.value.clone();
        if let Some(input) = self.dialog_input.clone() {
            input.update(cx, |input, cx| input.set_value(value, window, cx));
        }
        cx.notify();
    }

    /// Save the dialog value through the provider, closing on success.
    pub fn dialog_save(&mut self, cx: &mut Context<Self>) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        let Some(due) = dialog.submit_value() else {
            self.dialog = Some(dialog);
            cx.notify();
            return;
        };
        match self
            .provider
            .set_reminder(&dialog.page_path, &dialog.block_id, Some(due))
        {
            Ok(()) => {
                self.dialog_input = None;
                self.load_error = None;
                self.refresh(cx);
            }
            Err(error) => {
                dialog.error = Some(error);
                self.dialog = Some(dialog);
                cx.notify();
            }
        }
    }

    /// Clear the reminder through the provider, closing on success.
    pub fn dialog_clear(&mut self, cx: &mut Context<Self>) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        match self
            .provider
            .set_reminder(&dialog.page_path, &dialog.block_id, None)
        {
            Ok(()) => {
                self.dialog_input = None;
                self.load_error = None;
                self.refresh(cx);
            }
            Err(error) => {
                dialog.error = Some(error);
                self.dialog = Some(dialog);
                cx.notify();
            }
        }
    }

    /// Dismiss the dialog without saving.
    pub fn dismiss_dialog(&mut self, cx: &mut Context<Self>) {
        self.dialog = None;
        self.dialog_input = None;
        cx.notify();
    }
}

impl PanelRequests for RemindersView {
    fn pending_navigation(&self) -> Option<&str> {
        self.pending_navigation.as_deref()
    }

    fn clear_pending_navigation(&mut self) {
        self.pending_navigation = None;
    }

    fn pending_toast(&self) -> Option<&str> {
        self.pending_toast.as_deref()
    }

    fn take_pending_toast(&mut self) -> Option<String> {
        self.pending_toast.take()
    }

    fn take_pending_open_block(&mut self) -> Option<(String, String)> {
        self.pending_open_block.take()
    }
}

impl Render for RemindersView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        view::render(self, cx)
    }
}

/// Prefix a provider error with the localized panel title.
///
/// The shipped reminder panel renders the raw alert body, but the native panel
/// keeps a label so a failure is never mistaken for a reminder row. The title
/// is an existing catalog key; no reminder-specific error key exists.
pub(crate) fn failure(error: &str) -> String {
    format!("{}: {error}", t_global("reminders.panel.title"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    #[test]
    fn failure_prefixes_the_panel_title() {
        assert_eq!(failure("boom"), "Reminders: boom");
    }

    #[test]
    fn every_reminders_key_is_listed() {
        for key in [
            "reminders.panel.snooze_10min",
            "reminders.panel.snooze_1h",
            "reminders.panel.complete",
            "reminders.panel.open",
        ] {
            assert!(REMINDERS_I18N_KEYS.contains(&key));
        }
    }
}
