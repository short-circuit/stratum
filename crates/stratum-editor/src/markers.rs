//! Marker detection, page badge rows and reminder chips.
//!
//! Ports the shipped React helpers:
//! - `src/components/OutlinerEditor/markerDetection.ts` and `markerOptions.ts`
//!   for journal marker detection and menu options.
//! - `src/lib/reminders/reminderUtils.ts` (`humanizeDue`) for due-chip labels.
//!
//! All user-facing text is carried as translation keys plus interpolation
//! variables; the crate never stores English literals.

use chrono::{DateTime, Local, Utc};
use pkm_block::ops::{remove_property, set_property};
use pkm_block::{BlockId, BlockTree, Priority, TaskMarker};
use pkm_core::PkmResult;

use crate::document::EditorDocument;

/// Marker keywords in the shipped order.
pub const MARKER_KEYWORDS: [&str; 7] = [
    "TODO",
    "DOING",
    "DONE",
    "NOW",
    "LATER",
    "WAITING",
    "CANCELLED",
];

/// A marker offered by the editor marker menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkerOption {
    /// Shipped keyword shown as the menu title.
    pub keyword: &'static str,
    /// Structured marker value.
    pub marker: TaskMarker,
    /// Translation key for the menu subtitle.
    pub subtitle_key: &'static str,
    /// Translation key for the menu group.
    pub group_key: &'static str,
}

/// Marker values offered by every editor action menu, in display order.
pub const MARKER_OPTIONS: [MarkerOption; 7] = [
    MarkerOption {
        keyword: "TODO",
        marker: TaskMarker::Todo,
        subtitle_key: "editor.task_marker",
        group_key: "editor.markers",
    },
    MarkerOption {
        keyword: "DOING",
        marker: TaskMarker::Doing,
        subtitle_key: "editor.task_marker",
        group_key: "editor.markers",
    },
    MarkerOption {
        keyword: "DONE",
        marker: TaskMarker::Done,
        subtitle_key: "editor.task_marker",
        group_key: "editor.markers",
    },
    MarkerOption {
        keyword: "NOW",
        marker: TaskMarker::Now,
        subtitle_key: "editor.task_marker",
        group_key: "editor.markers",
    },
    MarkerOption {
        keyword: "LATER",
        marker: TaskMarker::Later,
        subtitle_key: "editor.task_marker",
        group_key: "editor.markers",
    },
    MarkerOption {
        keyword: "WAITING",
        marker: TaskMarker::Waiting,
        subtitle_key: "editor.task_marker",
        group_key: "editor.markers",
    },
    MarkerOption {
        keyword: "CANCELLED",
        marker: TaskMarker::Cancelled,
        subtitle_key: "editor.task_marker",
        group_key: "editor.markers",
    },
];

/// A priority offered by the editor marker menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriorityOption {
    /// Structured priority value.
    pub priority: Priority,
    /// Translation key with a `{{priority}}` placeholder.
    pub title_key: &'static str,
    /// Interpolation variable for [`Self::title_key`].
    pub title_vars: [(&'static str, &'static str); 1],
    /// Translation key for the menu subtitle.
    pub subtitle_key: &'static str,
    /// Translation key for the menu group.
    pub group_key: &'static str,
}

/// Priority options, in shipped display order.
pub const PRIORITY_OPTIONS: [PriorityOption; 3] = [
    PriorityOption {
        priority: Priority::A,
        title_key: "editor.priority_level",
        title_vars: [("priority", "A")],
        subtitle_key: "editor.high_priority",
        group_key: "editor.priority",
    },
    PriorityOption {
        priority: Priority::B,
        title_key: "editor.priority_level",
        title_vars: [("priority", "B")],
        subtitle_key: "editor.medium_priority",
        group_key: "editor.priority",
    },
    PriorityOption {
        priority: Priority::C,
        title_key: "editor.priority_level",
        title_vars: [("priority", "C")],
        subtitle_key: "editor.low_priority",
        group_key: "editor.priority",
    },
];

/// A page-level marker/priority badge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerBadge {
    pub block_id: BlockId,
    pub marker: Option<TaskMarker>,
    pub priority: Option<Priority>,
}

/// A due-reminder chip model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderChip {
    pub block_id: BlockId,
    pub due_iso: String,
    pub overdue: bool,
    pub label_key: &'static str,
    pub vars: Vec<(String, String)>,
}

/// Detect and apply marker keywords at the start of top-level blocks.
///
/// Only journal pages (`journals/…`) are processed, matching
/// `markerDetection.ts`. Blocks that already carry a marker are skipped. When a
/// keyword is applied, the keyword and the whitespace after it are removed from
/// the content exactly like the React helper (including the empty-content case
/// for a block that is only the keyword).
pub fn detect_and_apply_markers(tree: &mut BlockTree, page_path: &str) {
    if !page_path.starts_with("journals/") {
        return;
    }
    // The React helper iterates the root block array only; children are left
    // untouched. Mirror that so detection cannot silently rewrite nested rows.
    let roots: Vec<BlockId> = tree.roots().iter().map(|b| b.id).collect();
    for id in roots {
        let Some(block) = tree.get(id) else { continue };
        if block.marker.is_some() {
            continue;
        }
        let trimmed = block.content.trim();
        let Some(first_word) = trimmed.split_whitespace().next() else {
            continue;
        };
        let keyword = first_word.to_uppercase();
        let Some(marker) = TaskMarker::parse(&keyword) else {
            continue;
        };
        let rest = trimmed[first_word.len()..].trim().to_string();
        let changed = rest != trimmed;
        if let Some(block) = tree.get_mut(id) {
            block.marker = Some(marker);
            if changed {
                block.content = rest;
            }
        }
    }
}

/// Build the page-level badge strip in document order.
///
/// Only blocks that carry a marker or priority are included.
pub fn badge_row(doc: &EditorDocument) -> Vec<MarkerBadge> {
    doc.blocks()
        .into_iter()
        .filter(|b| b.marker.is_some() || b.priority.is_some())
        .map(|b| MarkerBadge {
            block_id: b.id,
            marker: b.marker,
            priority: b.priority,
        })
        .collect()
}

/// Build reminder chips for every block with a parsable `due` property.
///
/// `now` is injected so the humanization thresholds are deterministic in tests.
/// The thresholds and translation keys match `humanizeDue` exactly.
pub fn reminder_chips(doc: &EditorDocument, now: DateTime<Utc>) -> Vec<ReminderChip> {
    let mut chips = Vec::new();
    for block in doc.blocks() {
        let Some(raw) = block.properties.get("due") else {
            continue;
        };
        let Some(due) = pkm_block::parse_due(raw, &now) else {
            continue;
        };
        let overdue = due < now;
        let (label_key, vars) = humanize_due(due, now);
        chips.push(ReminderChip {
            block_id: block.id,
            due_iso: raw.clone(),
            overdue,
            label_key,
            vars,
        });
    }
    chips
}

/// Set or clear a block reminder by writing/removing the `due` property.
///
/// `iso_or_none` is stored verbatim; callers should pass RFC3339 UTC (the same
/// format `localInputToUtcIso` produces in the React app).
pub fn set_reminder(
    tree: &mut BlockTree,
    block_id: BlockId,
    iso_or_none: Option<&str>,
) -> PkmResult<()> {
    match iso_or_none {
        Some(iso) => set_property(tree, block_id, "due", iso),
        None => remove_property(tree, block_id, "due"),
    }
}

/// Port of `humanizeDue` returning the translation key and interpolation vars.
fn humanize_due(due: DateTime<Utc>, now: DateTime<Utc>) -> (&'static str, Vec<(String, String)>) {
    let diff_ms = due.signed_duration_since(now).num_milliseconds();
    let abs_ms = diff_ms.unsigned_abs();
    let is_future = diff_ms >= 0;

    if abs_ms < 60_000 {
        return if is_future {
            ("reminders.humanize.now", Vec::new())
        } else {
            ("reminders.humanize.overdue_now", Vec::new())
        };
    }
    let minutes = ((abs_ms as f64 / 60_000.0).round() as u64).max(1);
    if minutes < 60 {
        return if is_future {
            (
                "reminders.humanize.due_in_min",
                vec![("minutes".to_string(), minutes.to_string())],
            )
        } else {
            (
                "reminders.humanize.overdue_min",
                vec![("minutes".to_string(), minutes.to_string())],
            )
        };
    }
    let hours = ((abs_ms as f64 / 3_600_000.0).round() as u64).max(1);
    if hours < 24 {
        return if is_future {
            (
                "reminders.humanize.due_in_hours",
                vec![("hours".to_string(), hours.to_string())],
            )
        } else {
            (
                "reminders.humanize.overdue_hours",
                vec![("hours".to_string(), hours.to_string())],
            )
        };
    }
    (
        "reminders.humanize.due_date",
        vec![("date".to_string(), format_due_local(due).replace('T', " "))],
    )
}

/// Format a UTC timestamp as a local `datetime-local` value.
fn format_due_local(due: DateTime<Utc>) -> String {
    due.with_timezone(&Local)
        .format("%Y-%m-%dT%H:%M")
        .to_string()
}

#[cfg(test)]
mod tests;
