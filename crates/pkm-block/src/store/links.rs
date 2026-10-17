//! Implementation of the block store: link CRUD.

use super::BlockStore;
use super::StoreResult;
use crate::block::BlockId;
use pkm_core::PkmError;
use rusqlite::{params, params_from_iter};

impl BlockStore {
    pub fn insert_link(
        &self,
        source_block: BlockId,
        link_type: &str,
        target_page: Option<&str>,
        target_block: Option<BlockId>,
    ) -> StoreResult<()> {
        let source = source_block.to_string();
        let target_b = target_block.map(|b| b.to_string());

        self.conn
            .execute(
                "INSERT INTO links (source_block, link_type, target_page, target_block)
             VALUES (?1, ?2, ?3, ?4)",
                params![source, link_type, target_page, target_b],
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(())
    }

    pub fn get_backlinks_for_block(&self, target: BlockId) -> StoreResult<Vec<String>> {
        let target_str = target.to_string();
        let mut stmt = self
            .conn
            .prepare("SELECT source_block FROM links WHERE target_block = ?1")
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let sources: Vec<String> = stmt
            .query_map(params![target_str], |row| row.get(0))
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(sources)
    }

    pub fn get_backlinks_for_page(&self, target_page: &str) -> StoreResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT source_block FROM links WHERE target_page = ?1")
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let sources: Vec<String> = stmt
            .query_map(params![target_page], |row| row.get(0))
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(sources)
    }

    /// Return the distinct page paths (via their block rows) that contain at least
    /// one `page_ref` link whose target matches any of `targets`.
    ///
    /// Used by the page-move flow to find every page that must have its inbound
    /// wiki-links rewritten when a page is renamed or relocated. The `target_page`
    /// column is `COLLATE NOCASE`, so matching against the supplied identities is
    /// case-insensitive for free.
    pub fn get_source_pages_for_targets(&self, targets: &[String]) -> StoreResult<Vec<String>> {
        if targets.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = targets.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT DISTINCT sp.page_path FROM links l \
             JOIN blocks sp ON l.source_block = sp.id \
             WHERE l.link_type = 'page_ref' AND l.target_page IN ({})",
            placeholders.join(", ")
        );
        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let rows = stmt
            .query_map(params_from_iter(targets.iter()), |row| row.get(0))
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let mut sources = Vec::new();
        for row in rows {
            sources.push(row.map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?);
        }
        Ok(sources)
    }

    pub fn delete_links_for_block(&self, block_id: BlockId) -> StoreResult<()> {
        let id = block_id.to_string();
        self.conn
            .execute(
                "DELETE FROM links WHERE source_block = ?1 OR target_block = ?1",
                params![id],
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(())
    }

    /// Delete every link row whose source block belongs to the given page.
    ///
    /// Used to reconcile the `links` table when a page's blocks are rewritten (save or
    /// sync), so stale backlinks never survive a write. The `links` table drives
    /// block-level backlink queries and was previously populated only by tests, which
    /// left production backlinks permanently empty.
    pub fn delete_links_for_page(&self, page_path: &str) -> StoreResult<()> {
        self.conn
            .execute(
                "DELETE FROM links WHERE source_block IN \
                 (SELECT id FROM blocks WHERE page_path = ?1)",
                params![page_path],
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(())
    }

    pub fn resolve_link_target_path(&self, target: &str) -> Option<String> {
        let slugified = target.replace(' ', "-").to_lowercase();
        let lower = target.to_lowercase();
        let paths = self.list_pages().ok()?;

        for path in &paths {
            // Exact path match
            if path == target || path.as_str() == lower {
                return Some(path.clone());
            }
            let slug = std::path::Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(path)
                .to_string();
            let slug_lower = slug.replace(' ', "-").to_lowercase();
            if slug_lower == slugified || slug_lower == lower {
                return Some(path.clone());
            }
            // Title match (frontmatter title or slug-derived display name)
            if let Ok(Some(fm)) = self.get_page(path) {
                if let Some(ref t) = fm.title {
                    if t == target
                        || t.to_lowercase() == lower
                        || t.replace(' ', "-").to_lowercase() == slugified
                    {
                        return Some(path.clone());
                    }
                }
            }
            let display = slug.replace('-', " ");
            if display.to_lowercase() == lower || display == target {
                return Some(path.clone());
            }
        }
        None
    }
}
