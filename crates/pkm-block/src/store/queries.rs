//! Implementation of the block store: query helpers.

use super::BlockStore;
use super::StoreResult;
use crate::block::Block;
use pkm_core::PkmError;
use rusqlite::{params, params_from_iter};
use uuid::Uuid;

impl BlockStore {
    pub fn find_blocks_by_marker(&self, marker: &str) -> StoreResult<Vec<Block>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM blocks WHERE LOWER(marker) = LOWER(?1)")
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let ids: Vec<String> = stmt
            .query_map(params![marker], |row| row.get(0))
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?
            .filter_map(|r| r.ok())
            .collect();

        let mut blocks = Vec::new();
        for id_str in ids {
            if let Ok(id) = Uuid::parse_str(&id_str) {
                if let Ok(block) = self.get_block(id) {
                    blocks.push(block);
                }
            }
        }
        Ok(blocks)
    }

    /// Find blocks matching any of the given markers, returning each block
    /// paired with its page_path. Returns empty vec for empty markers slice.
    pub fn find_blocks_by_markers(&self, markers: &[&str]) -> StoreResult<Vec<(Block, String)>> {
        if markers.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders: Vec<String> = markers.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT id, page_path FROM blocks WHERE LOWER(marker) IN ({})",
            placeholders.join(", ")
        );

        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let params: Vec<String> = markers.iter().map(|m| m.to_lowercase()).collect();

        let rows: Vec<(String, String)> = stmt
            .query_map(params_from_iter(params.iter()), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?
            .filter_map(|r| r.ok())
            .collect();

        let mut results = Vec::new();
        for (id_str, page_path) in rows {
            if let Ok(id) = Uuid::parse_str(&id_str) {
                if let Ok(block) = self.get_block(id) {
                    results.push((block, page_path));
                }
            }
        }
        Ok(results)
    }

    pub fn block_count(&self) -> StoreResult<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM blocks", [], |row| row.get(0))
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(count as usize)
    }

    pub fn page_count(&self) -> StoreResult<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM pages", [], |row| row.get(0))
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(count as usize)
    }

    /// Get counts of incoming links per target page, ordered by count descending.
    /// Returns (target_page, count) pairs.
    pub fn get_backlink_counts(&self) -> StoreResult<Vec<(String, i64)>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT target_page, COUNT(*) as cnt \
                 FROM links \
                 WHERE target_page IS NOT NULL \
                 GROUP BY target_page \
                 ORDER BY cnt DESC",
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?);
        }
        Ok(results)
    }
}
