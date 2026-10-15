//! Block-reminder scheduler.
//!
//! The core is deliberately Tauri-free so it can be unit-tested with an
//! injected clock and notify callback. It scans the block store for blocks
//! whose `due` timestamp has passed, deduplicates previously-fired reminders
//! in memory, and invokes the notify callback. The Tauri wiring (OS
//! notifications + app event emission) lives in `lib.rs` and is injected as a
//! `NotifyFn` closure.

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use pkm_block::store::BlockStore;
use serde::{Deserialize, Serialize};

/// A reminder that is due, carrying everything the UI needs to act on it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DueReminder {
    pub block_id: String,
    pub page_path: String,
    pub content: String,
    pub due: DateTime<Utc>,
    pub marker: Option<String>,
    pub page_title: Option<String>,
}

/// Injected clock; returns the current UTC time.
pub type NowFn = Box<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// Injected notifier; called once per due reminder.
pub type NotifyFn = Box<dyn Fn(&DueReminder) + Send + Sync>;

/// Periodic scanner that fires reminders whose `due` time has passed.
///
/// Fired-memory is in-memory only: reminders are deduplicated between scans to
/// avoid repeated notifications, but an app restart re-fires any still-overdue
/// reminder, which is the intended behavior.
pub struct ReminderScheduler {
    store: BlockStore,
    now: NowFn,
    notify: NotifyFn,
    interval: Duration,
    /// Dedup key is `(block_id, due_string)`; re-snoozing changes the `due`
    /// string so the same block fires again after a snooze.
    fired: HashMap<(String, String), ()>,
}

impl ReminderScheduler {
    /// Create a new scheduler.
    ///
    /// `interval` is the scan period; `now` and `notify` are injected for
    /// testability.
    pub fn new(store: BlockStore, interval: Duration, now: NowFn, notify: NotifyFn) -> Self {
        Self {
            store,
            now,
            notify,
            interval,
            fired: HashMap::new(),
        }
    }

    /// Scan the store and notify for every due, open reminder not yet fired.
    ///
    /// Returns the number of reminders fired this scan. Blocks whose marker is
    /// `DONE`/`CANCELLED` are skipped, as are reminders whose `due` is still in
    /// the future.
    pub fn scan_and_notify(&mut self) -> usize {
        let now = (self.now)();
        let due_blocks = match self.store.find_due_blocks() {
            Ok(blocks) => blocks,
            Err(_) => return 0,
        };

        let mut fired_count = 0;
        for (block, page_path) in due_blocks {
            // Skip closed tasks (DONE / CANCELLED).
            if block.marker.map(|m| m.is_closed()).unwrap_or(false) {
                continue;
            }
            let Some(due) = block.due() else {
                continue;
            };
            if due > now {
                continue;
            }

            let due_key = due.to_rfc3339();
            let key = (block.id.to_string(), due_key);
            if self.fired.contains_key(&key) {
                continue;
            }

            let page_title = self
                .store
                .get_page(&page_path)
                .ok()
                .flatten()
                .and_then(|fm| fm.title);

            let reminder = DueReminder {
                block_id: block.id.to_string(),
                page_path: page_path.clone(),
                content: block.content,
                due,
                marker: block.marker.map(|m| m.as_str().to_string()),
                page_title,
            };
            (self.notify)(&reminder);
            self.fired.insert(key, ());
            fired_count += 1;
        }
        fired_count
    }

    /// Run the scheduler loop forever, scanning every `interval`.
    pub async fn run(mut self) {
        let mut interval = tokio::time::interval(self.interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            self.scan_and_notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkm_block::Block;
    use pkm_block::TaskMarker;
    use uuid::Uuid;

    fn fixed_now(dt: &str) -> NowFn {
        let t = DateTime::parse_from_rfc3339(dt)
            .unwrap()
            .with_timezone(&Utc);
        Box::new(move || t)
    }

    /// A recording notifier that pushes each fired reminder into a shared vec.
    fn recording_notify(captured: std::sync::Arc<std::sync::Mutex<Vec<DueReminder>>>) -> NotifyFn {
        Box::new(move |r| {
            captured.lock().unwrap().push(r.clone());
        })
    }

    fn scheduler_with(
        store: BlockStore,
        now: NowFn,
        captured: std::sync::Arc<std::sync::Mutex<Vec<DueReminder>>>,
    ) -> ReminderScheduler {
        ReminderScheduler::new(
            store,
            Duration::from_secs(30),
            now,
            recording_notify(captured),
        )
    }

    #[test]
    fn fires_when_due_is_past_but_not_future() {
        let store = BlockStore::open_in_memory().unwrap();
        let past_id = Uuid::new_v4();
        let future_id = Uuid::new_v4();
        store
            .insert_block(
                &Block::new(past_id, "Past".into())
                    .with_property("due", "2026-01-01T00:00:00.000Z"),
                "pages/a.md",
            )
            .unwrap();
        store
            .insert_block(
                &Block::new(future_id, "Future".into())
                    .with_property("due", "2027-01-01T00:00:00.000Z"),
                "pages/b.md",
            )
            .unwrap();

        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut sched = scheduler_with(store, fixed_now("2026-06-01T00:00:00Z"), captured.clone());

        let fired = sched.scan_and_notify();
        assert_eq!(fired, 1);
        let got = captured.lock().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].block_id, past_id.to_string());
    }

    #[test]
    fn skips_done_and_cancelled() {
        let store = BlockStore::open_in_memory().unwrap();
        for (marker, content) in [
            (Some(TaskMarker::Done), "done"),
            (Some(TaskMarker::Cancelled), "cancelled"),
            (Some(TaskMarker::Todo), "todo"),
        ] {
            let mut b = Block::new(Uuid::new_v4(), content.into())
                .with_property("due", "2026-01-01T00:00:00.000Z");
            b.marker = marker;
            store.insert_block(&b, "pages/a.md").unwrap();
        }

        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut sched = scheduler_with(store, fixed_now("2026-06-01T00:00:00Z"), captured.clone());

        let fired = sched.scan_and_notify();
        assert_eq!(fired, 1);
        let got = captured.lock().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].content, "todo");
    }

    #[test]
    fn fires_once_across_two_scans() {
        let store = BlockStore::open_in_memory().unwrap();
        let id = Uuid::new_v4();
        store
            .insert_block(
                &Block::new(id, "One".into()).with_property("due", "2026-01-01T00:00:00.000Z"),
                "pages/a.md",
            )
            .unwrap();

        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut sched = scheduler_with(store, fixed_now("2026-06-01T00:00:00Z"), captured.clone());

        assert_eq!(sched.scan_and_notify(), 1);
        assert_eq!(sched.scan_and_notify(), 0);
        assert_eq!(captured.lock().unwrap().len(), 1);
    }

    #[test]
    fn refires_after_snooze_because_due_key_changes() {
        let store = BlockStore::open_in_memory().unwrap();
        let id = Uuid::new_v4();
        // First due: already past at the "now" of the first scan.
        store
            .insert_block(
                &Block::new(id, "Snooze me".into())
                    .with_property("due", "2026-01-01T00:00:00.000Z"),
                "pages/a.md",
            )
            .unwrap();

        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut sched = scheduler_with(store, fixed_now("2026-06-01T00:00:00Z"), captured.clone());

        assert_eq!(sched.scan_and_notify(), 1);
        // Simulate a snooze: update the block's due to a new timestamp in the past
        // relative to the same (fixed) clock. The dedup key changes, so it re-fires.
        let block = sched.store.get_block(id).unwrap();
        let mut updated = block.clone();
        updated
            .properties
            .insert("due".to_string(), "2026-06-01T00:00:00.000Z".to_string());
        sched.store.update_block(&updated).unwrap();

        assert_eq!(sched.scan_and_notify(), 1);
        assert_eq!(captured.lock().unwrap().len(), 2);
    }
}
