//! Implementation of the block store: page CRUD.

use super::BlockStore;
use super::StoreResult;
use crate::page::{Page, PageFrontmatter};
use pkm_core::PkmError;
use rusqlite::{params, params_from_iter};
use std::collections::HashMap;

impl BlockStore {
    pub fn upsert_page(&self, page: &Page) -> StoreResult<()> {
        let path = page.rel_path.to_string_lossy().to_string();
        let title = page.frontmatter.title.clone();
        let frontmatter = serde_json::to_string(&page.frontmatter)?;
        let created_at = page.modified_at.to_rfc3339();
        let modified_at = page.modified_at.to_rfc3339();

        self.conn.execute(
            "INSERT OR REPLACE INTO pages (path, title, frontmatter, block_count, created_at, modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                path,
                title,
                frontmatter,
                page.block_count() as i64,
                created_at,
                modified_at,
            ],
        )
        .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(())
    }

    pub fn get_page(&self, path: &str) -> StoreResult<Option<PageFrontmatter>> {
        let result = self.conn.query_row(
            "SELECT frontmatter FROM pages WHERE path = ?1",
            params![path],
            |row| {
                let fm_str: String = row.get(0)?;
                Ok(fm_str)
            },
        );

        match result {
            Ok(fm_str) => {
                let fm: PageFrontmatter = serde_json::from_str(&fm_str)
                    .map_err(|e| PkmError::Serialization(e.to_string()))?;
                Ok(Some(fm))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(PkmError::Internal(format!("SQLite error: {e}"))),
        }
    }

    /// Batch version of `get_page`. Returns a map from path to frontmatter for all
    /// requested paths in a single query.
    pub fn get_pages(&self, paths: &[String]) -> StoreResult<HashMap<String, PageFrontmatter>> {
        if paths.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders: Vec<String> = paths.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT path, frontmatter FROM pages WHERE path IN ({})",
            placeholders.join(", ")
        );

        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;

        let rows = stmt
            .query_map(params_from_iter(paths.iter()), |row| {
                let path: String = row.get(0)?;
                let fm_str: String = row.get(1)?;
                Ok((path, fm_str))
            })
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;

        let mut result = HashMap::new();
        for row in rows {
            let (path, fm_str) =
                row.map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
            let fm: PageFrontmatter = serde_json::from_str(&fm_str)
                .map_err(|e| PkmError::Serialization(e.to_string()))?;
            result.insert(path, fm);
        }
        Ok(result)
    }

    pub fn list_pages(&self) -> StoreResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM pages ORDER BY modified_at DESC")
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let paths: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(paths)
    }

    pub fn delete_page(&self, path: &str) -> StoreResult<()> {
        self.delete_blocks_by_page(path)?;
        self.conn
            .execute("DELETE FROM pages WHERE path = ?1", params![path])
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(())
    }

    /// Read the `modified_at` TEXT column for a page row. Returns `None` when the
    /// page is not registered (or the value is NULL). Used to detect whether a
    /// file on disk is newer than the database's record during drift repair.
    pub fn get_page_modified_at(&self, path: &str) -> StoreResult<Option<String>> {
        let result = self.conn.query_row(
            "SELECT modified_at FROM pages WHERE path = ?1",
            params![path],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(PkmError::Internal(format!("SQLite error: {e}"))),
        }
    }
}
