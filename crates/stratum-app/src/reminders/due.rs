//! Reminder due-time parsing, bucketing and humanization.
//!
//! Ports the pure date helpers from the shipped React reminder utilities:
//! `parseDue` / `groupReminders` / `humanizeDue`. The module has no I/O so it
//! is safe to unit-test with fixed clocks.

use chrono::{DateTime, Local, LocalResult, NaiveDateTime, TimeZone, Utc};

/// Parse a due timestamp into a UTC instant.
///
/// Accepts RFC 3339 values (any offset, normalized to UTC) and a naive
/// `%Y-%m-%dT%H:%M` value interpreted in the system local timezone. Returns
/// `None` for empty or malformed input.
pub fn parse_due(raw: &str) -> Option<DateTime<Utc>> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Some(dt.with_timezone(&Utc));
    }

    let naive = NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M").ok()?;
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(dt) => Some(dt.with_timezone(&Utc)),
        LocalResult::Ambiguous(earliest, _) => Some(earliest.with_timezone(&Utc)),
        LocalResult::None => None,
    }
}

/// Reminder indices grouped by urgency.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReminderBuckets {
    /// Reminders strictly before `now`.
    pub overdue: Vec<usize>,
    /// Reminders later than `now` on the same local day.
    pub today: Vec<usize>,
    /// Reminders on a later local day.
    pub upcoming: Vec<usize>,
}

/// Group due timestamps into overdue/today/upcoming buckets.
///
/// The returned indices point into `dues`. Ordering follows the input order,
/// mirroring `groupReminders`.
pub fn bucket(dues: &[DateTime<Utc>], now: DateTime<Local>) -> ReminderBuckets {
    let mut buckets = ReminderBuckets::default();
    let now_utc = now.with_timezone(&Utc);
    let today = now.date_naive();
    for (index, due) in dues.iter().enumerate() {
        if *due < now_utc {
            buckets.overdue.push(index);
        } else if due.with_timezone(&Local).date_naive() == today {
            buckets.today.push(index);
        } else {
            buckets.upcoming.push(index);
        }
    }
    buckets
}

/// A humanized due description: an i18n key plus interpolation arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanizedDue {
    /// The i18n key to render.
    pub key: &'static str,
    /// Interpolation arguments as `(name, value)` pairs.
    pub args: Vec<(String, String)>,
}

fn arg(name: &str, value: String) -> (String, String) {
    (name.to_string(), value)
}

/// Produce a short human-readable due description.
///
/// Mirrors `humanizeDue`: `< 60 s` is "now"/"overdue", `< 60 min` reports
/// minutes, `< 24 h` reports hours, and anything older/further uses the local
/// `YYYY-MM-DD HH:MM` date argument.
pub fn humanize(due: DateTime<Utc>, now: DateTime<Local>) -> HumanizedDue {
    let diff_ms = due
        .signed_duration_since(now.with_timezone(&Utc))
        .num_milliseconds();
    let abs_ms = diff_ms.abs();
    let is_future = diff_ms >= 0;

    if abs_ms < 60_000 {
        return HumanizedDue {
            key: if is_future {
                "reminders.humanize.now"
            } else {
                "reminders.humanize.overdue_now"
            },
            args: Vec::new(),
        };
    }

    let minutes = ((abs_ms as f64 / 60_000.0).round() as i64).max(1);
    if minutes < 60 {
        return HumanizedDue {
            key: if is_future {
                "reminders.humanize.due_in_min"
            } else {
                "reminders.humanize.overdue_min"
            },
            args: vec![arg("minutes", minutes.to_string())],
        };
    }

    let hours = ((abs_ms as f64 / 3_600_000.0).round() as i64).max(1);
    if hours < 24 {
        return HumanizedDue {
            key: if is_future {
                "reminders.humanize.due_in_hours"
            } else {
                "reminders.humanize.overdue_hours"
            },
            args: vec![arg("hours", hours.to_string())],
        };
    }

    HumanizedDue {
        key: "reminders.humanize.due_date",
        args: vec![arg(
            "date",
            due.with_timezone(&Local)
                .format("%Y-%m-%d %H:%M")
                .to_string(),
        )],
    }
}

/// Format a due timestamp as a local `datetime-local` input value.
///
/// Mirrors `dueToLocalInput` from the shipped reminder utilities: the value is
/// rendered in the system local timezone as `YYYY-MM-DDTHH:MM`. Empty or
/// malformed input returns an empty string so the dialog can show a blank
/// field instead of leaking a raw timestamp.
pub fn due_to_local_input(raw: &str) -> String {
    match parse_due(raw) {
        Some(due) => due
            .with_timezone(&Local)
            .format("%Y-%m-%dT%H:%M")
            .to_string(),
        None => String::new(),
    }
}

/// Convert a local `datetime-local` input value into an RFC 3339 UTC string.
///
/// Mirrors `localInputToUtcIso`. A full RFC 3339 value is accepted and
/// normalized to UTC; a naive `YYYY-MM-DDTHH:MM` value is interpreted in the
/// system local timezone. Invalid or empty input returns an empty string so
/// the caller can decide whether to default to "now".
pub fn local_input_to_utc_iso(input: &str) -> String {
    let input = input.trim();
    if input.is_empty() {
        return String::new();
    }

    if let Ok(due) = DateTime::parse_from_rfc3339(input) {
        return due.with_timezone(&Utc).to_rfc3339();
    }

    let Ok(naive) = NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M") else {
        return String::new();
    };
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(due) => due.with_timezone(&Utc).to_rfc3339(),
        LocalResult::Ambiguous(earliest, _) => earliest.with_timezone(&Utc).to_rfc3339(),
        LocalResult::None => String::new(),
    }
}

/// A fixed local clock used by the tests.
#[cfg(test)]
fn local_at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single()
        .expect("valid local time")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn parses_rfc3339_and_normalizes_to_utc() {
        let parsed = parse_due("2026-10-15T09:00:00+02:00").expect("valid");
        assert_eq!(parsed, Utc.with_ymd_and_hms(2026, 10, 15, 7, 0, 0).unwrap());
    }

    #[test]
    fn parses_naive_local_values() {
        let parsed = parse_due("2026-10-15T09:00").expect("valid naive");
        let expected = local_at(2026, 10, 15, 9, 0).with_timezone(&Utc);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn rejects_empty_and_malformed_values() {
        assert_eq!(parse_due(""), None);
        assert_eq!(parse_due("   "), None);
        assert_eq!(parse_due("not-a-date"), None);
        assert_eq!(parse_due("2026-10-15"), None);
    }

    #[test]
    fn buckets_exactly_now_as_today() {
        let now = local_at(2026, 11, 10, 12, 0);
        let due = now.with_timezone(&Utc);
        let buckets = bucket(&[due], now);
        assert_eq!(buckets.overdue, Vec::<usize>::new());
        assert_eq!(buckets.today, vec![0]);
        assert_eq!(buckets.upcoming, Vec::<usize>::new());
    }

    #[test]
    fn buckets_overdue_today_and_upcoming() {
        let now = local_at(2026, 11, 10, 12, 0);
        let overdue = (now - Duration::minutes(5)).with_timezone(&Utc);
        let today = (now + Duration::hours(2)).with_timezone(&Utc);
        let tomorrow = local_at(2026, 11, 11, 9, 0).with_timezone(&Utc);
        let buckets = bucket(&[overdue, today, tomorrow], now);
        assert_eq!(buckets.overdue, vec![0]);
        assert_eq!(buckets.today, vec![1]);
        assert_eq!(buckets.upcoming, vec![2]);
    }

    #[test]
    fn humanize_covers_now_and_overdue_now() {
        let now = local_at(2026, 11, 10, 12, 0);
        let soon = (now + Duration::seconds(30)).with_timezone(&Utc);
        let past = (now - Duration::seconds(30)).with_timezone(&Utc);
        assert_eq!(humanize(soon, now).key, "reminders.humanize.now");
        assert_eq!(humanize(past, now).key, "reminders.humanize.overdue_now");
        assert!(humanize(soon, now).args.is_empty());
    }

    #[test]
    fn humanize_covers_minutes_and_hours() {
        let now = local_at(2026, 11, 10, 12, 0);
        let in_minutes = (now + Duration::minutes(5)).with_timezone(&Utc);
        let overdue_minutes = (now - Duration::minutes(7)).with_timezone(&Utc);
        let in_hours = (now + Duration::hours(3)).with_timezone(&Utc);
        let overdue_hours = (now - Duration::hours(2)).with_timezone(&Utc);

        let due = humanize(in_minutes, now);
        assert_eq!(due.key, "reminders.humanize.due_in_min");
        assert_eq!(due.args, vec![arg("minutes", "5".to_string())]);

        let overdue = humanize(overdue_minutes, now);
        assert_eq!(overdue.key, "reminders.humanize.overdue_min");
        assert_eq!(overdue.args, vec![arg("minutes", "7".to_string())]);

        let hours = humanize(in_hours, now);
        assert_eq!(hours.key, "reminders.humanize.due_in_hours");
        assert_eq!(hours.args, vec![arg("hours", "3".to_string())]);

        let overdue_h = humanize(overdue_hours, now);
        assert_eq!(overdue_h.key, "reminders.humanize.overdue_hours");
        assert_eq!(overdue_h.args, vec![arg("hours", "2".to_string())]);
    }

    #[test]
    fn humanize_falls_back_to_a_local_date() {
        let now = local_at(2026, 11, 10, 12, 0);
        let due = local_at(2026, 11, 15, 9, 30).with_timezone(&Utc);
        let humanized = humanize(due, now);
        assert_eq!(humanized.key, "reminders.humanize.due_date");
        assert_eq!(
            humanized.args,
            vec![arg("date", "2026-11-15 09:30".to_string())]
        );
    }

    #[test]
    fn due_to_local_input_renders_the_local_wall_clock() {
        let due = local_at(2026, 11, 15, 9, 30).with_timezone(&Utc);
        assert_eq!(
            due_to_local_input(&due.to_rfc3339()),
            "2026-11-15T09:30".to_string()
        );
        assert_eq!(
            due_to_local_input("2026-11-15T09:30"),
            "2026-11-15T09:30".to_string()
        );
        assert_eq!(due_to_local_input(""), "");
        assert_eq!(due_to_local_input("not-a-date"), "");
    }

    #[test]
    fn local_input_round_trips_through_utc() {
        let iso = local_input_to_utc_iso("2026-11-15T09:30");
        let parsed = parse_due(&iso).expect("converted value parses");
        assert_eq!(parsed, local_at(2026, 11, 15, 9, 30).with_timezone(&Utc));
        assert_eq!(local_input_to_utc_iso(&iso), iso);
        assert_eq!(local_input_to_utc_iso(""), "");
        assert_eq!(local_input_to_utc_iso("   "), "");
        assert_eq!(local_input_to_utc_iso("2026-11-15"), "");
    }
}
