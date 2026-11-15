//! Display helpers for saved-query metadata.
//!
//! [`format_updated_at`] mirrors the query panel's timestamp helper. Invalid or
//! empty input is returned verbatim so the UI never renders `NaN` or an empty
//! label for a row that has metadata.

use chrono::{DateTime, Local, Utc};

/// Format a saved query's `updated_at` timestamp for display.
///
/// Valid RFC 3339 timestamps are rendered as a short relative label
/// (`just now`, `N min ago`, `N h ago`, `N d ago`) and fall back to a local
/// `YYYY-MM-DD HH:MM` string for older rows. Empty input returns an empty
/// string; unparseable input is returned unchanged.
pub fn format_updated_at(iso: &str) -> String {
    if iso.is_empty() {
        return String::new();
    }
    let Ok(updated) = DateTime::parse_from_rfc3339(iso) else {
        return iso.to_string();
    };
    relative(updated.with_timezone(&Utc), Utc::now())
}

/// Render a relative label for `updated` against `now`.
fn relative(updated: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let diff = now.signed_duration_since(updated).num_seconds();
    if diff <= 0 {
        return "just now".to_string();
    }
    if diff < 60 {
        return "just now".to_string();
    }
    let minutes = diff / 60;
    if minutes < 60 {
        return format!("{minutes} min ago");
    }
    let hours = diff / 3_600;
    if hours < 24 {
        return format!("{hours} h ago");
    }
    let days = diff / 86_400;
    if days < 7 {
        return format!("{days} d ago");
    }
    updated
        .with_timezone(&Local)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(iso: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(iso)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn empty_and_invalid_input_are_returned_verbatim() {
        assert_eq!(format_updated_at(""), "");
        assert_eq!(format_updated_at("not-a-date"), "not-a-date");
    }

    #[test]
    fn relative_labels_cover_minutes_hours_and_days() {
        let now = at("2026-11-10T12:00:00Z");
        assert_eq!(relative(at("2026-11-10T11:59:30Z"), now), "just now");
        assert_eq!(relative(at("2026-11-10T11:55:00Z"), now), "5 min ago");
        assert_eq!(relative(at("2026-11-10T09:00:00Z"), now), "3 h ago");
        assert_eq!(relative(at("2026-11-07T12:00:00Z"), now), "3 d ago");
    }

    #[test]
    fn old_timestamps_fall_back_to_an_absolute_local_label() {
        let now = at("2026-11-10T12:00:00Z");
        let updated = at("2026-10-01T08:30:00Z");
        let expected = updated
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M")
            .to_string();
        assert_eq!(relative(updated, now), expected);
    }

    #[test]
    fn future_timestamps_render_as_just_now() {
        let now = at("2026-11-10T12:00:00Z");
        let future = Local
            .with_ymd_and_hms(2026, 11, 10, 13, 0, 0)
            .single()
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(relative(future, now), "just now");
    }
}
