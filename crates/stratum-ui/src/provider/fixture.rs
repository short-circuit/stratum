//! Deterministic fixture provider and page generator.
//!
//! [`FixtureProvider`] backs tests, `STRATUM_FIXTURE=1` and the 10k perf
//! capture. Pages are stored behind a mutex so the editor can save and reload
//! them, and [`fixture_blocks`] produces the same block list for the same path
//! and count every time.
//!
//! `STRATUM_FIXTURE_BLOCKS` controls the Welcome page size; the perf binary
//! selects the size explicitly through [`FixtureProvider::with_welcome_blocks`].

use std::collections::HashMap;
use std::sync::Mutex;

use stratum_editor::menus::{AiCall, AiCallKind};
use stratum_editor::BlockDto;
use uuid::Uuid;

use super::{derive_recents, RecentPage, ShellData, ShellSnapshot, VaultStats};

/// Deterministic provider used by tests and `STRATUM_FIXTURE=1`.
pub struct FixtureProvider {
    snapshot: ShellSnapshot,
    export_pages: u64,
    pages: Mutex<HashMap<String, Vec<BlockDto>>>,
}

impl FixtureProvider {
    /// Create the deterministic fixture snapshot.
    ///
    /// The raw list intentionally contains a journal page so the fixture
    /// exercises the same filtering and ordering as the real provider.
    /// `STRATUM_FIXTURE_BLOCKS` controls the Welcome page size: 10 000 selects
    /// the perf-capture document, anything else uses the small ~40-block page.
    pub fn new() -> Self {
        let welcome_blocks = std::env::var("STRATUM_FIXTURE_BLOCKS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|count| *count > 0)
            .unwrap_or(40);
        Self::with_welcome_blocks(welcome_blocks)
    }

    /// Create a fixture whose Welcome page has exactly `welcome_blocks` rows.
    ///
    /// The perf capture uses this to build a 10 000-block document without
    /// depending on process environment variables.
    pub fn with_welcome_blocks(welcome_blocks: usize) -> Self {
        let welcome_blocks = welcome_blocks.max(1);
        let mut pages = HashMap::new();
        pages.insert(
            "pages/Welcome.md".to_string(),
            fixture_blocks("pages/Welcome.md", welcome_blocks),
        );
        pages.insert(
            "pages/Project Notes.md".to_string(),
            fixture_blocks("pages/Project Notes.md", 34),
        );
        pages.insert(
            "pages/Reading List.md".to_string(),
            fixture_blocks("pages/Reading List.md", 7),
        );
        let raw = vec![
            RecentPage {
                path: "journals/2026-11-09.md".to_string(),
                title: "2026-11-09".to_string(),
                block_count: 5,
                modified_at: "2026-11-09T09:00:00Z".to_string(),
            },
            RecentPage {
                path: "pages/Welcome.md".to_string(),
                title: "Welcome".to_string(),
                block_count: welcome_blocks as u64,
                modified_at: "2026-11-09T08:00:00Z".to_string(),
            },
            RecentPage {
                path: "pages/Project Notes.md".to_string(),
                title: "Project Notes".to_string(),
                block_count: 34,
                modified_at: "2026-11-08T18:00:00Z".to_string(),
            },
            RecentPage {
                path: "pages/Reading List.md".to_string(),
                title: "Reading List".to_string(),
                block_count: 7,
                modified_at: "2026-11-07T12:00:00Z".to_string(),
            },
        ];
        Self {
            snapshot: ShellSnapshot {
                vault: Some(VaultStats {
                    block_count: 1240,
                    page_count: 87,
                }),
                recents: derive_recents(raw),
            },
            export_pages: 87,
            pages: Mutex::new(pages),
        }
    }
}

impl Default for FixtureProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellData for FixtureProvider {
    fn snapshot(&self) -> ShellSnapshot {
        self.snapshot.clone()
    }

    fn refresh(&self) -> ShellSnapshot {
        self.snapshot.clone()
    }

    fn export_html(&self) -> Result<u64, String> {
        Ok(self.export_pages)
    }

    fn load_blocks(&self, path: &str) -> Option<Vec<BlockDto>> {
        self.pages
            .lock()
            .ok()
            .and_then(|pages| pages.get(path).cloned())
    }

    fn save_blocks(&self, path: &str, blocks: Vec<BlockDto>) -> Result<(), String> {
        self.pages
            .lock()
            .map_err(|_| "fixture provider lock poisoned".to_string())?
            .insert(path.to_string(), blocks);
        Ok(())
    }

    fn ai_run(&self, call: AiCall) -> Result<String, String> {
        Ok(match call.kind {
            AiCallKind::Mermaid => "graph TD\n  A[Start] --> B[End]".to_string(),
            AiCallKind::Transform(_) | AiCallKind::Research | AiCallKind::Interlink => call.text,
        })
    }
}

/// Build a deterministic page of `count` fixture blocks.
///
/// The generator is shared by [`FixtureProvider`] and the window-free contract
/// tests. Blocks cycle through headings, tasks, wiki-links, tags, fenced code,
/// Mermaid, math, CJK/emoji, quotes and dividers, with a small nested subtree so
/// depth and collapse behaviour are exercised. UUIDs are v5 values derived from
/// the page path and row index, so the same call always returns the same blocks.
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
    use stratum_editor::menus::{AiCallScope, AiTransformAction};

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
    fn welcome_size_is_selectable() {
        let provider = FixtureProvider::with_welcome_blocks(10_000);
        let blocks = provider.load_blocks("pages/Welcome.md").expect("page");
        assert_eq!(blocks.len(), 10_000);
    }

    #[test]
    fn fixture_ai_run_is_deterministic() {
        let provider = FixtureProvider::new();
        let transform = AiCall {
            kind: AiCallKind::Transform(AiTransformAction::Rewrite),
            scope: AiCallScope::InsertAtCaret,
            text: "hello".into(),
            page_path: None,
        };
        assert_eq!(provider.ai_run(transform), Ok("hello".to_string()));
        let research = AiCall {
            kind: AiCallKind::Research,
            scope: AiCallScope::ReplacePage,
            text: "topic".into(),
            page_path: None,
        };
        assert_eq!(provider.ai_run(research), Ok("topic".to_string()));
        let interlink = AiCall {
            kind: AiCallKind::Interlink,
            scope: AiCallScope::ReplacePage,
            text: "body".into(),
            page_path: Some("pages/x.md".into()),
        };
        assert_eq!(provider.ai_run(interlink), Ok("body".to_string()));
        let mermaid = AiCall {
            kind: AiCallKind::Mermaid,
            scope: AiCallScope::InsertBlockAfterCaret,
            text: "prompt".into(),
            page_path: None,
        };
        assert_eq!(
            provider.ai_run(mermaid),
            Ok("graph TD\n  A[Start] --> B[End]".to_string())
        );
    }
}
