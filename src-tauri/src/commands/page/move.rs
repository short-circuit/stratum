//! Page move/rename command with inbound wiki-link rewrite.
//!
//! `move_page` atomically renames a page file, updates SQLite to the new path,
//! and rewrites every inbound wiki-link that referenced the old identity (path,
//! slug, title, display forms) so the graph and backlinks stay convergent.
//! Folder-only moves rewrite path-qualified links only; basename renames also
//! rewrite name links. Block references `((uuid))` are untouched.

use super::{get_file_mtime, resolve_safe_write_path, PageDto};
use crate::commands::vault::AppState;
use tauri::Emitter;

/// Normalize a page path, auto-appending a `.md` extension when missing
/// (mirrors `create_page`).
fn normalize_path(path: &str) -> String {
    if path.ends_with(".md") {
        path.to_string()
    } else {
        format!("{}.md", path)
    }
}

/// Compute the file stem (slug) of a vault-relative path.
fn rel_slug(rel: &str) -> String {
    std::path::Path::new(rel)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string()
}

/// Strip a trailing `.md` extension if present.
fn strip_md(rel: &str) -> String {
    rel.strip_suffix(".md").unwrap_or(rel).to_string()
}

/// The single supported rename/move operation for pages.
///
/// Atomic filesystem rename + inbound link rewrite + DB/index/graph update.
/// Page identity is NAME-based (frontmatter `title` if set, else slug). A folder
/// move (unchanged basename) never rewrites name links; a basename rename does.
#[tauri::command]
pub async fn move_page(
    old_path: String,
    new_path: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<PageDto, String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    let vault_path = state.vault_path.clone();
    let old_rel = normalize_path(&old_path);
    let new_rel = normalize_path(&new_path);

    // Validate/canonicalize both paths so they stay within the vault.
    resolve_safe_write_path(&vault_path, &old_rel)?;
    resolve_safe_write_path(&vault_path, &new_rel)?;

    let store = state.get_store().map_err(|e| e.to_string())?;
    let (dto, changed_sources) =
        move_page_core_with_changed(&store, &vault_path, &old_rel, &new_rel)?;

    // Auto-commit staging for the moved page and every rewritten source.
    state.record_change(&old_rel);
    state.record_change(&new_rel);
    for rel in &changed_sources {
        state.record_change(rel);
    }

    // Drop BlockIndex writer before IndexEngine acquires its own (same Tantivy dir).
    drop(state.block_index.take());
    let ie = state.ensure_index()?;
    let _ = ie.remove_note(&old_rel);
    ie.refresh_page(&new_rel, &vault_path)
        .map_err(|e| format!("Index refresh failed: {}", e))?;
    for rel in &changed_sources {
        let _ = ie.refresh_page(rel, &vault_path);
    }

    crate::commands::graph::invalidate_graph_cache();
    let _ = app.emit("pages-changed", ());
    // Suppress the file watcher echo of our own writes.
    state.watcher_last_save = std::time::SystemTime::now();

    Ok(dto)
}

/// Testable core: dispatch to the full move and discard the set of rewritten
/// sources. See [`move_page_core_with_changed`] for the real logic.
pub fn move_page_core(
    store: &pkm_block::BlockStore,
    vault_path: &std::path::Path,
    old_rel: &str,
    new_rel: &str,
) -> Result<PageDto, String> {
    move_page_core_with_changed(store, vault_path, old_rel, new_rel).map(|(dto, _changed)| dto)
}

fn move_page_core_with_changed(
    store: &pkm_block::BlockStore,
    vault_path: &std::path::Path,
    old_rel: &str,
    new_rel: &str,
) -> Result<(PageDto, Vec<String>), String> {
    let old_rel = normalize_path(old_rel);
    let new_rel = normalize_path(new_rel);
    let old_full = vault_path.join(&old_rel);
    let new_full = vault_path.join(&new_rel);

    // No-op: same path — nothing to do, succeed with the current page.
    if old_rel == new_rel {
        let dto = build_dto(store, &new_rel, &new_full)?;
        return Ok((dto, Vec::new()));
    }

    if new_full.exists() {
        return Err(format!("Page already exists: {}", new_rel));
    }
    if !old_full.exists() {
        return Err(format!("Page does not exist: {}", old_rel));
    }

    // Capture the OLD title before deleting the old row (title is preserved verbatim
    // across a move, so it also serves as the "new" title for rewrite purposes).
    let old_title = store
        .get_page(&old_rel)
        .map_err(|e| e.to_string())?
        .and_then(|fm| fm.title);
    let old_slug = rel_slug(&old_rel);
    let new_slug = rel_slug(&new_rel);
    let old_display = old_slug.replace('-', " ");
    let new_display = new_slug.replace('-', " ");

    // Atomic filesystem rename (create parent dirs first).
    if let Some(parent) = new_full.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if let Err(e) = std::fs::rename(&old_full, &new_full) {
        if e.kind() == std::io::ErrorKind::CrossesDevices {
            return Err(
                "move_page: source and destination are on different filesystems (EXDEV); \
                 copy+delete fallback not implemented yet"
                    .to_string(),
            );
        }
        return Err(format!("Failed to move {} -> {}: {}", old_rel, new_rel, e));
    }

    // Read the moved file and parse it (this is the source of truth for the new page).
    let new_content = std::fs::read_to_string(&new_full).map_err(|e| e.to_string())?;
    let (fm, _body, blocks) = pkm_markdown::block_parser::parse_document(&new_content);

    // DB update inside a single explicit transaction.
    store.execute_batch("BEGIN").map_err(|e| e.to_string())?;
    let db_result = (|| -> Result<(), String> {
        store
            .delete_blocks_by_page(&old_rel)
            .map_err(|e| e.to_string())?;
        store.delete_page(&old_rel).map_err(|e| e.to_string())?;
        let mut page = pkm_block::Page::new(new_full.clone(), vault_path);
        page.frontmatter = pkm_block::PageFrontmatter {
            title: fm.title.clone(),
            created: fm.created.clone(),
            modified: fm.modified.clone(),
            tags: fm.tags.clone(),
            aliases: fm.aliases.clone(),
            ..Default::default()
        };
        page.set_blocks(&blocks);
        store.upsert_page(&page).map_err(|e| e.to_string())?;
        for block in &blocks {
            store
                .insert_block(block, &new_rel)
                .map_err(|e| e.to_string())?;
        }
        crate::commands::page::reconcile_page_links(store, &new_rel, &blocks)?;
        Ok(())
    })();
    match db_result {
        Ok(()) => store.execute_batch("COMMIT").map_err(|e| e.to_string())?,
        Err(e) => {
            store.execute_batch("ROLLBACK").ok();
            return Err(e);
        }
    }

    // Old identities used both to (a) find inbound source pages and (b) build the
    // rewrite map. The map keys are one representative per identity; matching is
    // case-insensitive so no lower-cased duplicate keys are needed.
    let mut old_ids: Vec<String> = vec![old_rel.clone(), strip_md(&old_rel)];
    old_ids.push(old_slug.clone());
    old_ids.push(old_slug.to_lowercase());
    old_ids.push(old_display.clone());
    old_ids.push(old_display.to_lowercase());
    let mut pairs: Vec<(String, String)> = vec![
        (old_rel.clone(), new_rel.clone()),
        (strip_md(&old_rel), strip_md(&new_rel)),
        (old_slug.clone(), new_slug.clone()),
        (old_display, new_display),
    ];
    if let Some(ref ot) = old_title {
        old_ids.push(ot.clone());
        old_ids.push(ot.to_lowercase());
        // New title is unchanged from the old title, so this is an identity map.
        pairs.push((ot.clone(), ot.clone()));
    }
    let map_refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();

    // Inbound rewrite: every page whose blocks link to any old identity (this set
    // includes the moved page itself under new_rel). Apply the map ONCE per link
    // using only the old identities — no cascading second pass.
    let sources = store
        .get_source_pages_for_targets(&old_ids)
        .map_err(|e| e.to_string())?;
    let mut changed_sources = Vec::new();
    for rel in &sources {
        let full = vault_path.join(rel);
        let content = match std::fs::read_to_string(&full) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let (_sfm, _sbody, mut sblocks) = pkm_markdown::block_parser::parse_document(&content);
        let mut changed_any = false;
        for block in sblocks.iter_mut() {
            let (new_text, ch) =
                pkm_markdown::linker::rewrite_link_targets(&block.content, &map_refs);
            if ch {
                block.content = new_text;
                changed_any = true;
            }
        }
        if changed_any {
            let body = pkm_markdown::block_parser::serialize_blocks(&sblocks);
            let final_md =
                pkm_markdown::block_parser::assemble_blocks_markdown(&content, &body, None);
            // Atomic write via temp + rename, then re-sync the page's DB rows.
            let temp = full.with_extension("md.tmp");
            std::fs::write(&temp, &final_md).map_err(|e| e.to_string())?;
            std::fs::rename(&temp, &full).map_err(|e| e.to_string())?;
            crate::commands::page::sync_page_from_disk(store, rel, vault_path, None)?;
            changed_sources.push(rel.clone());
        }
    }

    let dto = build_dto(store, &new_rel, &new_full)?;
    Ok((dto, changed_sources))
}

/// Build the moved page's `PageDto` from its current DB state.
fn build_dto(
    store: &pkm_block::BlockStore,
    new_rel: &str,
    new_full: &std::path::Path,
) -> Result<PageDto, String> {
    let slug = rel_slug(new_rel);
    let blocks = store
        .get_blocks_by_page(new_rel)
        .map_err(|e| e.to_string())?;
    let title = store
        .get_page(new_rel)
        .map_err(|e| e.to_string())?
        .and_then(|fm| fm.title);
    Ok(PageDto {
        path: new_rel.to_string(),
        slug,
        title,
        block_count: blocks.len(),
        modified_at: get_file_mtime(new_full),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkm_block::BlockStore;
    use tempfile::TempDir;

    /// Create a temp vault directory and an opened BlockStore pointing at it.
    fn fixture() -> (TempDir, BlockStore) {
        let dir = TempDir::new().expect("tempdir");
        let vault = dir.path();
        std::fs::create_dir_all(vault.join(".pkm")).expect("create .pkm");
        let db_path = vault.join(".pkm").join("blocks.db");
        let store = BlockStore::open(&db_path).expect("open store");
        (dir, store)
    }

    /// Write a .md file into the vault and sync it into the store.
    fn write_page(store: &BlockStore, vault: &std::path::Path, rel: &str, content: &str) {
        let full = vault.join(rel);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(&full, content).expect("write file");
        crate::commands::page::sync_page_from_disk(store, rel, vault, None).expect("sync page");
    }

    #[test]
    fn folder_only_move_preserves_name_links() {
        let (dir, store) = fixture();
        let vault = dir.path();
        write_page(&store, vault, "pages/a.md", "[[pages/b|B]]\n[[B]]\n");
        write_page(&store, vault, "pages/b.md", "body of b\n");

        move_page_core(&store, vault, "pages/b.md", "pages/archive/b.md").expect("move ok");

        let a_disk = std::fs::read_to_string(vault.join("pages/a.md")).expect("read a");
        assert!(
            a_disk.contains("[[pages/archive/b|B]]"),
            "path link rewritten"
        );
        assert!(a_disk.contains("[[B]]"), "name link preserved");

        let pages = store.list_pages().expect("list");
        assert!(!pages.contains(&"pages/b.md".to_string()));
        assert!(pages.contains(&"pages/archive/b.md".to_string()));

        let a_blocks = store.get_blocks_by_page("pages/a.md").expect("a blocks");
        let a_id = a_blocks[0].id.to_string();
        let backlinks = store
            .get_backlinks_for_page("pages/archive/b")
            .expect("backlinks");
        assert!(backlinks.contains(&a_id));
        let old_backlinks = store.get_backlinks_for_page("pages/b").expect("old");
        assert!(old_backlinks.is_empty());
    }

    #[test]
    fn rename_rewrites_name_and_path_links() {
        let (dir, store) = fixture();
        let vault = dir.path();
        write_page(
            &store,
            vault,
            "pages/a.md",
            "[[b]]\n[[pages/b]]\n[[b|see B]]\n[[B]]\n",
        );
        write_page(&store, vault, "pages/b.md", "body of b\n");

        move_page_core(&store, vault, "pages/b.md", "pages/c.md").expect("move ok");

        let a_disk = std::fs::read_to_string(vault.join("pages/a.md")).expect("read a");
        assert!(a_disk.contains("[[c]]"));
        assert!(a_disk.contains("[[pages/c]]"));
        assert!(a_disk.contains("[[c|see B]]"));
        assert!(a_disk.contains("[[C]]"));
    }

    #[test]
    fn title_form_links_remain_intact() {
        let (dir, store) = fixture();
        let vault = dir.path();
        write_page(&store, vault, "pages/a.md", "[[B Title]]\n[[b]]\n");
        write_page(
            &store,
            vault,
            "pages/b.md",
            "---\ntitle: B Title\n---\nbody\n",
        );

        move_page_core(&store, vault, "pages/b.md", "pages/c.md").expect("move ok");

        let a_disk = std::fs::read_to_string(vault.join("pages/a.md")).expect("read a");
        assert!(a_disk.contains("[[B Title]]"));
        assert!(a_disk.contains("[[c]]"));

        let backlinks = store.get_backlinks_for_page("B Title").expect("backlinks");
        assert!(!backlinks.is_empty());
    }

    #[test]
    fn no_double_rewrite_of_existing_target() {
        let (dir, store) = fixture();
        let vault = dir.path();
        // a.md already contains a `[[c]]` link; renaming b->c must rewrite only
        // `[[b]]` and leave the pre-existing `[[c]]` untouched.
        write_page(&store, vault, "pages/a.md", "[[b]] [[c]]\n");
        write_page(&store, vault, "pages/b.md", "body of b\n");

        move_page_core(&store, vault, "pages/b.md", "pages/c.md").expect("move ok");

        let a_disk = std::fs::read_to_string(vault.join("pages/a.md")).expect("read a");
        // The source page is re-serialized in block-list format; assert that the
        // `[[b]]` was rewritten to `[[c]]` and the pre-existing `[[c]]` is intact
        // (exactly two `[[c]]`, no trailing `[[b]]`).
        assert!(a_disk.contains("[[c]] [[c]]"));
        assert!(!a_disk.contains("[[b]]"));
        assert_eq!(a_disk.matches("[[c]]").count(), 2);
    }

    #[test]
    fn missing_old_file_is_error() {
        let (dir, store) = fixture();
        let vault = dir.path();
        write_page(&store, vault, "pages/a.md", "hello\n");

        let err = move_page_core(&store, vault, "pages/missing.md", "pages/x.md")
            .expect_err("should error");
        assert!(err.contains("does not exist"), "err: {err}");
    }

    #[test]
    fn existing_destination_is_error() {
        let (dir, store) = fixture();
        let vault = dir.path();
        write_page(&store, vault, "pages/a.md", "hello a\n");
        write_page(&store, vault, "pages/b.md", "hello b\n");

        let err =
            move_page_core(&store, vault, "pages/b.md", "pages/a.md").expect_err("should error");
        assert!(err.contains("already exists"), "err: {err}");
    }

    #[test]
    fn noop_same_path_succeeds() {
        let (dir, store) = fixture();
        let vault = dir.path();
        write_page(&store, vault, "pages/a.md", "hello\n");

        let dto = move_page_core(&store, vault, "pages/a.md", "pages/a.md").expect("noop ok");
        assert_eq!(dto.path, "pages/a.md");
        let pages = store.list_pages().expect("list");
        assert!(pages.contains(&"pages/a.md".to_string()));
    }
}
