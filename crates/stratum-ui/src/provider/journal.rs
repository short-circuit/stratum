//! Journal paths, date parsing and the real journal/repair provider mapping.
//!
//! The pure helpers ([`journal_path`], [`parse_journal_date`] and
//! [`journal_dates_from_paths`]) are shared by the fixture provider and the
//! journal panel, and are unit-tested without a window. The [`AppProvider`]
//! implementation maps the journal seam onto [`stratum_app::StratumService`]
//! using the blocking bridge in [`super`].

use chrono::NaiveDate;

use super::AppProvider;

/// Build the vault-relative journal page path for `date`.
///
/// `date` is expected to be a `YYYY-MM-DD` string; the helper does not validate
/// it, so callers that accept user input should run [`parse_journal_date`] on
/// the result first.
pub fn journal_path(date: &str) -> String {
    format!("journals/{date}.md")
}

/// Extract a validated `YYYY-MM-DD` date from a journal page path.
///
/// Returns `None` for non-journal paths, missing extensions and impossible
/// calendar dates (for example `2026-02-30`), so a stray file cannot create a
/// phantom day in the stack or calendar.
pub fn parse_journal_date(path: &str) -> Option<String> {
    let stem = path.strip_prefix("journals/")?.strip_suffix(".md")?;
    let date = NaiveDate::parse_from_str(stem, "%Y-%m-%d").ok()?;
    Some(date.format("%Y-%m-%d").to_string())
}

/// Collect the valid, unique journal dates from a page-path list, newest first.
///
/// Invalid entries are skipped and duplicates are collapsed, so the output is
/// deterministic regardless of the provider's ordering.
pub fn journal_dates_from_paths<I, S>(paths: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut dates: Vec<String> = paths
        .into_iter()
        .filter_map(|path| parse_journal_date(path.as_ref()))
        .collect();
    dates.sort();
    dates.dedup();
    dates.reverse();
    dates
}

impl AppProvider {
    /// List the journal dates the vault currently has, newest first.
    pub(super) fn read_journal_dates(&self) -> Vec<String> {
        match self.runtime.block_on(self.service.list_pages()) {
            Ok(list) => journal_dates_from_paths(list.pages.into_iter().map(|page| page.path)),
            Err(_) => Vec::new(),
        }
    }

    /// Ensure the local date's journal page exists and return its path.
    pub(super) fn ensure_journal(&self) -> Result<String, String> {
        self.runtime
            .block_on(self.service.ensure_today_journal())
            .map(|page| page.path)
    }

    /// Create a journal page for `date`, tolerating an already-existing page.
    pub(super) fn create_journal(&self, date: &str) -> Result<String, String> {
        let path = journal_path(date);
        match self.runtime.block_on(
            self.service
                .create_page(path.clone(), Some(date.to_string())),
        ) {
            Ok(page) => Ok(page.path),
            Err(error) if error.contains("already exists") => Ok(path),
            Err(error) => Err(error),
        }
    }

    /// Reconcile every on-disk page into SQLite, discarding progress events.
    pub(super) fn repair_database(&self) -> Result<(), String> {
        self.runtime
            .block_on(self.service.repair_db_from_disk(&stratum_app::NoopSink))
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_paths_round_trip() {
        assert_eq!(journal_path("2026-11-10"), "journals/2026-11-10.md");
        assert_eq!(
            parse_journal_date("journals/2026-11-10.md"),
            Some("2026-11-10".to_string())
        );
    }

    #[test]
    fn parse_rejects_non_journal_and_impossible_dates() {
        assert_eq!(parse_journal_date("pages/Welcome.md"), None);
        assert_eq!(parse_journal_date("journals/2026-11-10"), None);
        assert_eq!(parse_journal_date("journals/not-a-date.md"), None);
        assert_eq!(parse_journal_date("journals/2026-02-30.md"), None);
        assert_eq!(
            parse_journal_date("journals/2024-02-29.md"),
            Some("2024-02-29".to_string())
        );
    }

    #[test]
    fn journal_dates_are_valid_unique_and_descending() {
        let dates = journal_dates_from_paths([
            "journals/2026-11-08.md",
            "journals/2026-11-09.md",
            "pages/Welcome.md",
            "journals/2026-11-08.md",
            "journals/2026-02-30.md",
            "journals/2026-11-07.md",
        ]);
        assert_eq!(dates, ["2026-11-09", "2026-11-08", "2026-11-07"]);
    }

    #[test]
    fn empty_path_list_produces_no_dates() {
        assert!(journal_dates_from_paths(Vec::<String>::new()).is_empty());
    }
}
