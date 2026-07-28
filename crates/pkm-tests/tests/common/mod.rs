//! Shared test helpers for Stratum PKM integration tests.
//!
//! Provides `create_test_vault()` which sets up a real temp vault
//! with filesystem, SQLite, and Tantivy index ready to use.
//! No mocks — everything is real files and databases.

#![allow(dead_code)]

use pkm_block::{Block, BlockStore, Page, TaskMarker};
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

/// A test vault fixture holding all resources.
/// The `TempDir` is kept alive for the lifetime of this struct.
pub struct TestVault {
    pub _dir: TempDir,
    pub vault_path: PathBuf,
    pub db_path: PathBuf,
    pub store: BlockStore,
    pub block_index: Option<pkm_index::block_search::BlockIndex>,
}

impl TestVault {
    #[allow(dead_code)]
    /// Create the index engine directory
    pub fn ensure_index_dir(&self) -> PathBuf {
        let p = self.vault_path.join(".pkm").join("search");
        std::fs::create_dir_all(&p).ok();
        p
    }

    #[allow(dead_code)]
    /// Get a BlockIndex, creating it if necessary.
    pub fn block_index(&mut self) -> &mut pkm_index::block_search::BlockIndex {
        let path = self.vault_path.join(".pkm").join("search");
        self.block_index
            .get_or_insert_with(|| pkm_index::block_search::BlockIndex::create(&path).unwrap());
        self.block_index.as_mut().unwrap()
    }

    /// Create a test .md file on disk.
    pub fn create_md_file(&self, rel_path: &str, content: &str) -> PathBuf {
        let full = self.vault_path.join(rel_path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&full, content).unwrap();
        full
    }

    /// Add a page to SQLite.
    pub fn add_page(&self, rel_path: &str) {
        let full = self.vault_path.join(rel_path);
        let page = Page::new(full, &self.vault_path);
        self.store.upsert_page(&page).unwrap();
    }

    /// Add a block to SQLite.
    pub fn add_block(&self, rel_path: &str, content: &str) -> Block {
        let id = Uuid::new_v4();
        let block = Block::new(id, content.to_string());
        self.store.insert_block(&block, rel_path).unwrap();
        block
    }

    /// Add a block with marker.
    pub fn add_block_with_marker(
        &self,
        rel_path: &str,
        content: &str,
        marker: TaskMarker,
    ) -> Block {
        let id = Uuid::new_v4();
        let block = Block::new(id, content.to_string()).with_marker(marker);
        self.store.insert_block(&block, rel_path).unwrap();
        block
    }

    #[allow(dead_code)]
    /// Add a link between blocks.
    pub fn add_link(&self, source: Uuid, target_page: &str) {
        self.store
            .insert_link(source, "page_ref", Some(target_page), None)
            .unwrap();
    }
}

/// Create a fully-realized test vault in a temp directory.
pub fn create_test_vault() -> TestVault {
    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().to_path_buf();

    std::fs::create_dir_all(vault_path.join(".pkm")).unwrap();
    std::fs::create_dir_all(vault_path.join("pages")).unwrap();
    std::fs::create_dir_all(vault_path.join("journals")).unwrap();
    std::fs::create_dir_all(vault_path.join("templates")).unwrap();

    let db_path = vault_path.join(".pkm").join("blocks.db");
    let store = BlockStore::open(&db_path).unwrap();

    let search_path = vault_path.join(".pkm").join("search");
    std::fs::create_dir_all(&search_path).unwrap();

    TestVault {
        _dir: dir,
        vault_path,
        db_path,
        store,
        block_index: None,
    }
}
