//! Installed theme storage and Theme Hub operations.
//!
//! Themes are small JSON documents downloaded from the community theme
//! catalog. The shipped React app keeps them in `localStorage`
//! (`src/components/PluginsPanel/useThemeHub.ts`); the headless service stores
//! them under `<vault>/.pkm/themes/<id>.json` and records the active theme in
//! `<vault>/.pkm/themes/active`. Downloading is behind
//! [`CatalogFetch`](crate::plugins::catalog::CatalogFetch) so tests stay
//! offline.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::plugins::catalog::{CatalogFetch, ReqwestFetch, ThemeCatalogEntry};
use crate::state::StratumService;

/// Built-in theme id used when no installed theme is active.
pub const BUILTIN_THEME_ID: &str = "__builtin__";
/// Stable error for a theme document outside the accepted shape.
pub const THEME_SCHEMA_INVALID: &str = "theme_schema_invalid";
/// Stable error for a download failure.
pub const THEME_DOWNLOAD_ERROR: &str = "theme_download_error";
/// Stable error for a checksum mismatch.
pub const THEME_CHECKSUM_MISMATCH: &str = "theme_checksum_mismatch";
/// Stable error for an id that cannot be used as a file name.
pub const THEME_INVALID_ID: &str = "theme_invalid_id";

/// Theme values applied to the shell and settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeValues {
    /// Whether the theme is dark.
    pub dark_mode: bool,
    /// Primary color.
    pub primary_color: String,
    /// Secondary color.
    pub secondary_color: String,
    /// Base font size.
    pub font_size: f64,
}

/// One installed theme.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct InstalledTheme {
    /// Stable theme id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Theme values.
    pub values: ThemeValues,
    /// Whether this theme is currently active.
    pub active: bool,
}

/// On-disk theme record.
#[derive(Debug, Serialize, Deserialize)]
struct StoredTheme {
    id: String,
    name: String,
    values: ThemeValues,
}

/// The built-in fallback theme values.
pub fn builtin_values() -> ThemeValues {
    ThemeValues {
        dark_mode: true,
        primary_color: "#f97316".to_string(),
        secondary_color: "#6b7280".to_string(),
        font_size: 16.0,
    }
}

/// Directory holding installed theme files.
fn theme_dir(vault: &Path) -> PathBuf {
    vault.join(".pkm").join("themes")
}

/// Validate `id` and return its stored file path.
fn theme_path(vault: &Path, id: &str) -> Result<PathBuf, String> {
    let valid = !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !valid {
        return Err(THEME_INVALID_ID.to_string());
    }
    Ok(theme_dir(vault).join(format!("{id}.json")))
}

/// Path of the active-theme marker.
fn active_path(vault: &Path) -> PathBuf {
    theme_dir(vault).join("active")
}

/// Read the active theme id, if any.
pub fn active_theme_id(vault: &Path) -> Option<String> {
    std::fs::read_to_string(active_path(vault))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Write the active theme id.
fn set_active(vault: &Path, id: &str) -> Result<(), String> {
    std::fs::create_dir_all(theme_dir(vault)).map_err(|e| format!("theme_write_error: {e}"))?;
    std::fs::write(active_path(vault), id).map_err(|e| format!("theme_write_error: {e}"))
}

/// Remove the active-theme marker (fall back to the built-in theme).
fn clear_active(vault: &Path) -> Result<(), String> {
    match std::fs::remove_file(active_path(vault)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("theme_write_error: {error}")),
    }
}

/// Read one installed theme from disk.
fn read_stored(vault: &Path, id: &str) -> Option<StoredTheme> {
    let path = theme_path(vault, id).ok()?;
    let raw = std::fs::read(&path).ok()?;
    serde_json::from_slice(&raw).ok()
}

/// Write one theme record to disk.
fn write_stored(vault: &Path, stored: &StoredTheme) -> Result<(), String> {
    let path = theme_path(vault, &stored.id)?;
    std::fs::create_dir_all(theme_dir(vault)).map_err(|e| format!("theme_write_error: {e}"))?;
    let encoded = serde_json::to_vec(stored).map_err(|e| e.to_string())?;
    std::fs::write(path, encoded).map_err(|e| format!("theme_write_error: {e}"))
}

/// List every installed theme, marking the active one.
///
/// Unreadable or malformed theme files are skipped. The list is sorted by id
/// so captures and tests stay deterministic.
pub fn list_installed_themes(vault: &Path) -> Vec<InstalledTheme> {
    let active = active_theme_id(vault);
    let Ok(entries) = std::fs::read_dir(theme_dir(vault)) else {
        return Vec::new();
    };
    let mut themes = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = std::fs::read(&path) else {
            continue;
        };
        let Ok(stored) = serde_json::from_slice::<StoredTheme>(&raw) else {
            continue;
        };
        themes.push(InstalledTheme {
            active: active.as_deref() == Some(stored.id.as_str()),
            id: stored.id,
            name: stored.name,
            values: stored.values,
        });
    }
    themes.sort_by(|a, b| a.id.cmp(&b.id));
    themes
}

/// Apply an installed theme by id and return its values.
///
/// [`BUILTIN_THEME_ID`] clears the active marker and returns the built-in
/// fallback. Any other id must exist on disk.
pub fn apply_theme(vault: &Path, id: &str) -> Result<ThemeValues, String> {
    if id == BUILTIN_THEME_ID {
        clear_active(vault)?;
        return Ok(builtin_values());
    }
    let stored = read_stored(vault, id).ok_or_else(|| format!("theme_not_found: {id}"))?;
    set_active(vault, &stored.id)?;
    Ok(stored.values)
}

/// Remove an installed theme.
///
/// When the removed theme was active the marker is cleared, which falls back
/// to the built-in theme. Removing a missing theme is not an error.
pub fn uninstall_theme(vault: &Path, id: &str) -> Result<(), String> {
    let path = theme_path(vault, id)?;
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("theme_write_error: {error}")),
    }
    if active_theme_id(vault).as_deref() == Some(id) {
        clear_active(vault)?;
    }
    Ok(())
}

/// Compute the lowercase hex sha256 digest of `bytes`.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Download, verify and install a catalog theme, making it active.
///
/// The download must match the catalog entry's sha256 and contain a JSON
/// object with the four theme fields. The stored record also keeps the id and
/// name so the installed list survives restarts without the catalog.
pub async fn install_theme_from_catalog<F: CatalogFetch>(
    vault: &Path,
    entry: ThemeCatalogEntry,
    fetch: &F,
) -> Result<InstalledTheme, String> {
    // Validate the id before downloading so a bad entry cannot leave files.
    theme_path(vault, &entry.id)?;
    let expected = entry.sha256.trim().to_ascii_lowercase();
    if expected.len() != 64 || !expected.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(THEME_CHECKSUM_MISMATCH.to_string());
    }

    let bytes = fetch
        .fetch(&entry.download_url)
        .await
        .map_err(|_| THEME_DOWNLOAD_ERROR.to_string())?;
    if sha256_hex(&bytes) != expected {
        return Err(THEME_CHECKSUM_MISMATCH.to_string());
    }

    let document: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| THEME_SCHEMA_INVALID.to_string())?;
    if !document.is_object() {
        return Err(THEME_SCHEMA_INVALID.to_string());
    }
    let values: ThemeValues =
        serde_json::from_value(document).map_err(|_| THEME_SCHEMA_INVALID.to_string())?;

    let stored = StoredTheme {
        id: entry.id.clone(),
        name: entry.name.clone(),
        values: values.clone(),
    };
    write_stored(vault, &stored)?;
    set_active(vault, &entry.id)?;
    Ok(InstalledTheme {
        id: entry.id,
        name: entry.name,
        values,
        active: true,
    })
}

impl StratumService {
    /// List installed themes, marking the active one.
    pub async fn list_installed_themes(&self) -> Result<Vec<InstalledTheme>, String> {
        Ok(list_installed_themes(&self.vault_path()))
    }

    /// Install a catalog theme from its `download_url` and make it active.
    ///
    /// The change is recorded for the auto-commit engine.
    pub async fn install_theme(&self, entry: ThemeCatalogEntry) -> Result<InstalledTheme, String> {
        let vault = self.vault_path();
        let installed = install_theme_from_catalog(&vault, entry, &ReqwestFetch).await?;
        if let Ok(mut state) = self.lock() {
            state.record_change(&format!(".pkm/themes/{}.json", installed.id));
        }
        Ok(installed)
    }

    /// Apply an installed theme by id, returning its values.
    ///
    /// [`BUILTIN_THEME_ID`] falls back to the built-in theme.
    pub async fn apply_theme(&self, id: String) -> Result<ThemeValues, String> {
        apply_theme(&self.vault_path(), &id)
    }

    /// Uninstall a theme; removing the active theme falls back to the built-in.
    pub async fn uninstall_theme(&self, id: String) -> Result<(), String> {
        let vault = self.vault_path();
        uninstall_theme(&vault, &id)?;
        if let Ok(mut state) = self.lock() {
            state.record_change(&format!(".pkm/themes/{id}.json"));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "themes_tests.rs"]
mod tests;
