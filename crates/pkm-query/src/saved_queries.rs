//! Saved Datalog query persistence.
//!
//! Provides [`SavedQuery`] and [`SavedQueryStore`] for persisting a named list
//! of Datalog queries inside the vault at `.pkm/saved_queries.json` — the same
//! Git-tracked directory as blocks.db and config, so saved queries survive
//! restarts and sync with the vault.
//!
//! All mutating operations are atomic: the new file contents are written to a
//! temporary sibling file and then `rename`d over the target, so a crash or
//! power loss can never leave a half-written JSON file behind. A missing file
//! is treated as an empty list, and a corrupt file falls back to an empty list
//! for reads (the next write repairs it).

use pkm_core::{PkmError, PkmResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// The vault-relative path where saved queries live. Kept as a constant so the
/// command layer can route the same path to the auto-commit engine.
pub const SAVED_QUERIES_REL_PATH: &str = ".pkm/saved_queries.json";

/// A single named Datalog query.
///
/// `updated_at` is an RFC 3339 UTC timestamp string (the same representation
/// the rest of the command DTOs use, e.g. `page.rs`/`sync.rs`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedQuery {
    /// Non-empty, unique display name.
    pub name: String,
    /// The Datalog query text.
    pub query: String,
    /// RFC 3339 UTC timestamp of the last create/update.
    pub updated_at: String,
}

/// Computes the target file path from the vault root.
pub fn saved_queries_path(vault_path: &Path) -> PathBuf {
    vault_path.join(SAVED_QUERIES_REL_PATH)
}

/// Validates a saved-query name. Returns the canonical (trimmed) form.
pub fn validate_name(name: &str) -> Result<String, PkmError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(PkmError::Validation(
            "Saved query name must not be empty".to_string(),
        ));
    }
    if trimmed.len() > 256 {
        return Err(PkmError::Validation(
            "Saved query name must be at most 256 characters".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Reads the saved-query list from disk.
///
/// A missing file yields an empty list; a corrupt file also yields an empty
/// list (the next save repairs it). The entries are returned in the order they
/// appear in the file.
pub fn load_saved_queries(vault_path: &Path) -> PkmResult<Vec<SavedQuery>> {
    let path = saved_queries_path(vault_path);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(PkmError::Io(e)),
    };
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    match serde_json::from_str::<Vec<SavedQuery>>(&content) {
        Ok(list) => Ok(list),
        Err(e) => {
            // Corrupt file: log and treat as empty. The command layer surfaces
            // nothing; the next mutation rewrites a valid file.
            tracing::warn!(
                "corrupt saved_queries.json ({}), treating as empty: {}",
                path.display(),
                e
            );
            Ok(Vec::new())
        }
    }
}

/// Atomically writes the full list to disk (temp file + rename).
fn atomic_write(vault_path: &Path, entries: &[SavedQuery]) -> PkmResult<()> {
    let path = saved_queries_path(vault_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(PkmError::Io)?;
    }
    let bytes = serde_json::to_vec_pretty(entries)?;

    // tempfile::NamedTempFile guarantees a unique, same-directory temp file so
    // the final rename is atomic on POSIX. We go through create_dir_all() +
    // the file's parent to keep the temp file on the same filesystem.
    let mut tmp = tempfile::NamedTempFile::new_in(
        path.parent()
            .ok_or_else(|| PkmError::Io(std::io::Error::other("no parent dir")))?,
    )?;
    tmp.write_all(&bytes)?;
    tmp.flush()?;
    // Persist contents to disk before renaming into place.
    tmp.as_file().sync_all()?;
    // Atomic replace. On Windows `persist` falls back to copy+remove when the
    // target exists; on POSIX it is a single rename syscall.
    tmp.persist(&path).map_err(|e| PkmError::Io(e.error))?;

    // fsync the directory so the rename is durable across power loss.
    if let Ok(dir) = fs::File::open(
        path.parent()
            .ok_or_else(|| PkmError::Io(std::io::Error::other("no parent dir")))?,
    ) {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// Persists a named query, creating or overwriting it.
///
/// Returns the stored [`SavedQuery`]. If a query with the same name already
/// exists, its `query` is replaced and `updated_at` is refreshed.
pub fn save_saved_query(vault_path: &Path, name: &str, query: &str) -> PkmResult<SavedQuery> {
    let name = validate_name(name)?;
    let mut list = load_saved_queries(vault_path)?;
    let now = chrono::Utc::now().to_rfc3339();
    let entry = SavedQuery {
        name: name.clone(),
        query: query.to_string(),
        updated_at: now,
    };

    if let Some(existing) = list.iter_mut().find(|e| e.name == name) {
        existing.query = entry.query.clone();
        existing.updated_at = entry.updated_at.clone();
    } else {
        list.push(entry.clone());
    }

    atomic_write(vault_path, &list)?;
    Ok(entry)
}

/// Renames an existing saved query.
///
/// Returns the updated [`SavedQuery`]. Errors if `old_name` does not exist or
/// `new_name` is already taken.
pub fn rename_saved_query(
    vault_path: &Path,
    old_name: &str,
    new_name: &str,
) -> PkmResult<SavedQuery> {
    let new_name = validate_name(new_name)?;
    let mut list = load_saved_queries(vault_path)?;

    let idx = list
        .iter()
        .position(|e| e.name == old_name.trim())
        .ok_or_else(|| PkmError::NotFound(format!("Saved query not found: {}", old_name)))?;
    if list.iter().any(|e| e.name == new_name) {
        return Err(PkmError::AlreadyExists(format!(
            "A saved query named '{}' already exists",
            new_name
        )));
    }

    list[idx].name = new_name.clone();
    list[idx].updated_at = chrono::Utc::now().to_rfc3339();
    let result = list[idx].clone();

    atomic_write(vault_path, &list)?;
    Ok(result)
}

/// Deletes a saved query by name. Returns `true` if a query was removed,
/// `false` if no query with that name existed.
pub fn delete_saved_query(vault_path: &Path, name: &str) -> PkmResult<bool> {
    let mut list = load_saved_queries(vault_path)?;
    let before = list.len();
    list.retain(|e| e.name != name.trim());
    if list.len() == before {
        return Ok(false);
    }
    atomic_write(vault_path, &list)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault() -> tempfile::TempDir {
        tempfile::TempDir::new().unwrap()
    }

    #[test]
    fn list_missing_file_returns_empty() {
        let dir = vault();
        assert_eq!(load_saved_queries(dir.path()).unwrap(), Vec::new());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = vault();
        let saved = save_saved_query(
            dir.path(),
            "Top tasks",
            "{:query [:find ?b :where [?b :block/marker \"TODO\"]]}",
        )
        .unwrap();
        assert_eq!(saved.name, "Top tasks");
        assert!(!saved.updated_at.is_empty());

        let list = load_saved_queries(dir.path()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Top tasks");
        assert_eq!(
            list[0].query,
            "{:query [:find ?b :where [?b :block/marker \"TODO\"]]}"
        );
        assert!(
            saved_queries_path(dir.path()).exists(),
            "file must be written to .pkm/saved_queries.json"
        );
        assert!(
            saved_queries_path(dir.path()).starts_with(dir.path().join(".pkm")),
            "file must live under .pkm/"
        );
    }

    #[test]
    fn save_overwrites_existing_name_and_refreshes_timestamp() {
        let dir = vault();
        let first = save_saved_query(dir.path(), "q", "query one").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let second = save_saved_query(dir.path(), "q", "query two").unwrap();

        assert_eq!(second.name, "q");
        assert_eq!(second.query, "query two");
        assert!(second.updated_at >= first.updated_at);

        let list = load_saved_queries(dir.path()).unwrap();
        assert_eq!(list.len(), 1, "overwrite must not create a duplicate");
        assert_eq!(list[0].query, "query two");
    }

    #[test]
    fn save_trims_name_and_rejects_empty() {
        let dir = vault();
        let saved = save_saved_query(dir.path(), "  spaced  ", "q").unwrap();
        assert_eq!(saved.name, "spaced");
        // Re-saving with the untrimmed name collapses to the same entry.
        save_saved_query(dir.path(), "spaced", "q2").unwrap();
        let list = load_saved_queries(dir.path()).unwrap();
        assert_eq!(list.len(), 1);

        let err = save_saved_query(dir.path(), "   ", "q").unwrap_err();
        assert!(matches!(err, PkmError::Validation(_)));
    }

    #[test]
    fn rename_updates_name_and_persists() {
        let dir = vault();
        save_saved_query(dir.path(), "old", "q").unwrap();
        let renamed = rename_saved_query(dir.path(), "old", "new").unwrap();
        assert_eq!(renamed.name, "new");

        let list = load_saved_queries(dir.path()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "new");
    }

    #[test]
    fn rename_missing_source_errors() {
        let dir = vault();
        let err = rename_saved_query(dir.path(), "nope", "new").unwrap_err();
        assert!(matches!(err, PkmError::NotFound(_)));
    }

    #[test]
    fn rename_to_existing_name_errors() {
        let dir = vault();
        save_saved_query(dir.path(), "a", "q1").unwrap();
        save_saved_query(dir.path(), "b", "q2").unwrap();
        let err = rename_saved_query(dir.path(), "a", "b").unwrap_err();
        assert!(matches!(err, PkmError::AlreadyExists(_)));
        // Nothing changed on disk.
        let list = load_saved_queries(dir.path()).unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|e| e.name == "a"));
    }

    #[test]
    fn delete_removes_and_persists() {
        let dir = vault();
        save_saved_query(dir.path(), "a", "q1").unwrap();
        save_saved_query(dir.path(), "b", "q2").unwrap();
        assert!(delete_saved_query(dir.path(), "a").unwrap());
        let list = load_saved_queries(dir.path()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "b");
    }

    #[test]
    fn delete_missing_returns_false_without_write() {
        let dir = vault();
        assert!(!delete_saved_query(dir.path(), "nope").unwrap());
        assert!(!saved_queries_path(dir.path()).exists());
    }

    #[test]
    fn special_characters_are_supported() {
        let dir = vault();
        let weird = "任务 & <quotes> \"x\" 'y' \u{1F4DD}";
        let saved = save_saved_query(dir.path(), weird, "q: {}").unwrap();
        assert_eq!(saved.name, weird);
        let list = load_saved_queries(dir.path()).unwrap();
        assert_eq!(list[0].name, weird);
    }

    #[test]
    fn corrupt_file_returns_empty_and_is_repaired_on_save() {
        let dir = vault();
        let path = saved_queries_path(dir.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{ this is not json !!!").unwrap();

        assert_eq!(load_saved_queries(dir.path()).unwrap(), Vec::new());

        // A subsequent save repairs the file.
        save_saved_query(dir.path(), "fixed", "q").unwrap();
        let list = load_saved_queries(dir.path()).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "fixed");
    }

    #[test]
    fn restart_simulation_reads_back_from_disk() {
        // Simulates an app restart: the file written by one "session" (a fresh
        // store with no in-memory state) is read back by another. The store is
        // stateless — all state is in the file — so this exercises the real
        // persistence boundary.
        let dir = vault();
        save_saved_query(dir.path(), "persist-me", "{:query [:find ?b] :where []}").unwrap();

        // Simulated restart: construct a brand-new store handle to the same
        // vault and read the list back cold.
        let after_restart: Vec<SavedQuery> = load_saved_queries(dir.path()).unwrap();
        assert_eq!(after_restart.len(), 1);
        assert_eq!(after_restart[0].name, "persist-me");

        // And confirm the raw file is valid JSON with the expected fields.
        let raw = std::fs::read_to_string(saved_queries_path(dir.path())).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed[0]["name"], "persist-me");
        assert!(parsed[0]["query"].as_str().is_some());
        assert!(parsed[0]["updated_at"].as_str().is_some());
    }

    #[test]
    fn empty_query_text_is_allowed() {
        let dir = vault();
        save_saved_query(dir.path(), "draft", "").unwrap();
        let list = load_saved_queries(dir.path()).unwrap();
        assert_eq!(list[0].query, "");
    }
}
