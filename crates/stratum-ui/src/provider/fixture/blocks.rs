//! Deterministic fixture page generator.
//!
//! [`fixture_blocks`] produces the same block list for the same path and count
//! every time. The generator cycles through headings, tasks, wiki-links, tags,
//! fenced code, Mermaid, math, CJK/emoji, quotes and dividers, with a small
//! nested subtree so depth and collapse behaviour are exercised.

use stratum_editor::BlockDto;
use uuid::Uuid;

use crate::provider::journal_path;

/// Build a deterministic page of `count` fixture blocks.
///
/// UUIDs are v5 values derived from the page path and row index, so the same
/// call always returns the same blocks. The generator is shared by the fixture
/// provider and the window-free contract tests.
pub fn fixture_blocks(path: &str, count: usize) -> Vec<BlockDto> {
    let mut blocks = Vec::with_capacity(count);
    let mut last_at_depth: Vec<Option<String>> = Vec::new();
    for index in 0..count {
        let depth = match index % 9 {
            0 | 6 | 8 => 0,
            1 | 2 | 4 => 1,
            3 => 2,
            _ => 0,
        };
        let id = fixture_uuid(path, index);
        let parent_id = if depth == 0 {
            None
        } else {
            last_at_depth.get(depth - 1).cloned().flatten()
        };
        let left_id = last_at_depth.get(depth).cloned().flatten();
        let (content, marker, priority, heading_level) = fixture_content(index);
        blocks.push(BlockDto {
            id,
            content,
            parent_id,
            left_id,
            properties: Vec::new(),
            marker,
            priority,
            collapsed: false,
            heading_level,
        });
        if last_at_depth.len() <= depth {
            last_at_depth.resize(depth + 1, None);
        }
        last_at_depth[depth] = blocks.last().map(|block| block.id.clone());
        last_at_depth.truncate(depth + 1);
    }
    blocks
}

/// Build the deterministic five-block body for a fixture journal day.
///
/// Each date gets its own heading, planning line, task, meeting note and
/// reflection, so stacked journal captures show distinct day content instead of
/// repeating the generic page fixture. Rows form a flat `left_id` chain with no
/// parents, matching the journal's day layout.
pub fn fixture_journal_blocks(date: &str) -> Vec<BlockDto> {
    let path = journal_path(date);
    let rows = [
        (format!("Journal {date}"), None, None, Some(1)),
        (
            format!("Entry for {date} — planning the day."),
            None,
            None,
            None,
        ),
        (
            format!("TODO Review the notes for {date} #journal"),
            Some("TODO".to_string()),
            Some("A".to_string()),
            None,
        ),
        (format!("Met with the team on {date}."), None, None, None),
        (
            format!("Reflection on {date}: steady progress."),
            None,
            None,
            None,
        ),
    ];
    let mut blocks: Vec<BlockDto> = Vec::with_capacity(rows.len());
    for (index, (content, marker, priority, heading_level)) in rows.into_iter().enumerate() {
        blocks.push(BlockDto {
            id: fixture_uuid(&path, index),
            content,
            parent_id: None,
            left_id: blocks.last().map(|block| block.id.clone()),
            properties: Vec::new(),
            marker,
            priority,
            collapsed: false,
            heading_level,
        });
    }
    blocks
}

/// Deterministic v5 UUID for one fixture row.
fn fixture_uuid(path: &str, index: usize) -> String {
    Uuid::new_v5(&Uuid::NAMESPACE_OID, format!("{path}#{index}").as_bytes()).to_string()
}

/// Content and metadata for one fixture row.
fn fixture_content(index: usize) -> (String, Option<String>, Option<String>, Option<u8>) {
    match index % 10 {
        0 => ("Welcome to Stratum".to_string(), None, None, Some(1)),
        1 => (
            "Explore [[Project Notes]] and the reading list.".to_string(),
            None,
            None,
            None,
        ),
        2 => (
            "TODO Review the reading list #research".to_string(),
            Some("TODO".to_string()),
            Some("A".to_string()),
            None,
        ),
        3 => (
            "```rust\nfn main() {\n    println!(\"stratum\");\n}\n```".to_string(),
            None,
            None,
            None,
        ),
        4 => (
            "```mermaid\ngraph TD; A-->B;\n```".to_string(),
            None,
            None,
            None,
        ),
        5 => ("$$\\frac{1}{2}$$".to_string(), None, None, None),
        6 => (
            "你好，世界 🌍 — deterministic CJK and emoji.".to_string(),
            None,
            None,
            None,
        ),
        7 => (
            "> A quoted line from the fixture.".to_string(),
            None,
            None,
            None,
        ),
        8 => ("---".to_string(), None, None, None),
        _ => (
            format!("Row {index} links [[Note {index}]] and tag #inbox"),
            None,
            None,
            None,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_blocks_are_deterministic_and_nested() {
        let first = fixture_blocks("pages/Welcome.md", 40);
        let second = fixture_blocks("pages/Welcome.md", 40);
        assert_eq!(first, second);
        assert_eq!(first.len(), 40);
        assert!(first.iter().any(|block| block.parent_id.is_some()));
        assert!(first
            .iter()
            .any(|block| block.content.starts_with("```mermaid")));
        assert!(first.iter().any(|block| block.content.contains("你好")));
        assert!(first
            .iter()
            .any(|block| block.marker.as_deref() == Some("TODO")));
    }

    #[test]
    fn fixture_journal_blocks_are_date_specific_and_deterministic() {
        let first = fixture_journal_blocks("2026-11-09");
        let same = fixture_journal_blocks("2026-11-09");
        let other = fixture_journal_blocks("2026-11-08");
        assert_eq!(first, same);
        assert_ne!(first[0].content, other[0].content);
        assert!(first[0].content.contains("2026-11-09"));
        assert!(other[0].content.contains("2026-11-08"));
        assert_eq!(first.len(), 5);
        assert!(first.iter().all(|block| block.parent_id.is_none()));
    }
}
