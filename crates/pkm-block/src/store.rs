//! SQLite-backed storage for blocks and pages.
//!
//! Primary storage layer for the hybrid SQLite+.md architecture.
//! All CRUD operations happen against SQLite first; .md files are
//! written asynchronously for portability and git sync.

use crate::block::{Block, BlockId, BlockMeta, Priority, TaskMarker};
use crate::page::{Page, PageFrontmatter};
use chrono::{DateTime, Utc};
use pkm_core::PkmError;
use rusqlite::{params, params_from_iter, Connection};
use std::collections::BTreeMap;
use std::path::Path;
use uuid::Uuid;

pub type StoreResult<T> = Result<T, PkmError>;

pub struct BlockStore {
    conn: Connection,
}

impl BlockStore {
    /// Open or create the block store at the given path.
    pub fn open(path: &Path) -> StoreResult<Self> {
        let conn =
            Connection::open(path).map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    /// Open an in-memory store (for testing).
    pub fn open_in_memory() -> StoreResult<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    /// Execute a raw SQL batch (used for BEGIN/COMMIT/ROLLBACK wrapping).
    /// Enables callers to wrap multi-step operations in explicit transactions
    /// to prevent data loss on partial failure.
    pub fn execute_batch(&self, sql: &str) -> StoreResult<()> {
        self.conn
            .execute_batch(sql)
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))
    }

    fn init_schema(&self) -> StoreResult<()> {
        self.conn
            .execute_batch(
                "
            CREATE TABLE IF NOT EXISTS blocks (
                id TEXT PRIMARY KEY,
                page_path TEXT NOT NULL,
                content TEXT NOT NULL DEFAULT '',
                parent_id TEXT,
                left_id TEXT,
                properties TEXT NOT NULL DEFAULT '{}',
                marker TEXT,
                priority TEXT,
                collapsed INTEGER NOT NULL DEFAULT 0,
                heading_level INTEGER,
                created_at TEXT NOT NULL,
                modified_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_blocks_page ON blocks(page_path);
            CREATE INDEX IF NOT EXISTS idx_blocks_parent ON blocks(parent_id);
            CREATE INDEX IF NOT EXISTS idx_blocks_marker ON blocks(marker);

            CREATE TABLE IF NOT EXISTS pages (
                path TEXT PRIMARY KEY,
                title TEXT,
                frontmatter TEXT NOT NULL DEFAULT '{}',
                block_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                modified_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_block TEXT NOT NULL,
                link_type TEXT NOT NULL,
                target_page TEXT COLLATE NOCASE,
                target_block TEXT,
                FOREIGN KEY (source_block) REFERENCES blocks(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_links_source ON links(source_block);
            CREATE INDEX IF NOT EXISTS idx_links_target_page ON links(target_page);
            CREATE INDEX IF NOT EXISTS idx_links_target_block ON links(target_block);
            ",
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        self.conn
            .execute_batch(
                "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -64000;
            PRAGMA temp_store = MEMORY;
            PRAGMA mmap_size = 268435456;
            PRAGMA busy_timeout = 5000;
            ",
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(())
    }

    // --- Block CRUD ---

    pub fn insert_block(&self, block: &Block, page_path: &str) -> StoreResult<()> {
        let id = block.id.to_string();
        let properties = serde_json::to_string(&block.properties)?;
        let marker = block.marker.map(|m| m.as_str().to_string());
        let priority = block.priority.map(|p| p.as_str().to_string());
        let parent_id = block.parent_id.map(|p| p.to_string());
        let left_id = block.left_id.map(|l| l.to_string());
        let created_at = block.created_at.to_rfc3339();
        let modified_at = block.modified_at.to_rfc3339();

        self.conn
            .execute(
                "INSERT OR REPLACE INTO blocks (id, page_path, content, parent_id, left_id,
             properties, marker, priority, collapsed, heading_level, created_at, modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    id,
                    page_path,
                    block.content,
                    parent_id,
                    left_id,
                    properties,
                    marker,
                    priority,
                    block.meta.collapsed as i32,
                    block.meta.heading_level,
                    created_at,
                    modified_at,
                ],
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(())
    }

    pub fn get_block(&self, id: BlockId) -> StoreResult<Block> {
        let id_str = id.to_string();
        self.conn
            .query_row(
                "SELECT id, content, parent_id, left_id, properties, marker, priority,
                 collapsed, heading_level, created_at, modified_at
                 FROM blocks WHERE id = ?1",
                params![id_str],
                |row| {
                    let id: String = row.get(0)?;
                    let content: String = row.get(1)?;
                    let parent_id: Option<String> = row.get(2)?;
                    let left_id: Option<String> = row.get(3)?;
                    let properties_str: String = row.get(4)?;
                    let marker: Option<String> = row.get(5)?;
                    let priority: Option<String> = row.get(6)?;
                    let collapsed: bool = row.get::<_, i32>(7)? != 0;
                    let heading_level: Option<u8> = row.get(8)?;
                    let created_at: String = row.get(9)?;
                    let modified_at: String = row.get(10)?;

                    let id = Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::nil());
                    let properties: BTreeMap<String, String> =
                        serde_json::from_str(&properties_str).unwrap_or_default();
                    let marker = marker.and_then(|m| TaskMarker::parse(&m));
                    let priority = priority.and_then(|p| Priority::parse(&p));

                    Ok(Block {
                        id,
                        content,
                        parent_id: parent_id.and_then(|s| Uuid::parse_str(&s).ok()),
                        left_id: left_id.and_then(|s| Uuid::parse_str(&s).ok()),
                        properties,
                        marker,
                        priority,
                        meta: BlockMeta {
                            collapsed,
                            heading_level,
                        },
                        created_at: DateTime::parse_from_rfc3339(&created_at)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        modified_at: DateTime::parse_from_rfc3339(&modified_at)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => PkmError::BlockNotFound(format!("{}", id)),
                other => PkmError::Internal(format!("SQLite error: {other}")),
            })
    }

    /// Get a block by ID along with its page_path.
    /// Returns (Block, page_path) or an error if the block is not found.
    pub fn get_block_with_page_path(&self, id: BlockId) -> StoreResult<(Block, String)> {
        let id_str = id.to_string();
        self.conn
            .query_row(
                "SELECT id, content, parent_id, left_id, properties, marker, priority,
                 collapsed, heading_level, created_at, modified_at, page_path
                 FROM blocks WHERE id = ?1",
                params![id_str],
                |row| {
                    let id: String = row.get(0)?;
                    let content: String = row.get(1)?;
                    let parent_id: Option<String> = row.get(2)?;
                    let left_id: Option<String> = row.get(3)?;
                    let properties_str: String = row.get(4)?;
                    let marker: Option<String> = row.get(5)?;
                    let priority: Option<String> = row.get(6)?;
                    let collapsed: bool = row.get::<_, i32>(7)? != 0;
                    let heading_level: Option<u8> = row.get(8)?;
                    let created_at: String = row.get(9)?;
                    let modified_at: String = row.get(10)?;
                    let page_path: String = row.get(11)?;

                    let id = Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::nil());
                    let properties: BTreeMap<String, String> =
                        serde_json::from_str(&properties_str).unwrap_or_default();
                    let marker = marker.and_then(|m| TaskMarker::parse(&m));
                    let priority = priority.and_then(|p| Priority::parse(&p));

                    Ok((
                        Block {
                            id,
                            content,
                            parent_id: parent_id.and_then(|s| Uuid::parse_str(&s).ok()),
                            left_id: left_id.and_then(|s| Uuid::parse_str(&s).ok()),
                            properties,
                            marker,
                            priority,
                            meta: BlockMeta {
                                collapsed,
                                heading_level,
                            },
                            created_at: DateTime::parse_from_rfc3339(&created_at)
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(|_| Utc::now()),
                            modified_at: DateTime::parse_from_rfc3339(&modified_at)
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(|_| Utc::now()),
                        },
                        page_path,
                    ))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => PkmError::BlockNotFound(format!("{}", id)),
                other => PkmError::Internal(format!("SQLite error: {other}")),
            })
    }

    pub fn get_blocks_by_page(&self, page_path: &str) -> StoreResult<Vec<Block>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, content, parent_id, left_id, properties, marker, priority,
                 collapsed, heading_level, created_at, modified_at
                 FROM blocks WHERE page_path = ?1 ORDER BY rowid",
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        let blocks: Vec<Block> = stmt
            .query_map(params![page_path], |row| {
                let id: String = row.get(0)?;
                let content: String = row.get(1)?;
                let parent_id: Option<String> = row.get(2)?;
                let left_id: Option<String> = row.get(3)?;
                let properties_str: String = row.get(4)?;
                let marker: Option<String> = row.get(5)?;
                let priority: Option<String> = row.get(6)?;
                let collapsed: bool = row.get::<_, i32>(7)? != 0;
                let heading_level: Option<u8> = row.get(8)?;
                let created_at: String = row.get(9)?;
                let modified_at: String = row.get(10)?;

                let id = Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::nil());
                let properties: BTreeMap<String, String> =
                    serde_json::from_str(&properties_str).unwrap_or_default();
                let marker = marker.and_then(|m| TaskMarker::parse(&m));
                let priority = priority.and_then(|p| Priority::parse(&p));

                Ok(Block {
                    id,
                    content,
                    parent_id: parent_id.and_then(|s| Uuid::parse_str(&s).ok()),
                    left_id: left_id.and_then(|s| Uuid::parse_str(&s).ok()),
                    properties,
                    marker,
                    priority,
                    meta: BlockMeta {
                        collapsed,
                        heading_level,
                    },
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    modified_at: DateTime::parse_from_rfc3339(&modified_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(blocks)
    }

    pub fn update_block(&self, block: &Block) -> StoreResult<()> {
        let id = block.id.to_string();
        let properties = serde_json::to_string(&block.properties)?;
        let marker = block.marker.map(|m| m.as_str().to_string());
        let priority = block.priority.map(|p| p.as_str().to_string());
        let parent_id = block.parent_id.map(|p| p.to_string());
        let left_id = block.left_id.map(|l| l.to_string());
        let modified_at = block.modified_at.to_rfc3339();

        self.conn
            .execute(
                "UPDATE blocks SET content = ?2, parent_id = ?3, left_id = ?4,
             properties = ?5, marker = ?6, priority = ?7, collapsed = ?8,
             heading_level = ?9, modified_at = ?10
             WHERE id = ?1",
                params![
                    id,
                    block.content,
                    parent_id,
                    left_id,
                    properties,
                    marker,
                    priority,
                    block.meta.collapsed as i32,
                    block.meta.heading_level,
                    modified_at,
                ],
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(())
    }

    pub fn delete_block(&self, id: BlockId) -> StoreResult<()> {
        let id_str = id.to_string();
        let affected = self
            .conn
            .execute("DELETE FROM blocks WHERE id = ?1", params![id_str])
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        if affected == 0 {
            return Err(PkmError::BlockNotFound(format!("{}", id)));
        }
        Ok(())
    }

    pub fn delete_blocks_by_page(&self, page_path: &str) -> StoreResult<usize> {
        let count = self
            .conn
            .execute(
                "DELETE FROM blocks WHERE page_path = ?1",
                params![page_path],
            )
            .map_err(|e| PkmError::Internal(format!("SQLite error: {e}")))?;
        Ok(count)
    }

    // --- Page CRUD ---

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

    // --- Link CRUD ---

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

    // --- Query helpers ---

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use uuid::Uuid;

    #[test]
    fn test_open_in_memory() {
        let store = BlockStore::open_in_memory().unwrap();
        assert_eq!(store.block_count().unwrap(), 0);
    }

    #[test]
    fn test_open_disk() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.db");
        let store = BlockStore::open(&path).unwrap();
        assert_eq!(store.page_count().unwrap(), 0);
    }

    #[test]
    fn test_insert_and_get_block() {
        let store = BlockStore::open_in_memory().unwrap();
        let id = Uuid::new_v4();
        let block = Block::new(id, "Hello world".into())
            .with_marker(TaskMarker::Todo)
            .with_priority(Priority::A);

        store.insert_block(&block, "pages/test.md").unwrap();

        let retrieved = store.get_block(id).unwrap();
        assert_eq!(retrieved.content, "Hello world");
        assert_eq!(retrieved.marker, Some(TaskMarker::Todo));
        assert_eq!(retrieved.priority, Some(Priority::A));
    }

    #[test]
    fn test_update_block() {
        let store = BlockStore::open_in_memory().unwrap();
        let id = Uuid::new_v4();
        let mut block = Block::new(id, "Original".into());

        store.insert_block(&block, "pages/test.md").unwrap();

        block.content = "Updated".into();
        block.marker = Some(TaskMarker::Done);
        store.update_block(&block).unwrap();

        let retrieved = store.get_block(id).unwrap();
        assert_eq!(retrieved.content, "Updated");
        assert_eq!(retrieved.marker, Some(TaskMarker::Done));
    }

    #[test]
    fn test_delete_block() {
        let store = BlockStore::open_in_memory().unwrap();
        let id = Uuid::new_v4();
        let block = Block::new(id, "Delete me".into());

        store.insert_block(&block, "pages/test.md").unwrap();
        assert_eq!(store.block_count().unwrap(), 1);

        store.delete_block(id).unwrap();
        assert_eq!(store.block_count().unwrap(), 0);

        // Getting deleted block should error
        assert!(store.get_block(id).is_err());
    }

    #[test]
    fn test_get_blocks_by_page() {
        let store = BlockStore::open_in_memory().unwrap();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        store
            .insert_block(&Block::new(a, "A".into()), "p/a.md")
            .unwrap();
        store
            .insert_block(&Block::new(b, "B".into()), "p/a.md")
            .unwrap();
        store
            .insert_block(&Block::new(c, "C".into()), "p/b.md")
            .unwrap();

        let page_a = store.get_blocks_by_page("p/a.md").unwrap();
        assert_eq!(page_a.len(), 2);
    }

    #[test]
    fn test_upsert_and_get_page() {
        let store = BlockStore::open_in_memory().unwrap();
        let page = Page::new("pages/test.md".into(), std::path::Path::new("/vault"));

        store.upsert_page(&page).unwrap();

        let fm = store.get_page("pages/test.md").unwrap();
        assert!(fm.is_some());

        let pages = store.list_pages().unwrap();
        assert_eq!(pages.len(), 1);
    }

    #[test]
    fn test_insert_and_get_links() {
        let store = BlockStore::open_in_memory().unwrap();
        let source = Uuid::new_v4();
        let target = Uuid::new_v4();

        store
            .insert_block(&Block::new(source, "Source".into()), "pages/src.md")
            .unwrap();
        store
            .insert_block(&Block::new(target, "Target".into()), "pages/tgt.md")
            .unwrap();

        store
            .insert_link(source, "block_ref", None, Some(target))
            .unwrap();
        store
            .insert_link(source, "page_ref", Some("Target Page"), None)
            .unwrap();

        let block_backlinks = store.get_backlinks_for_block(target).unwrap();
        assert_eq!(block_backlinks.len(), 1);
        assert_eq!(block_backlinks[0], source.to_string());

        let page_backlinks = store.get_backlinks_for_page("Target Page").unwrap();
        assert_eq!(page_backlinks.len(), 1);
    }

    #[test]
    fn test_find_blocks_by_marker() {
        let store = BlockStore::open_in_memory().unwrap();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        store
            .insert_block(
                &Block::new(a, "Task 1".into()).with_marker(TaskMarker::Todo),
                "pages/tasks.md",
            )
            .unwrap();
        store
            .insert_block(
                &Block::new(b, "Task 2".into()).with_marker(TaskMarker::Done),
                "pages/tasks.md",
            )
            .unwrap();

        let todos = store.find_blocks_by_marker("TODO").unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].id, a);
    }

    #[test]
    fn test_find_blocks_by_markers() {
        let store = BlockStore::open_in_memory().unwrap();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        store
            .insert_block(
                &Block::new(a, "Task 1".into()).with_marker(TaskMarker::Todo),
                "pages/tasks.md",
            )
            .unwrap();
        store
            .insert_block(
                &Block::new(b, "Task 2".into()).with_marker(TaskMarker::Doing),
                "pages/tasks.md",
            )
            .unwrap();
        store
            .insert_block(
                &Block::new(c, "Task 3".into()).with_marker(TaskMarker::Done),
                "pages/archive.md",
            )
            .unwrap();

        // Query for ["TODO", "DOING"] — returns 2 results with correct page_path
        let results = store.find_blocks_by_markers(&["TODO", "DOING"]).unwrap();
        assert_eq!(results.len(), 2);
        for (block, page_path) in &results {
            assert_eq!(page_path.as_str(), "pages/tasks.md");
            assert!(
                block.marker == Some(TaskMarker::Todo) || block.marker == Some(TaskMarker::Doing)
            );
        }
        let result_ids: Vec<BlockId> = results.iter().map(|(b, _)| b.id).collect();
        assert!(result_ids.contains(&a));
        assert!(result_ids.contains(&b));
        assert!(!result_ids.contains(&c));

        // Query for ["NOW"] — returns empty
        let empty = store.find_blocks_by_markers(&["NOW"]).unwrap();
        assert_eq!(empty.len(), 0);

        // Query with empty markers — returns empty
        let empty2 = store.find_blocks_by_markers(&[]).unwrap();
        assert_eq!(empty2.len(), 0);
    }

    #[test]
    fn test_delete_blocks_by_page() {
        let store = BlockStore::open_in_memory().unwrap();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        store
            .insert_block(&Block::new(a, "A".into()), "pages/to_delete.md")
            .unwrap();
        store
            .insert_block(&Block::new(b, "B".into()), "pages/keep.md")
            .unwrap();

        let deleted = store.delete_blocks_by_page("pages/to_delete.md").unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(store.block_count().unwrap(), 1);
    }
}
