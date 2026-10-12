//! SQLite-backed block store.
//!
//! The [`BlockStore`] struct and its connection/schema management live here;
//! block/page/link CRUD and query helpers are in sibling modules.

use pkm_core::PkmError;
use rusqlite::Connection;
use std::path::Path;

pub type StoreResult<T> = Result<T, PkmError>;

mod blocks;
mod links;
mod pages;
mod queries;

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
}

#[cfg(test)]
mod tests;
