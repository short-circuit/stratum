//! Reminder toaster: due-reminder polling, announce-once bookkeeping and the
//! toast action that opens a reminder's page.
//!
//! [`super::chrome`] renders the toast; this module owns the toaster state, the
//! localized message/action keys and the 30 s poll gate so [`super`] stays under
//! the repository file-length limit. The navigation the toast action triggers
//! stays in [`super::page`] (`open_page_with_highlight`), which this module only
//! calls.
//!
//! The toaster is manual-dismiss only. The React shell auto-dismisses success
//! messages after a timeout; wiring a GPUI timer is possible but the task
//! allows the simpler behaviour, and the dismiss button keeps the toast from
//! blocking the window.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use gpui_kit::prelude::*;
use gpui_kit::*;
use stratum_i18n::t_global;

use crate::provider::ReminderRow;

use super::Shell;

/// Shell-side reminder toaster state.
///
/// `toast` is the currently rendered message, `toasted` remembers the block ids
/// already announced, `last_poll` gates the 30 s poll and `watch_started` keeps
/// the background timer single-spawned.
#[derive(Default)]
pub(super) struct ReminderToaster {
    /// The current toast, rendered by the shell chrome.
    pub(super) toast: Option<Toast>,
    /// Block ids already surfaced in a reminder toast, so a due reminder is
    /// announced once. Snoozing a reminder drops it from the due set and the
    /// id is pruned, so it can be announced again when it becomes due.
    toasted: HashSet<String>,
    /// Last provider poll for due reminders; gates the 30 s render check.
    last_poll: Option<Instant>,
    /// Whether the 30 s background reminder poll has been spawned.
    watch_started: bool,
}

/// A toaster action that navigates into the shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToastAction {
    /// Open a page and reveal a block (the reminder "open" action).
    ReminderOpen {
        /// Vault-relative page path.
        page_path: String,
        /// Block id to reveal.
        block_id: String,
    },
}

/// A transient message shown in the toaster.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast {
    /// Localized message body.
    pub message: String,
    /// Optional action label (for example `reminders.toast.open`).
    pub action_label: Option<String>,
    /// Optional action performed when the label is clicked.
    pub action: Option<ToastAction>,
}

impl Toast {
    /// Build a message-only toast.
    pub fn message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            action_label: None,
            action: None,
        }
    }

    /// Build a toast with an action chip.
    pub fn with_action(
        message: impl Into<String>,
        action_label: impl Into<String>,
        action: ToastAction,
    ) -> Self {
        Self {
            message: message.into(),
            action_label: Some(action_label.into()),
            action: Some(action),
        }
    }
}

impl Shell {
    /// The current toast message, if any.
    pub fn toast_message(&self) -> Option<&str> {
        self.toaster
            .toast
            .as_ref()
            .map(|toast| toast.message.as_str())
    }

    /// The current toast action label, if any.
    pub fn toast_action_label(&self) -> Option<&str> {
        self.toaster
            .toast
            .as_ref()
            .and_then(|toast| toast.action_label.as_deref())
    }

    /// The current toast action, if any.
    pub fn toast_action(&self) -> Option<&ToastAction> {
        self.toaster
            .toast
            .as_ref()
            .and_then(|toast| toast.action.as_ref())
    }

    /// Push a success toast.
    pub fn push_toast(&mut self, message: impl Into<String>) {
        self.toaster.toast = Some(Toast::message(message));
    }

    /// Run the pending toast action and dismiss the toast.
    ///
    /// Returns `true` when an action was present and consumed. The reminder
    /// "open" action routes to the page panel and reveals the block, matching
    /// the shipped `ReminderToaster`.
    pub fn activate_toast_action(&mut self) -> bool {
        let Some(action) = self
            .toaster
            .toast
            .as_ref()
            .and_then(|toast| toast.action.clone())
        else {
            return false;
        };
        self.toaster.toast = None;
        match action {
            ToastAction::ReminderOpen {
                page_path,
                block_id,
            } => self.open_page_with_highlight(page_path, block_id),
        }
        true
    }

    /// Poll the provider for due reminders and announce a fresh one.
    ///
    /// A reminder is announced once per due window: the block id is remembered
    /// while it stays due, and pruned once it is snoozed or cleared so the next
    /// due window toasts again. The message mirrors the shipped toaster
    /// (`reminders.toast.due_title` — block content) with the localized
    /// `reminders.toast.open` action.
    pub fn poll_reminders(&mut self, cx: &mut Context<Self>) {
        let due = self.provider.due_reminders();
        if let Some(toast) = self.next_due_toast(&due) {
            self.toaster.toast = Some(toast);
            cx.notify();
        }
    }

    /// Select the next reminder to announce and remember its block id.
    ///
    /// Split out of [`Self::poll_reminders`] so the announce-once/prune rule is
    /// testable without a GPUI context.
    pub(super) fn next_due_toast(&mut self, due: &[ReminderRow]) -> Option<Toast> {
        let due_ids: HashSet<&str> = due.iter().map(|row| row.block_id.as_str()).collect();
        self.toaster
            .toasted
            .retain(|block_id| due_ids.contains(block_id.as_str()));
        let reminder = due
            .iter()
            .find(|row| !self.toaster.toasted.contains(&row.block_id))?;
        self.toaster.toasted.insert(reminder.block_id.clone());
        let title = t_global("reminders.toast.due_title");
        Some(Toast::with_action(
            format!("{title} — {}", reminder.content),
            t_global("reminders.toast.open"),
            ToastAction::ReminderOpen {
                page_path: reminder.page_path.clone(),
                block_id: reminder.block_id.clone(),
            },
        ))
    }

    /// Run a due-reminder poll at most once every 30 seconds.
    pub(super) fn maybe_poll_reminders(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        let ready = self
            .toaster
            .last_poll
            .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(30));
        if !ready {
            return;
        }
        self.toaster.last_poll = Some(now);
        self.poll_reminders(cx);
    }

    /// Spawn the background reminder poll once.
    ///
    /// Render is only driven by notifications, so a timer wakes the shell every
    /// 30 s to re-check the provider; the render gate above keeps both paths
    /// from polling faster than that.
    pub(super) fn ensure_reminder_watch(&mut self, cx: &mut Context<Self>) {
        if self.toaster.watch_started {
            return;
        }
        self.toaster.watch_started = true;
        cx.spawn(
            async move |this: WeakEntity<Shell>, cx: &mut AsyncApp| loop {
                cx.background_executor()
                    .timer(Duration::from_secs(30))
                    .await;
                let Ok(()) = this.update(cx, |shell, cx| shell.maybe_poll_reminders(cx)) else {
                    break;
                };
            },
        )
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::Toast;

    #[test]
    fn message_only_toasts_have_no_action() {
        let toast = Toast::message("Saved");
        assert_eq!(toast.action_label, None);
        assert_eq!(toast.action, None);
    }
}
