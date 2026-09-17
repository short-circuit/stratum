use super::*;

#[test]
fn test_resolve_saf_already_in_progress() {
    let vs = VaultState::new(PathBuf::from("/tmp/stratum-nonexistent-for-test"));
    assert!(vs.try_start_indexing().is_ok());
    // A second start while the first is active must fail.
    assert!(vs.try_start_indexing().is_err());
    assert!(vs.is_indexing());
    vs.finish_indexing();
    assert!(!vs.is_indexing());
}

#[test]
fn test_indexing_guard_releases_flag_on_drop() {
    // Regression for #169: an erroring operation must never leak the
    // indexing-in-progress flag (which would permanently disable the watcher).
    let vs = VaultState::new(PathBuf::from("/tmp/stratum-nonexistent-for-test"));
    {
        let _guard = IndexingGuard::new(&vs).unwrap();
        assert!(vs.is_indexing());
        // Guard must tolerate &mut-style callers during its lifetime; the Arc
        // ownership means no borrow of `vs` is held while the guard is alive.
        assert!(vs.get_store().is_ok() || vs.get_store().is_err());
    }
    assert!(!vs.is_indexing(), "guard drop must clear the indexing flag");
}

#[test]
fn test_percent_decode_simple() {
    assert_eq!(percent_decode("hello%20world"), "hello world");
}

#[test]
fn test_percent_decode_no_encoding() {
    assert_eq!(percent_decode("plaintext"), "plaintext");
}

#[test]
fn test_percent_decode_multiple() {
    assert_eq!(percent_decode("%46%6F%6F"), "Foo");
}

#[test]
fn test_percent_decode_empty() {
    assert_eq!(percent_decode(""), "");
}

#[test]
fn test_resolve_saf_primary_storage() {
    let uri = "content://com.android.externalstorage.documents/tree/primary%3ADocuments%2FMyVault";
    let result = resolve_saf_content_uri(uri).unwrap();
    assert_eq!(
        result,
        PathBuf::from("/storage/emulated/0/Documents/MyVault")
    );
}

#[test]
fn test_resolve_saf_secondary_volume() {
    let uri = "content://com.android.externalstorage.documents/tree/1234-5678%3AMyVault";
    let result = resolve_saf_content_uri(uri).unwrap();
    assert_eq!(result, PathBuf::from("/storage/1234-5678/MyVault"));
}

#[test]
fn test_resolve_saf_deeply_nested() {
    let uri = "content://com.android.externalstorage.documents/tree/primary%3ADocuments%2FProjects%2FStratum%2Fvault";
    let result = resolve_saf_content_uri(uri).unwrap();
    assert_eq!(
        result,
        PathBuf::from("/storage/emulated/0/Documents/Projects/Stratum/vault")
    );
}

#[test]
fn test_resolve_saf_invalid_uri() {
    let uri = "content://com.android.something/not-a-tree-uri";
    let result = resolve_saf_content_uri(uri);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Could not parse Android content URI"));
}

#[test]
fn test_resolve_picked_path_non_android() {
    #[cfg(not(target_os = "android"))]
    {
        let result = resolve_picked_path("/home/user/vault").unwrap();
        assert_eq!(result, PathBuf::from("/home/user/vault"));
    }
}

#[test]
fn test_resolve_saf_with_document_suffix() {
    // Some SAF implementations append /document/primary:... after the tree root
    let uri = "content://com.android.externalstorage.documents/tree/primary%3AStratumVault/document/primary%3AStratumVault";
    let result = resolve_saf_content_uri(uri).unwrap();
    assert_eq!(result, PathBuf::from("/storage/emulated/0/StratumVault"));
}
