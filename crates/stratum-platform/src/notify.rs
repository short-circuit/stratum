//! Desktop notifications for block reminders.
//!
//! The shipped app showed a system notification per due reminder and, when the
//! notification was activated, focused the window and revealed the block. This
//! module keeps that contract: the pure half builds the notification text and
//! the stable notification id, the OS half posts it through `notify-rust`
//! (D-Bus on Linux, WinRT toasts on Windows, `NSUserNotification` on macOS) and
//! reports an activation back to the shell as [`crate::PlatformEvent::ReminderOpen`].
//!
//! Text parity with the shipped `setup.rs`: the body is the block content with
//! newlines collapsed, prefixed by the page title when known, and capped at 200
//! characters.

#[cfg(desktop)]
use std::sync::mpsc::Sender;

use pkm_core::PkmResult;

#[cfg(desktop)]
use crate::PlatformEvent;

/// The maximum number of characters in a notification body.
pub const MAX_BODY_CHARS: usize = 200;

/// Map a block id to the stable 32-bit notification id.
///
/// Byte-identical to the shipped `reminder_hash_to_i32` so a reminder replaces
/// its own notification instead of stacking duplicates.
pub fn hash_block_id(block_id: &str) -> i32 {
    let mut hash: i32 = 0;
    for byte in block_id.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(i32::from(byte));
    }
    hash
}

/// One reminder notification, ready for the OS layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReminderNotice {
    /// The stable notification id (see [`hash_block_id`]).
    pub id: i32,
    /// The localized notification title.
    pub title: String,
    /// The localized body text.
    pub body: String,
    /// Whether the OS notification sound plays.
    pub sound: bool,
    /// Vault-relative page path of the reminder.
    pub page_path: String,
    /// Block id of the reminder.
    pub block_id: String,
}

/// Build a reminder notification.
///
/// `page_title` is the human title of the reminder's page; it is prefixed to the
/// body when known, exactly like the shipped scheduler did.
pub fn reminder_notice(
    title: &str,
    content: &str,
    page_title: Option<&str>,
    page_path: &str,
    block_id: &str,
    sound: bool,
) -> ReminderNotice {
    let mut body = content.replace('\n', " ");
    if let Some(page_title) = page_title {
        if !page_title.is_empty() {
            body = format!("{page_title} — {body}");
        }
    }
    let body: String = body.chars().take(MAX_BODY_CHARS).collect();
    ReminderNotice {
        id: hash_block_id(block_id),
        title: title.to_string(),
        body,
        sound,
        page_path: page_path.to_string(),
        block_id: block_id.to_string(),
    }
}

/// The OS sound name for the current platform.
pub fn sound_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "ms-winsoundevent:Notification.Default"
    } else if cfg!(target_os = "macos") {
        "default"
    } else {
        // freedesktop.org notification sound (GNOME / KDE).
        "message-new-instant"
    }
}

/// A notification sink.
pub trait Notifier: Send + Sync {
    /// Post `notice` and arrange for its activation to reach the shell.
    fn notify(&self, notice: &ReminderNotice) -> PkmResult<()>;
}

/// A sink that drops every notification (headless runs, mobile, tests).
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopNotifier;

impl Notifier for NoopNotifier {
    fn notify(&self, _notice: &ReminderNotice) -> PkmResult<()> {
        Ok(())
    }
}

/// The system notification sink.
#[cfg(desktop)]
pub struct SystemNotifier {
    events: Sender<PlatformEvent>,
    open_label: String,
}

#[cfg(desktop)]
impl SystemNotifier {
    /// Create a sink that reports activations on `events`.
    ///
    /// `open_label` is the localized action label shown on the notification
    /// (the shipped app used a single "tap" action that opened the reminder).
    pub fn new(events: Sender<PlatformEvent>, open_label: String) -> Self {
        Self { events, open_label }
    }
}

#[cfg(desktop)]
impl Notifier for SystemNotifier {
    fn notify(&self, notice: &ReminderNotice) -> PkmResult<()> {
        use notify_rust::Notification;

        let mut builder = Notification::new();
        builder
            .summary(&notice.title)
            .body(&notice.body)
            .id(notice.id as u32)
            .appname("Stratum")
            // The `default` action fires when the notification body is clicked,
            // which is the shipped "tap to open the reminder" behaviour.
            .action("default", &self.open_label);
        if notice.sound {
            builder.sound_name(sound_name());
        }

        let handle = builder
            .show()
            .map_err(|e| pkm_core::PkmError::Internal(format!("notification: {e}")))?;

        // Activation has to be awaited on a thread of its own: the handle blocks
        // until the notification is dismissed or activated.
        let events = self.events.clone();
        let page_path = notice.page_path.clone();
        let block_id = notice.block_id.clone();
        std::thread::spawn(move || {
            handle.wait_for_action(|action| {
                if action == "default" {
                    let _ = events.send(PlatformEvent::ReminderOpen {
                        page_path: page_path.clone(),
                        block_id: block_id.clone(),
                    });
                }
            });
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_and_matches_the_shipped_formula() {
        let id = "0f8fad5b-d9cb-469f-a165-70867728950e";
        let mut expected: i32 = 0;
        for byte in id.bytes() {
            expected = expected.wrapping_mul(31).wrapping_add(i32::from(byte));
        }
        assert_eq!(hash_block_id(id), expected);
        assert_eq!(hash_block_id(id), hash_block_id(id));
        assert_ne!(hash_block_id("a"), hash_block_id("b"));
    }

    #[test]
    fn body_collapses_newlines_and_prefixes_the_page_title() {
        let notice = reminder_notice(
            "Reminder due",
            "first line\nsecond line",
            Some("Project plan"),
            "pages/project.md",
            "block-1",
            true,
        );
        assert_eq!(notice.body, "Project plan — first line second line");
        assert_eq!(notice.title, "Reminder due");
        assert_eq!(notice.page_path, "pages/project.md");
        assert_eq!(notice.block_id, "block-1");
        assert!(notice.sound);
    }

    #[test]
    fn body_without_a_page_title_is_the_content_alone() {
        let notice = reminder_notice("Reminder due", "call the dentist", None, "p.md", "b", false);
        assert_eq!(notice.body, "call the dentist");
        assert!(!notice.sound);
    }

    #[test]
    fn body_is_capped_at_the_shipped_limit() {
        let long = "x".repeat(400);
        let notice = reminder_notice("Reminder due", &long, None, "p.md", "b", false);
        assert_eq!(notice.body.chars().count(), MAX_BODY_CHARS);
    }

    #[test]
    fn empty_page_title_is_ignored() {
        let notice = reminder_notice("t", "content", Some(""), "p.md", "b", false);
        assert_eq!(notice.body, "content");
    }
}
