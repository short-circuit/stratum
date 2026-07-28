//! Shared test helpers for Tauri-level integration tests.
//!
//! Provides `create_test_vault()` which sets up a real temp vault
//! with filesystem, SQLite, and Tantivy index ready to use.

use pkm_block::{Block, BlockStore, Page, PageFrontmatter, Priority, TaskMarker};
use pkm_index::block_search::BlockIndex;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

/// A test vault fixture holding all resources.
/// The `TempDir` is kept alive for the lifetime of this struct.
pub struct TestVault {
    pub _dir: TempDir,
    pub vault_path: PathBuf,
    pub db_path: PathBuf,
    pub store: BlockStore,
    pub block_index: Option<BlockIndex>,
}

impl TestVault {
    /// Create the index engine directory (but don't create full IndexEngine
    /// since that requires the notes to already exist in the graph).
    pub fn ensure_index_dir(&self) -> PathBuf {
        let p = self.vault_path.join(".pkm").join("search");
        std::fs::create_dir_all(&p).ok();
        p
    }

    /// Get a BlockIndex, creating it if necessary.
    pub fn block_index(&mut self) -> &mut BlockIndex {
        let path = self.vault_path.join(".pkm").join("search");
        self.block_index
            .get_or_insert_with(|| BlockIndex::create(&path).unwrap());
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

    /// Add a block with marker to SQLite.
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

    /// Add a block with properties.
    pub fn add_block_with_props(
        &self,
        rel_path: &str,
        content: &str,
        props: &[(&str, &str)],
    ) -> Block {
        let id = Uuid::new_v4();
        let mut block = Block::new(id, content.to_string());
        for (k, v) in props {
            block = block.with_property(*k, *v);
        }
        self.store.insert_block(&block, rel_path).unwrap();
        block
    }

    /// Add a link between blocks in the store.
    pub fn add_link(&self, source: Uuid, target_page: &str) {
        self.store
            .insert_link(source, "page_ref", Some(target_page), None)
            .unwrap();
    }
}

/// Create a fully-realized test vault in a temp directory.
///
/// Returns a `TestVault` with:
/// - Real filesystem with `pages/`, `journals/`, `.pkm/` directories
/// - A real SQLite `BlockStore` at `.pkm/blocks.db`
/// - Pre-created `.pkm/search/` directory for Tantivy
pub fn create_test_vault() -> TestVault {
    let dir = TempDir::new().unwrap();
    let vault_path = dir.path().to_path_buf();

    // Create directory structure
    std::fs::create_dir_all(vault_path.join(".pkm")).unwrap();
    std::fs::create_dir_all(vault_path.join("pages")).unwrap();
    std::fs::create_dir_all(vault_path.join("journals")).unwrap();
    std::fs::create_dir_all(vault_path.join("templates")).unwrap();

    // Create SQLite store
    let db_path = vault_path.join(".pkm").join("blocks.db");
    let store = BlockStore::open(&db_path).unwrap();

    // Create Tantivy search directory
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
