//! Page-surface provider seam.
//!
//! The page panel needs page metadata, the Note menu actions (reindex,
//! normalize, move, delete), backlinks with snippets and related-note
//! suggestions. [`AppProvider`] maps those calls onto
//! `stratum_app::StratumService`; [`super::FixtureProvider`] returns
//! deterministic data for `pages/fixture.md`.
//!
//! The pure helpers in this module ([`normalize_move_path`],
//! [`group_backlinks`], [`clamp_relevance`]) are shared by the page panel and
//! its window-free tests.

use stratum_app::search::ConnectionSuggestion;
use stratum_app::NoopSink;

use super::AppProvider;

/// Page metadata shown in the page header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageInfo {
    /// Vault-relative page path.
    pub path: String,
    /// Frontmatter title, when the page has one.
    pub title: Option<String>,
    /// File-stem slug used when the title is absent.
    pub slug: String,
    /// Number of blocks on the page.
    pub block_count: u64,
    /// RFC 3339 modification timestamp.
    pub modified_at: String,
}

/// Result of reindexing one page.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReindexSummary {
    /// Pages indexed successfully.
    pub succeeded: u64,
    /// Pages that failed to index.
    pub failed: u64,
    /// Per-page error messages.
    pub errors: Vec<String>,
}

/// One backlink row (linked reference or unlinked mention).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BacklinkRow {
    /// Block id of the source row.
    pub source_id: String,
    /// Vault-relative path of the page that contains the row.
    pub source_page: String,
    /// Source row content.
    pub context: String,
    /// Whether the row contains an explicit wiki-link to the page.
    pub is_linked: bool,
}

/// A bounded backlink snippet shown by the Ctrl+hover popup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BacklinkSnippet {
    /// Display title of the note that contains the anchor.
    pub note_title: String,
    /// Exact content of the backlinked anchor row.
    pub anchor_content: String,
    /// Bounded surrounding rows in document order.
    pub context: Vec<String>,
}

/// One related-note suggestion.
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionRow {
    /// Suggested page title.
    pub title: String,
    /// Suggested page path.
    pub page_path: String,
    /// Raw relevance score reported by the index.
    pub score: f64,
    /// Preview snippet from the suggested page.
    pub snippet: String,
}

/// Normalize a user-entered move path.
///
/// The shipped `MovePageDialog` trims the input and appends `.md` when the
/// extension is missing. An empty input stays empty so validation can report
/// the localized "path is required" error.
pub fn normalize_move_path(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.ends_with(".md") {
        return trimmed.to_string();
    }
    format!("{trimmed}.md")
}

/// Partition backlink rows into linked references and unlinked mentions.
///
/// The input order is preserved inside each partition so the card lists rows in
/// the same order the service returned them.
pub fn group_backlinks(rows: Vec<BacklinkRow>) -> (Vec<BacklinkRow>, Vec<BacklinkRow>) {
    let mut linked = Vec::new();
    let mut unlinked = Vec::new();
    for row in rows {
        if row.is_linked {
            linked.push(row);
        } else {
            unlinked.push(row);
        }
    }
    (linked, unlinked)
}

/// Clamp a relevance score into the 0..=100 percentage range the card shows.
///
/// Non-finite values (NaN/inf) clamp to zero so a malformed index score cannot
/// render as an empty or nonsensical percentage.
pub fn clamp_relevance(score: f64) -> u32 {
    if !score.is_finite() {
        return 0;
    }
    score.clamp(0.0, 100.0).round() as u32
}

impl AppProvider {
    /// Read page metadata through the service layer.
    pub(super) fn read_page_info(&self, path: &str) -> Option<PageInfo> {
        self.runtime
            .block_on(self.service.open_page(path.to_string()))
            .ok()
            .map(|page| PageInfo {
                path: page.path,
                title: page.title,
                slug: page.slug,
                block_count: page.block_count as u64,
                modified_at: page.modified_at,
            })
    }

    /// Reindex one page from disk into SQLite and the search index.
    pub(super) fn reindex_one_page(&self, path: &str) -> Result<ReindexSummary, String> {
        self.runtime
            .block_on(self.service.reindex_page(path.to_string(), &NoopSink))
            .map(|result| ReindexSummary {
                succeeded: result.succeeded as u64,
                failed: result.failed as u64,
                errors: result.errors,
            })
    }

    /// Normalize one page file through the block parser.
    pub(super) fn normalize_one_page(&self, path: &str) -> Result<(), String> {
        self.runtime
            .block_on(self.service.normalize_file(path.to_string(), &NoopSink))
    }

    /// Move/rename a page, returning the stored new path.
    pub(super) fn move_one_page(&self, old_path: &str, new_path: &str) -> Result<String, String> {
        self.runtime
            .block_on(
                self.service
                    .move_page(old_path.to_string(), new_path.to_string(), &NoopSink),
            )
            .map(|page| page.path)
    }

    /// Delete a page.
    pub(super) fn delete_one_page(&self, path: &str) -> Result<(), String> {
        self.runtime
            .block_on(self.service.delete_page(path.to_string()))
    }

    /// Read linked references and unlinked mentions for `path`.
    pub(super) fn read_backlinks(&self, path: &str) -> Vec<BacklinkRow> {
        match self
            .runtime
            .block_on(self.service.get_page_backlinks(path.to_string()))
        {
            Ok(rows) => rows
                .into_iter()
                .map(|row| BacklinkRow {
                    source_id: row.source_id,
                    source_page: row.source_page,
                    context: row.context,
                    is_linked: row.is_linked,
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Read the bounded snippet around a backlink anchor.
    pub(super) fn read_backlink_snippet(
        &self,
        note_id: &str,
        anchor_id: &str,
    ) -> Option<BacklinkSnippet> {
        self.runtime
            .block_on(
                self.service
                    .get_backlink_snippet(note_id.to_string(), anchor_id.to_string()),
            )
            .ok()
            .map(|snippet| BacklinkSnippet {
                note_title: snippet.note_title,
                anchor_content: snippet.anchor_content,
                context: snippet.context,
            })
    }

    /// Read related-note suggestions for `path`.
    pub(super) fn read_suggestions(&self, path: &str) -> Result<Vec<ConnectionRow>, String> {
        self.runtime
            .block_on(self.service.suggest_connections(path.to_string()))
            .map(|rows| rows.into_iter().map(connection_row).collect())
    }

    /// Append one markdown block to `path`.
    pub(super) fn append_block(&self, path: &str, markdown: &str) -> Result<(), String> {
        self.runtime
            .block_on(
                self.service
                    .insert_block(path.to_string(), markdown.to_string(), None, None),
            )
            .map(|_| ())
    }
}

/// Map a service suggestion onto the shell DTO.
fn connection_row(suggestion: ConnectionSuggestion) -> ConnectionRow {
    ConnectionRow {
        title: suggestion.title,
        page_path: suggestion.page_path,
        score: suggestion.score as f64,
        snippet: suggestion.snippet,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    fn row(id: &str, linked: bool) -> BacklinkRow {
        BacklinkRow {
            source_id: id.to_string(),
            source_page: format!("pages/{id}.md"),
            context: format!("context {id}"),
            is_linked: linked,
        }
    }

    #[test]
    fn move_paths_trim_and_append_the_markdown_extension() {
        assert_eq!(normalize_move_path("  pages/New  "), "pages/New.md");
        assert_eq!(normalize_move_path("pages/New.md"), "pages/New.md");
        assert_eq!(normalize_move_path("   "), "");
    }

    #[test]
    fn backlink_grouping_preserves_stable_order() {
        let rows = vec![
            row("a", true),
            row("b", false),
            row("c", true),
            row("d", false),
        ];
        let (linked, unlinked) = group_backlinks(rows);
        let linked_ids: Vec<_> = linked.iter().map(|row| row.source_id.as_str()).collect();
        let unlinked_ids: Vec<_> = unlinked.iter().map(|row| row.source_id.as_str()).collect();
        assert_eq!(linked_ids, ["a", "c"]);
        assert_eq!(unlinked_ids, ["b", "d"]);
    }

    #[test]
    fn relevance_is_clamped_to_a_percentage() {
        assert_eq!(clamp_relevance(-12.0), 0);
        assert_eq!(clamp_relevance(0.0), 0);
        assert_eq!(clamp_relevance(42.4), 42);
        assert_eq!(clamp_relevance(42.6), 43);
        assert_eq!(clamp_relevance(150.0), 100);
        assert_eq!(clamp_relevance(f64::NAN), 0);
        assert_eq!(clamp_relevance(f64::INFINITY), 0);
    }
}
