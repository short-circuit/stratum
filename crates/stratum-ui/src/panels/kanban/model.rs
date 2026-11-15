//! Kanban column mapping and board grouping.
//!
//! The board is a view over task markers, so the column a card belongs to is
//! derived from its marker exactly as the shipped panel did: `TODO`, `NOW`,
//! `LATER` and `WAITING` land in To Do, `DOING` in In Progress, `DONE` and
//! `CANCELLED` in Done. Moving a card between columns rewrites its marker to
//! that column's canonical marker (`TODO`, `DOING`, `DONE`), which is what keeps
//! the board and the Markdown notes in sync.
//!
//! [`ColumnId`] and the grouping helpers carry no UI state, so the contract
//! tests can pin the mapping without a window.

use crate::provider::panels::KanbanCardRow;

/// A board column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColumnId {
    /// To Do: not started yet.
    ToDo,
    /// In Progress: actively being worked on.
    InProgress,
    /// Done: completed or cancelled.
    Done,
}

impl ColumnId {
    /// Every column, in board order.
    pub const ALL: [ColumnId; 3] = [ColumnId::ToDo, ColumnId::InProgress, ColumnId::Done];

    /// The catalog key for the column header.
    pub fn label_key(self) -> &'static str {
        match self {
            ColumnId::ToDo => "kanban.to_do",
            ColumnId::InProgress => "kanban.in_progress",
            ColumnId::Done => "kanban.done",
        }
    }

    /// The column's accent color (the shipped `#f59a0b`, `#3b82f6`, `#10b981`).
    pub fn accent(self) -> u32 {
        match self {
            ColumnId::ToDo => 0xf59a0b,
            ColumnId::InProgress => 0x3b82f6,
            ColumnId::Done => 0x10b981,
        }
    }

    /// The markers that place a card in this column.
    pub fn markers(self) -> &'static [&'static str] {
        match self {
            ColumnId::ToDo => &["TODO", "NOW", "LATER", "WAITING"],
            ColumnId::InProgress => &["DOING"],
            ColumnId::Done => &["DONE", "CANCELLED"],
        }
    }

    /// The marker written when a card is dropped into this column.
    pub fn target_marker(self) -> &'static str {
        match self {
            ColumnId::ToDo => "TODO",
            ColumnId::InProgress => "DOING",
            ColumnId::Done => "DONE",
        }
    }

    /// The stable element-id fragment for this column.
    pub fn slug(self) -> &'static str {
        match self {
            ColumnId::ToDo => "todo",
            ColumnId::InProgress => "in-progress",
            ColumnId::Done => "done",
        }
    }

    /// Parse a column from the capture flags (`todo`, `to_do`, `in_progress`,
    /// `in-progress`, `done`).
    pub fn from_env(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().replace('-', "_").as_str() {
            "todo" | "to_do" => Some(ColumnId::ToDo),
            "in_progress" | "doing" => Some(ColumnId::InProgress),
            "done" => Some(ColumnId::Done),
            _ => None,
        }
    }
}

/// Resolve the column for a task marker.
///
/// Unknown or absent markers are not board cards; the query that feeds the
/// board only returns marker-bearing blocks, so `None` here means a marker that
/// is outside the shipped set (for example a custom plugin marker).
pub fn column_for_marker(marker: &str) -> Option<ColumnId> {
    let marker = marker.trim().to_ascii_uppercase();
    ColumnId::ALL
        .into_iter()
        .find(|column| column.markers().contains(&marker.as_str()))
}

/// Group cards into board columns, in board order.
///
/// Cards keep their input order inside a column; cards with an unknown or
/// absent marker are omitted (they are not tasks the board can place).
pub fn group(cards: &[KanbanCardRow]) -> Vec<(ColumnId, Vec<KanbanCardRow>)> {
    ColumnId::ALL
        .into_iter()
        .map(|column| {
            let rows = cards
                .iter()
                .filter(|card| {
                    card.marker
                        .as_deref()
                        .and_then(column_for_marker)
                        .is_some_and(|resolved| resolved == column)
                })
                .cloned()
                .collect();
            (column, rows)
        })
        .collect()
}

/// The label a card shows for its page.
///
/// Uses the page title when the front matter carries one, else the file stem —
/// the shipped card fell back to stripping `journals/` and `.md`.
pub fn page_label(card: &KanbanCardRow) -> String {
    if let Some(title) = card
        .page_title
        .as_ref()
        .filter(|title| !title.trim().is_empty())
    {
        return title.clone();
    }
    let path = card.page_path.as_str();
    let stem = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".md");
    stem.to_string()
}

/// Whether the card lives on a journal page.
pub fn is_journal(card: &KanbanCardRow) -> bool {
    card.page_path.starts_with("journals/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str, marker: Option<&str>, page: &str) -> KanbanCardRow {
        let mut row = KanbanCardRow::new(id, "task", "TODO", page);
        row.marker = marker.map(str::to_string);
        row
    }

    #[test]
    fn every_shipped_marker_maps_to_its_column() {
        for (marker, column) in [
            ("TODO", ColumnId::ToDo),
            ("NOW", ColumnId::ToDo),
            ("LATER", ColumnId::ToDo),
            ("WAITING", ColumnId::ToDo),
            ("DOING", ColumnId::InProgress),
            ("DONE", ColumnId::Done),
            ("CANCELLED", ColumnId::Done),
        ] {
            assert_eq!(column_for_marker(marker), Some(column), "{marker}");
        }
        assert_eq!(column_for_marker("todo"), Some(ColumnId::ToDo));
        assert_eq!(column_for_marker("  doing "), Some(ColumnId::InProgress));
        assert_eq!(column_for_marker("CUSTOM"), None);
    }

    #[test]
    fn grouping_places_every_marker_and_drops_unknowns() {
        let cards = vec![
            card("a", Some("TODO"), "pages/a.md"),
            card("b", Some("DOING"), "pages/b.md"),
            card("c", Some("DONE"), "pages/c.md"),
            card("d", Some("WAITING"), "pages/d.md"),
            card("e", None, "pages/e.md"),
        ];
        let grouped = group(&cards);
        let counts: Vec<(ColumnId, Vec<String>)> = grouped
            .into_iter()
            .map(|(column, rows)| (column, rows.into_iter().map(|row| row.id).collect()))
            .collect();
        assert_eq!(
            counts,
            vec![
                (ColumnId::ToDo, vec!["a".to_string(), "d".to_string()]),
                (ColumnId::InProgress, vec!["b".to_string()]),
                (ColumnId::Done, vec!["c".to_string()]),
            ]
        );
    }

    #[test]
    fn target_markers_round_trip_through_the_columns() {
        for column in ColumnId::ALL {
            assert_eq!(column_for_marker(column.target_marker()), Some(column));
        }
    }

    #[test]
    fn page_labels_fall_back_to_the_file_stem() {
        let mut row = card("a", Some("TODO"), "journals/2026-11-13.md");
        assert_eq!(page_label(&row), "2026-11-13");
        assert!(is_journal(&row));
        row.page_title = Some("Today".to_string());
        assert_eq!(page_label(&row), "Today");
        assert!(!is_journal(&card("b", Some("TODO"), "pages/b.md")));
    }

    #[test]
    fn column_flags_parse_the_capture_values() {
        assert_eq!(ColumnId::from_env("todo"), Some(ColumnId::ToDo));
        assert_eq!(ColumnId::from_env("to_do"), Some(ColumnId::ToDo));
        assert_eq!(
            ColumnId::from_env("in_progress"),
            Some(ColumnId::InProgress)
        );
        assert_eq!(
            ColumnId::from_env("in-progress"),
            Some(ColumnId::InProgress)
        );
        assert_eq!(ColumnId::from_env("done"), Some(ColumnId::Done));
        assert_eq!(ColumnId::from_env("nope"), None);
    }
}
