//! Window-free reminder grouping, humanization and due-dialog state.
//!
//! The panel view only reads these helpers; parsing, bucketing and the local
//! `datetime-local` conversion stay in [`stratum_app::reminders::due`] so the
//! native panel and the React reminder utilities agree on the same rules.

use chrono::{DateTime, Duration, Local, LocalResult, NaiveDate, TimeZone, Utc};
use stratum_app::reminders::due::{
    bucket, due_to_local_input, humanize, local_input_to_utc_iso, parse_due,
};
use stratum_i18n::tr_global;

use crate::provider::ReminderRow;

/// Reminder row indices grouped by urgency.
///
/// Indices point into the panel's loaded rows so the renderer can keep stable
/// `reminder-row-{index}` selectors across sections.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReminderGroups {
    /// Reminders strictly before `now`.
    pub overdue: Vec<usize>,
    /// Reminders later than `now` on the same local day.
    pub today: Vec<usize>,
    /// Reminders on a later local day.
    pub upcoming: Vec<usize>,
}

impl ReminderGroups {
    /// The total number of grouped rows.
    pub fn total(&self) -> usize {
        self.overdue.len() + self.today.len() + self.upcoming.len()
    }
}

/// Group rows by urgency, skipping rows with an unparseable due timestamp.
pub fn group(rows: &[ReminderRow], now: DateTime<Local>) -> ReminderGroups {
    let mut indices = Vec::with_capacity(rows.len());
    let mut dues = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        if let Some(due) = parse_due(&row.due) {
            indices.push(index);
            dues.push(due);
        }
    }
    let buckets = bucket(&dues, now);
    ReminderGroups {
        overdue: buckets
            .overdue
            .into_iter()
            .filter_map(|index| indices.get(index).copied())
            .collect(),
        today: buckets
            .today
            .into_iter()
            .filter_map(|index| indices.get(index).copied())
            .collect(),
        upcoming: buckets
            .upcoming
            .into_iter()
            .filter_map(|index| indices.get(index).copied())
            .collect(),
    }
}

/// The localized humanized due caption for one row.
pub fn humanized_due(row: &ReminderRow, now: DateTime<Local>) -> String {
    match parse_due(&row.due) {
        Some(due) => {
            let humanized = humanize(due, now);
            let args: Vec<(&str, &str)> = humanized
                .args
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_str()))
                .collect();
            tr_global(humanized.key, &args)
        }
        None => row.due.clone(),
    }
}

/// The secondary caption shown under a reminder: page title and due state.
pub fn row_subtitle(row: &ReminderRow, now: DateTime<Local>) -> String {
    let due = humanized_due(row, now);
    match row.page_title.as_deref() {
        Some(title) if !title.is_empty() => format!("{title} \u{b7} {due}"),
        _ => due,
    }
}

/// The quick due choices offered by the reminder dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickDue {
    /// One hour from the current local time.
    InOneHour,
    /// Tomorrow at 09:00 local time.
    Tomorrow0900,
    /// Today at 18:00 local time.
    Evening1800,
}

impl QuickDue {
    /// Every quick choice in shipped display order.
    pub const ALL: [QuickDue; 3] = [
        QuickDue::InOneHour,
        QuickDue::Tomorrow0900,
        QuickDue::Evening1800,
    ];

    /// The catalog key for this choice.
    pub fn label_key(self) -> &'static str {
        match self {
            QuickDue::InOneHour => "reminders.dialog.in_1_hour",
            QuickDue::Tomorrow0900 => "reminders.dialog.tomorrow_0900",
            QuickDue::Evening1800 => "reminders.dialog.evening_1800",
        }
    }

    /// Resolve the choice into a local `datetime-local` input value.
    pub fn resolve(self, now: DateTime<Local>) -> String {
        match self {
            QuickDue::InOneHour => format_local(now + Duration::hours(1)),
            QuickDue::Tomorrow0900 => {
                let tomorrow = now
                    .date_naive()
                    .succ_opt()
                    .unwrap_or_else(|| now.date_naive());
                format_local(at_local(tomorrow, 9, 0).unwrap_or(now))
            }
            QuickDue::Evening1800 => format_local(at_local(now.date_naive(), 18, 0).unwrap_or(now)),
        }
    }
}

/// The set/change/clear reminder dialog state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DueDialog {
    /// Vault-relative page path of the reminder block.
    pub page_path: String,
    /// Block id of the reminder.
    pub block_id: String,
    /// The row's due timestamp when the dialog opened, if it had one.
    pub initial_due: Option<String>,
    /// The current `datetime-local` input value.
    pub value: String,
    /// A validation/provider error shown inside the dialog.
    pub error: Option<String>,
}

impl DueDialog {
    /// Build the dialog for an existing reminder row.
    pub fn for_row(row: &ReminderRow) -> Self {
        Self {
            page_path: row.page_path.clone(),
            block_id: row.block_id.clone(),
            initial_due: Some(row.due.clone()),
            value: due_to_local_input(&row.due),
            error: None,
        }
    }

    /// The dialog title key (change an existing reminder vs set a new one).
    pub fn title_key(&self) -> &'static str {
        if self.initial_due.is_some() {
            "reminders.dialog.change_reminder"
        } else {
            "reminders.dialog.set_reminder"
        }
    }

    /// Replace the input value.
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.error = None;
    }

    /// Apply a quick choice to the input value.
    pub fn apply_quick(&mut self, quick: QuickDue, now: DateTime<Local>) {
        self.value = quick.resolve(now);
        self.error = None;
    }

    /// Convert the input into a UTC ISO string.
    ///
    /// An empty/invalid value defaults to "now", matching the shipped dialog,
    /// and the input is rewritten so the user sees what was saved. Returns
    /// `None` only when the default cannot be rendered, which cannot happen for
    /// a valid local clock.
    pub fn submit_value(&mut self) -> Option<String> {
        let mut iso = local_input_to_utc_iso(&self.value);
        if iso.is_empty() {
            iso = Utc::now().to_rfc3339();
            self.value = due_to_local_input(&iso);
        }
        Some(iso)
    }
}

/// Format a local instant as a `datetime-local` input value.
fn format_local(value: DateTime<Local>) -> String {
    value.format("%Y-%m-%dT%H:%M").to_string()
}

/// Resolve a local date/time, preferring the earliest ambiguous instant.
fn at_local(date: NaiveDate, hour: u32, minute: u32) -> Option<DateTime<Local>> {
    let naive = date.and_hms_opt(hour, minute, 0)?;
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(value) => Some(value),
        LocalResult::Ambiguous(earliest, _) => Some(earliest),
        LocalResult::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    fn local_at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .expect("valid local time")
    }

    fn reminder(id: &str, due: DateTime<Utc>) -> ReminderRow {
        ReminderRow {
            block_id: id.to_string(),
            page_path: "pages/fixture.md".to_string(),
            page_title: Some("Fixture".to_string()),
            content: "Review the fixture".to_string(),
            due: due.to_rfc3339(),
        }
    }

    #[test]
    fn groups_rows_by_urgency_and_skips_invalid_dues() {
        let now = local_at(2026, 11, 10, 12, 0);
        let rows = vec![
            reminder("overdue", (now - Duration::hours(2)).with_timezone(&Utc)),
            reminder("today", (now + Duration::hours(2)).with_timezone(&Utc)),
            reminder("upcoming", local_at(2026, 11, 12, 9, 0).with_timezone(&Utc)),
            ReminderRow {
                due: "not-a-date".to_string(),
                ..reminder("invalid", now.with_timezone(&Utc))
            },
        ];
        let groups = group(&rows, now);
        assert_eq!(groups.overdue, vec![0]);
        assert_eq!(groups.today, vec![1]);
        assert_eq!(groups.upcoming, vec![2]);
        assert_eq!(groups.total(), 3);
    }

    #[test]
    fn subtitles_combine_the_page_title_and_humanized_due() {
        let now = local_at(2026, 11, 10, 12, 0);
        let row = reminder("overdue", (now - Duration::minutes(7)).with_timezone(&Utc));
        let subtitle = row_subtitle(&row, now);
        assert!(subtitle.starts_with("Fixture \u{b7} "));
        assert!(subtitle.contains("Overdue"));
    }

    #[test]
    fn quick_choices_resolve_to_local_input_values() {
        let now = local_at(2026, 11, 10, 12, 0);
        assert_eq!(QuickDue::InOneHour.resolve(now), "2026-11-10T13:00");
        assert_eq!(QuickDue::Tomorrow0900.resolve(now), "2026-11-11T09:00");
        assert_eq!(QuickDue::Evening1800.resolve(now), "2026-11-10T18:00");
    }

    #[test]
    fn dialog_defaults_an_empty_value_to_now_and_keeps_a_valid_one() {
        let now = local_at(2026, 11, 10, 12, 0);
        let row = reminder("today", (now + Duration::hours(2)).with_timezone(&Utc));
        let mut dialog = DueDialog::for_row(&row);
        assert_eq!(dialog.title_key(), "reminders.dialog.change_reminder");
        assert_eq!(dialog.value, "2026-11-10T14:00");

        dialog.set_value("");
        let iso = dialog.submit_value().expect("defaults to now");
        assert!(parse_due(&iso).is_some());
        assert!(!dialog.value.is_empty());

        dialog.set_value("2026-11-12T08:15");
        let iso = dialog.submit_value().expect("valid input");
        assert_eq!(
            parse_due(&iso),
            Some(local_at(2026, 11, 12, 8, 15).with_timezone(&Utc))
        );
    }

    #[test]
    fn dialog_quick_apply_updates_the_input() {
        let now = local_at(2026, 11, 10, 12, 0);
        let row = reminder("today", (now + Duration::hours(2)).with_timezone(&Utc));
        let mut dialog = DueDialog::for_row(&row);
        dialog.apply_quick(QuickDue::Tomorrow0900, now);
        assert_eq!(dialog.value, "2026-11-11T09:00");
        assert_eq!(dialog.error, None);
    }
}
