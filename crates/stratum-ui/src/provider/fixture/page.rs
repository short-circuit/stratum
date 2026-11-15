//! Deterministic page-surface data for the fixture provider.
//!
//! The page panel needs page metadata, the Note menu actions, backlinks with a
//! snippet and two suggestions for `pages/fixture.md`. Keeping the bodies here
//! lets `fixture/mod.rs` stay under the repository file-length limit while the
//! single `ShellData` impl forwards to these inherent helpers.

use super::*;
use crate::provider::normalize_move_path;

impl FixtureProvider {
    /// Deterministic page metadata for an existing fixture page.
    pub(super) fn fixture_page_info(&self, path: &str) -> Option<PageInfo> {
        let block_count = self
            .pages
            .lock()
            .ok()
            .and_then(|pages| pages.get(path).map(Vec::len))? as u64;
        let (title, modified_at) = self
            .snapshot
            .recents
            .iter()
            .find(|page| page.path == path)
            .map(|page| (Some(page.title.clone()), page.modified_at.clone()))
            .unwrap_or((None, "2026-11-10T10:00:00Z".to_string()));
        Some(PageInfo {
            path: path.to_string(),
            title,
            slug: file_stem(path).to_string(),
            block_count,
            modified_at,
        })
    }

    /// Report one page as successfully reindexed.
    pub(super) fn fixture_reindex(&self, path: &str) -> Result<ReindexSummary, String> {
        if self.load_blocks(path).is_none() {
            return Err(format!("Page does not exist: {path}"));
        }
        Ok(ReindexSummary {
            succeeded: 1,
            failed: 0,
            errors: Vec::new(),
        })
    }

    /// Accept normalization for an existing page.
    pub(super) fn fixture_normalize(&self, path: &str) -> Result<(), String> {
        if self.load_blocks(path).is_some() {
            Ok(())
        } else {
            Err(format!("Page does not exist: {path}"))
        }
    }

    /// Move one fixture page, rejecting an existing target.
    pub(super) fn fixture_move(&self, old_path: &str, new_path: &str) -> Result<String, String> {
        let target = normalize_move_path(new_path);
        let mut pages = self
            .pages
            .lock()
            .map_err(|_| "fixture provider lock poisoned".to_string())?;
        if pages.contains_key(&target) && target != old_path {
            return Err(format!("Page already exists: {target}"));
        }
        let blocks = pages
            .remove(old_path)
            .ok_or_else(|| format!("Page does not exist: {old_path}"))?;
        pages.insert(target.clone(), blocks);
        Ok(target)
    }

    /// Remove one fixture page.
    pub(super) fn fixture_delete(&self, path: &str) -> Result<(), String> {
        self.pages
            .lock()
            .map_err(|_| "fixture provider lock poisoned".to_string())?
            .remove(path);
        Ok(())
    }

    /// Two linked references and one unlinked mention for the fixture page.
    pub(super) fn fixture_backlinks(&self, path: &str) -> Vec<BacklinkRow> {
        if path != FIXTURE_PAGE {
            return Vec::new();
        }
        vec![
            BacklinkRow {
                source_id: "22222222-2222-5222-8222-222222222222".to_string(),
                source_page: FIXTURE_BACKLINK_SOURCE.to_string(),
                context: "Project notes link to [[fixture]] from the planning row.".to_string(),
                is_linked: true,
            },
            BacklinkRow {
                source_id: "33333333-3333-5333-8333-333333333333".to_string(),
                source_page: "pages/Reading List.md".to_string(),
                context: "The reading list links to [[Fixture Note]] in a review note.".to_string(),
                is_linked: true,
            },
            BacklinkRow {
                source_id: "44444444-4444-5444-8444-444444444444".to_string(),
                source_page: "pages/Welcome.md".to_string(),
                context: "The fixture page is mentioned here without a wiki-link.".to_string(),
                is_linked: false,
            },
        ]
    }

    /// One bounded snippet for the fixture's linked backlink.
    pub(super) fn fixture_snippet(
        &self,
        note_id: &str,
        anchor_id: &str,
    ) -> Option<BacklinkSnippet> {
        if note_id.is_empty() || anchor_id.is_empty() {
            return None;
        }
        let note_title = self
            .snapshot
            .recents
            .iter()
            .find(|page| page.path == note_id)
            .map(|page| page.title.clone())
            .unwrap_or_else(|| file_stem(note_id).to_string());
        Some(BacklinkSnippet {
            note_title,
            anchor_content: "Project notes link to [[fixture]] from the planning row.".to_string(),
            context: vec![
                "Planning overview for the current sprint.".to_string(),
                "Project notes link to [[fixture]] from the planning row.".to_string(),
                "Follow-up items live in the reading list.".to_string(),
            ],
        })
    }

    /// Two deterministic related-note suggestions for the fixture page.
    pub(super) fn fixture_suggestions(&self, path: &str) -> Result<Vec<ConnectionRow>, String> {
        if path != FIXTURE_PAGE {
            return Ok(Vec::new());
        }
        Ok(vec![
            ConnectionRow {
                title: "Reading List".to_string(),
                page_path: "pages/Reading List.md".to_string(),
                score: 82.0,
                snippet: "A shared reading queue with notes on offline knowledge tools."
                    .to_string(),
            },
            ConnectionRow {
                title: "Project Notes".to_string(),
                page_path: "pages/Project Notes.md".to_string(),
                score: 64.0,
                snippet: "Sprint planning notes that reference the fixture page.".to_string(),
            },
        ])
    }

    /// Append one markdown block to a fixture page.
    pub(super) fn fixture_insert(&self, path: &str, markdown: &str) -> Result<(), String> {
        let mut pages = self
            .pages
            .lock()
            .map_err(|_| "fixture provider lock poisoned".to_string())?;
        let entry = pages.entry(path.to_string()).or_insert_with(Vec::new);
        let index = entry.len();
        let id = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("{path}#insert-{index}").as_bytes(),
        )
        .to_string();
        entry.push(BlockDto {
            id,
            content: markdown.to_string(),
            parent_id: None,
            left_id: entry.last().map(|block| block.id.clone()),
            properties: Vec::new(),
            marker: None,
            priority: None,
            collapsed: false,
            heading_level: None,
        });
        Ok(())
    }
}
