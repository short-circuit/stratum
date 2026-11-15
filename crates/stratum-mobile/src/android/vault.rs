//! Vault location and first-run layout for the Android host.
//!
//! The shipped mobile app stores the vault in the app's documents directory so
//! the plain `.md` files stay user-visible and survive reinstalls that preserve
//! documents. [`StratumVault::location`] resolves
//! `gpui_mobile::packages::path_provider::documents_directory()` and falls back
//! to the temporary directory when documents is unavailable, while
//! [`StratumVault::open`] creates the first-run layout:
//!
//! - the vault root (`…/StratumVault`),
//! - the `.pkm` state directory (SQLite block store, search index, config),
//! - a `.pkm/mobile.json` marker recording that the host created the vault.
//!
//! No pages are seeded: an empty vault is valid, the shell renders its empty
//! states, and the user creates pages from the UI. The marker only makes the
//! first run observable from `adb shell` without adding user-visible content.

use std::fs;
use std::path::{Path, PathBuf};

/// The vault directory name created under the app documents directory.
pub const VAULT_DIR: &str = crate::VAULT_DIR;

/// The state directory name the service layer uses.
const STATE_DIR: &str = ".pkm";

/// The first-run marker file name inside the state directory.
const MARKER_FILE: &str = "mobile.json";

/// An opened on-device vault.
///
/// Constructing the vault creates the layout but never seeds pages. The
/// returned path is handed to `stratum_ui::provider::provider_from_env`, which
/// owns every later read/write through the headless service layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StratumVault {
    root: PathBuf,
}

impl StratumVault {
    /// Resolve the vault location on the device.
    ///
    /// Prefers the app documents directory; when that is unavailable (for
    /// example before the platform finished booting) it falls back to the
    /// temporary directory. Both come from the vendored gpui-mobile packages
    /// and return `Err` rather than panicking when no platform exists.
    pub fn location() -> Result<PathBuf, String> {
        match gpui_mobile::packages::path_provider::documents_directory() {
            Ok(documents) => Ok(documents.join(VAULT_DIR)),
            Err(error) => {
                log::warn!(
                    "vault: documents directory unavailable ({error}); trying the temporary directory"
                );
                gpui_mobile::packages::path_provider::temporary_directory()
                    .map(|temporary| temporary.join(VAULT_DIR))
                    .map_err(|fallback| {
                        format!(
                            "no writable vault location (documents: {error}; temporary: {fallback})"
                        )
                    })
            }
        }
    }

    /// Create the first-run layout for `root` and return the opened vault.
    ///
    /// Creates the vault root, the `.pkm` state directory and the first-run
    /// marker. Idempotent: an existing vault is opened without touching its
    /// pages or configuration.
    pub fn open(root: PathBuf) -> Result<Self, String> {
        let state = root.join(STATE_DIR);
        fs::create_dir_all(&state).map_err(|error| {
            format!(
                "failed to create the vault state directory {}: {error}",
                state.display()
            )
        })?;

        let marker_path = state.join(MARKER_FILE);
        if !marker_path.exists() {
            let marker = serde_json::json!({
                "version": 1,
                "host": crate::APPLICATION_ID,
                "seeded_pages": 0,
            });
            let bytes = serde_json::to_vec_pretty(&marker)
                .map_err(|error| format!("failed to encode the first-run marker: {error}"))?;
            fs::write(&marker_path, bytes).map_err(|error| {
                format!(
                    "failed to write the first-run marker {}: {error}",
                    marker_path.display()
                )
            })?;
            log::info!(
                "vault: created first-run layout at {} (no pages seeded)",
                root.display()
            );
        } else {
            log::info!("vault: reusing the existing vault at {}", root.display());
        }

        Ok(Self { root })
    }

    /// The vault root path handed to the service provider.
    pub fn path(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "stratum-mobile-vault-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn open_creates_the_state_layout_without_seeding_pages() {
        let root = scratch_root("layout");
        let vault = StratumVault::open(root.clone()).expect("vault opens");
        assert_eq!(vault.path(), root.as_path());
        assert!(root.join(STATE_DIR).is_dir());
        assert!(root.join(STATE_DIR).join(MARKER_FILE).is_file());

        // Nothing user-visible is seeded: the vault has no Markdown pages.
        let pages = fs::read_dir(&root)
            .expect("vault root is readable")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"))
            .count();
        assert_eq!(pages, 0);

        let marker =
            fs::read_to_string(root.join(STATE_DIR).join(MARKER_FILE)).expect("marker is readable");
        let marker: serde_json::Value = serde_json::from_str(&marker).expect("marker is JSON");
        assert_eq!(marker["seeded_pages"], 0);
        assert_eq!(marker["host"], crate::APPLICATION_ID);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn open_is_idempotent() {
        let root = scratch_root("idempotent");
        let first = StratumVault::open(root.clone()).expect("first open");
        let marker_path = root.join(STATE_DIR).join(MARKER_FILE);
        let marker = fs::read(&marker_path).expect("marker written");
        let second = StratumVault::open(root.clone()).expect("second open");
        assert_eq!(first, second);
        assert_eq!(
            fs::read(&marker_path).expect("marker still readable"),
            marker
        );
        let _ = fs::remove_dir_all(&root);
    }
}
