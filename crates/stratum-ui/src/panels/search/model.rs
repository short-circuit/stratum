//! Search classification, score labels and status for the native search panel.
//!
//! The model is deliberately window-free: [`classify_query`] mirrors the
//! shipped `SearchPanel` routing (leading `#` is a tag search, everything else
//! is full-text) and [`score_label`] reproduces the React caption
//! (`· score: {score}`) through the shipped catalog key.

use stratum_i18n::t_global;

/// How a search query should be routed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchMode {
    /// No usable query (empty or whitespace only).
    Idle,
    /// Tag search with the leading `#` removed.
    Tag(String),
    /// Full-text search.
    FullText(String),
}

/// Classify a raw query exactly like the shipped search panel.
///
/// A leading `#` selects tag search with the marker stripped and the remainder
/// trimmed. Whitespace-only input and a bare `#` are idle so the panel does not
/// issue an empty provider call.
pub fn classify_query(raw: &str) -> SearchMode {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "#" {
        return SearchMode::Idle;
    }
    match trimmed.strip_prefix('#') {
        Some(tag) => SearchMode::Tag(tag.trim().to_string()),
        None => SearchMode::FullText(trimmed.to_string()),
    }
}

/// Format the relevance caption used by every result row.
pub fn score_label(score: f32) -> String {
    format!("{} {:.2}", t_global("search._score"), score)
}

/// The current search status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchStatus {
    /// No search has been submitted.
    Idle,
    /// A provider search is in flight.
    Searching,
    /// The last search completed.
    Done,
    /// The last search failed with a localized message.
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    #[test]
    fn classification_strips_tags_and_trims_input() {
        assert_eq!(
            classify_query("#project"),
            SearchMode::Tag("project".to_string())
        );
        assert_eq!(
            classify_query("#  spaced tag  "),
            SearchMode::Tag("spaced tag".to_string())
        );
        assert_eq!(
            classify_query("todo"),
            SearchMode::FullText("todo".to_string())
        );
        assert_eq!(
            classify_query("  todo  "),
            SearchMode::FullText("todo".to_string())
        );
    }

    #[test]
    fn classification_treats_empty_and_bare_hash_as_idle() {
        assert_eq!(classify_query(""), SearchMode::Idle);
        assert_eq!(classify_query("   "), SearchMode::Idle);
        assert_eq!(classify_query("#"), SearchMode::Idle);
        assert_eq!(classify_query(" # "), SearchMode::Idle);
    }

    #[test]
    fn score_labels_use_two_decimals() {
        assert_eq!(score_label(1.0), "· score: 1.00");
        assert_eq!(score_label(0.72), "· score: 0.72");
    }
}
