//! Regression tests for the journal file-exists DB-drift bug.
//!
//! Reproduces the DB-drift class (see `stratum-development` skill,
//! `references/db-drift-repair.md`): a stale `blocks.db` whose journal page is
//! missing/unregistered while the `.md` file exists on disk. The journal read
//! path must converge from disk so the frontend derives `todayExists=true` from
//! `list_pages` and the editor renders the on-disk journal content without an app
//! restart.
//!
//! Methodology (per the bug card): each test builds a throwaway COPY of a vault —
//! the SQLite file set (blocks.db + WAL + SHM if present) plus the `journals/`
//! directory — inside a `tempfile::TempDir`, runs the exact command-layer logic
//! against the copy, and asserts the post-condition. The source (working) vault is
//! only read; the real vault is never mutated.

mod common;

use common::create_test_vault;

/// Copy the SQLite file set of a vault (the DB-drift repro starts from a copy of
/// blocks.db + WAL + SHM, since WAL mode keeps recent pages in the WAL). Files that
/// do not exist (e.g. `-wal`/`-shm`, which SQLite removes on a clean close) are
/// skipped; the trio is copied exactly as present so the copy is a faithful stale
/// snapshot.
fn copy_db_trio(src_vault: &std::path::Path, dst_vault: &std::path::Path) {
    std::fs::create_dir_all(dst_vault.join(".pkm")).unwrap();
    for name in ["blocks.db", "blocks.db-wal", "blocks.db-shm"] {
        let src = src_vault.join(".pkm").join(name);
        if src.exists() {
            std::fs::copy(&src, dst_vault.join(".pkm").join(name)).unwrap();
        }
    }
}

// ---------------------------------------------------------------------------
// JN regression — `ensure_today_journal` file-exists branch converges from disk
// ---------------------------------------------------------------------------

/// Regression: when today's journal file EXISTS on disk but its page is NOT in
/// SQLite (the stale/imported blocks.db shape), `ensure_today_journal_core` must
/// re-sync the page + blocks from disk and return a `PageDto` reflecting the
/// on-disk journal, so the frontend's `todayExists` (derived from `list_pages`)
/// becomes true and the editor renders the real content.
///
/// Against the pre-4a7b980 code this test FAILED: the file-exists branch returned
/// a `PageDto` straight from SQLite (page unregistered, block_count 0) and left
/// the journal invisible/empty until an app restart ran `sync_filesystem_to_db`.
#[test]
fn ensure_today_journal_file_exists_syncs_stale_db_from_disk() {
    use app_lib::commands::page::ensure_today_journal_core;

    // Source vault: a real on-disk journal plus one unrelated registered page so
    // the copied DB is a non-trivial stale vault that simply lacks the journal.
    let src = create_test_vault();
    src.create_md_file(
        "journals/2026-09-06.md",
        "---\ntitle: 2026-09-06\n---\n- on-disk entry\n",
    );
    src.add_page("pages/other.md");
    drop(src.store); // flush: checkpoint the WAL into blocks.db before copying.

    // Copy the DB trio + journals/ into a fresh temp vault.
    let copy = create_test_vault();
    copy_db_trio(&src.vault_path, &copy.vault_path);
    std::fs::create_dir_all(copy.vault_path.join("journals")).unwrap();
    std::fs::copy(
        src.vault_path.join("journals/2026-09-06.md"),
        copy.vault_path.join("journals/2026-09-06.md"),
    )
    .unwrap();
    drop(copy.store); // release the freshly-created empty handle before reopening.

    // Reopen the copied (stale) store — the file-exists branch runs against it.
    let store = pkm_block::BlockStore::open(&copy.db_path).unwrap();
    let page = ensure_today_journal_core(&store, &copy.vault_path, "2026-09-06").unwrap();

    // The page must now be registered in SQLite (`todayExists` is derived from
    // `list_pages`), and the returned PageDto must carry the on-disk block.
    let pages = store.list_pages().unwrap();
    assert!(
        pages.contains(&"journals/2026-09-06.md".to_string()),
        "file-exists branch must register the journal page so todayExists=true"
    );
    assert_eq!(page.block_count, 1, "PageDto must reflect on-disk content");
    let blocks = store.get_blocks_by_page("journals/2026-09-06.md").unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].content, "on-disk entry");
}

/// Regression: the file-exists convergence must be idempotent — a second call
/// once the page is already in SQLite must not duplicate blocks or change content.
#[test]
fn ensure_today_journal_file_exists_converges_idempotently() {
    use app_lib::commands::page::ensure_today_journal_core;

    let src = create_test_vault();
    src.create_md_file(
        "journals/2026-09-06.md",
        "---\ntitle: 2026-09-06\n---\n- on-disk entry\n",
    );
    src.add_page("pages/other.md");
    drop(src.store);

    let copy = create_test_vault();
    copy_db_trio(&src.vault_path, &copy.vault_path);
    std::fs::create_dir_all(copy.vault_path.join("journals")).unwrap();
    std::fs::copy(
        src.vault_path.join("journals/2026-09-06.md"),
        copy.vault_path.join("journals/2026-09-06.md"),
    )
    .unwrap();
    drop(copy.store);

    let store = pkm_block::BlockStore::open(&copy.db_path).unwrap();
    let first = ensure_today_journal_core(&store, &copy.vault_path, "2026-09-06").unwrap();
    let second = ensure_today_journal_core(&store, &copy.vault_path, "2026-09-06").unwrap();

    assert_eq!(first.block_count, 1);
    assert_eq!(second.block_count, 1);
    let pages = store.list_pages().unwrap();
    assert_eq!(
        pages
            .iter()
            .filter(|p| p.as_str() == "journals/2026-09-06.md")
            .count(),
        1,
        "repeated ensure must not duplicate the journal page"
    );
    let blocks = store.get_blocks_by_page("journals/2026-09-06.md").unwrap();
    assert_eq!(blocks.len(), 1, "repeated ensure must not duplicate blocks");
}

// ---------------------------------------------------------------------------
// SYNC regression — `sync_filesystem_to_db` heals missing/zero-block pages
// ---------------------------------------------------------------------------

/// Regression: `sync_filesystem_to_db` must heal a page registered in SQLite
/// with ZERO block rows when the `.md` on disk has real content (the exact
/// decayed shape found in the live vault — page present with a block_count but
/// zero `blocks` rows). After the call the on-disk blocks are queryable.
#[test]
fn sync_filesystem_to_db_heals_zero_block_page_from_disk() {
    use app_lib::commands::page::sync_filesystem_to_db;

    let tv = create_test_vault();
    // Decayed shape: page registered in SQLite with zero block rows, but the
    // on-disk file has real content.
    tv.create_md_file(
        "pages/Borrowing.md",
        "- on-disk content one\n- on-disk content two\n",
    );
    tv.add_page("pages/Borrowing.md");

    // Precondition: the drift actually exists.
    let before = tv.store.get_blocks_by_page("pages/Borrowing.md").unwrap();
    assert!(
        before.is_empty(),
        "precondition: stale DB has zero block rows"
    );

    let healed = sync_filesystem_to_db(&tv.vault_path, &tv.db_path).unwrap();
    assert!(healed >= 1, "sync must heal the zero-block page: {healed}");

    // The on-disk blocks are now queryable.
    let blocks = tv.store.get_blocks_by_page("pages/Borrowing.md").unwrap();
    assert_eq!(
        blocks.len(),
        2,
        "healed page must expose its on-disk blocks"
    );
    assert!(
        blocks.iter().any(|b| b.content == "on-disk content one"),
        "healed block content must match the on-disk file"
    );
    assert!(
        blocks.iter().any(|b| b.content == "on-disk content two"),
        "healed block content must match the on-disk file"
    );
}

/// Regression: `sync_filesystem_to_db` must register a page missing entirely
/// from SQLite when the `.md` file exists on disk (the stale-DB import shape).
/// After the call the page is queryable with its on-disk blocks.
#[test]
fn sync_filesystem_to_db_heals_missing_page_from_disk() {
    use app_lib::commands::page::sync_filesystem_to_db;

    let tv = create_test_vault();
    // File on disk, page NOT in SQLite at all.
    tv.create_md_file("pages/new-note.md", "- fresh on-disk block\n");

    let healed = sync_filesystem_to_db(&tv.vault_path, &tv.db_path).unwrap();
    assert!(healed >= 1, "sync must import the missing page: {healed}");

    let pages = tv.store.list_pages().unwrap();
    assert!(
        pages.contains(&"pages/new-note.md".to_string()),
        "missing page must be registered in SQLite"
    );
    let blocks = tv.store.get_blocks_by_page("pages/new-note.md").unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].content, "fresh on-disk block");
}

/// Regression: `sync_filesystem_to_db` must be idempotent — running it twice must
/// not duplicate block rows for a healed page.
#[test]
fn sync_filesystem_to_db_heals_idempotently() {
    use app_lib::commands::page::sync_filesystem_to_db;

    let tv = create_test_vault();
    tv.create_md_file("pages/stable.md", "- stable block\n");
    tv.add_page("pages/stable.md");

    let first = sync_filesystem_to_db(&tv.vault_path, &tv.db_path).unwrap();
    assert!(first >= 1, "first sync must heal: {first}");

    // Idempotent: a page whose rows are already present is not re-synced (the
    // return value is the count of pages touched, so 0 means "nothing to heal").
    let second = sync_filesystem_to_db(&tv.vault_path, &tv.db_path).unwrap();
    assert_eq!(second, 0, "already-healed page must not be re-synced");

    let blocks = tv.store.get_blocks_by_page("pages/stable.md").unwrap();
    assert_eq!(blocks.len(), 1, "healed blocks must not be duplicated");
}
