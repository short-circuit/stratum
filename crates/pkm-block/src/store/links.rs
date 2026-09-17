//! Implementation of the block store: link CRUD.

use super::BlockStore;
use super::StoreResult;
use crate::block::BlockId;
use pkm_core::PkmError;
use rusqlite::params;

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
}
