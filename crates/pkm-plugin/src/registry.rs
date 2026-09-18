use crate::permissions::{Permission, PermissionSet};
use pkm_core::config::PluginConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Metadata describing a plugin's identity and capabilities.
///
/// Concrete Rust type per `docs/advanced/plugins.md` §2.5 (normative). The
/// runtime is keyed on [`Self::id`]; `name` is retained as the display name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub schema_version: u32,
    /// Canonical reverse-DNS id (ABI-frozen identity; e.g. `com.example.my-plugin`).
    pub id: String,
    /// Display name.
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_entry")]
    pub entry: String,
    pub permissions: PermissionSet,
    /// Hook enablement map: key `on<Name>`, value enabled (spec §8).
    #[serde(default)]
    pub hooks: HashMap<String, bool>,
}

fn default_entry() -> String {
    "plugin.wasm".to_string()
}

impl PluginManifest {
    /// Validate the manifest per spec §2.3. Returns a human-readable error on
    /// the first violated rule.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "unsupported schema_version {}",
                self.schema_version
            ));
        }
        let id_ok = self.id.len() <= 128
            && !self.id.is_empty()
            && self
                .id
                .chars()
                .next()
                .map(|c| c.is_ascii_alphanumeric())
                .unwrap_or(false)
            && self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
        if !id_ok {
            return Err(format!("invalid plugin id `{}`", self.id));
        }
        if self.name.is_empty() || self.name.chars().count() > 64 {
            return Err(format!("invalid plugin name `{}`", self.name));
        }
        if self.version.is_empty() || self.version.chars().count() > 32 {
            return Err(format!("invalid plugin version `{}`", self.version));
        }
        if self.description.chars().count() > 512 {
            return Err("description exceeds 512 chars".to_string());
        }
        if self.author.chars().count() > 128 {
            return Err("author exceeds 128 chars".to_string());
        }
        // Unknown permissions are rejected (spec §2.3 "Unknown permission ⇒ ConfigError").
        for p in self.permission_names() {
            if Permission::parse(&p).is_none() {
                return Err(format!("unknown permission `{}`", p));
            }
        }
        Ok(())
    }

    /// Raw permission strings as declared in the manifest.
    pub fn permission_names(&self) -> Vec<String> {
        self.permissions.iter().map(|p| p.to_string()).collect()
    }

    /// Whether hook `name` is declared AND enabled.
    pub fn hook_enabled(&self, name: &str) -> bool {
        self.hooks.get(name).copied().unwrap_or(false)
    }

    /// Names of hooks that are declared AND enabled (spec §8), sorted for
    /// deterministic output. Used to surface hook capability in the UI.
    pub fn enabled_hook_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .hooks
            .iter()
            .filter(|(_, enabled)| **enabled)
            .map(|(name, _)| name.clone())
            .collect();
        names.sort();
        names
    }

    /// The guest export for a hook (spec §8: `register_hook("onSave") ⇒ "onSave"`).
    pub fn hook_export(name: &str) -> String {
        normalize_hook_export(name)
    }
}

/// Normalize a hook name to its guest-export form (e.g. `on_save` -> `onSave`).
pub fn normalize_hook_export(hook: &str) -> String {
    match hook {
        "on_save" => "onSave",
        "on_open" => "onOpen",
        "on_link" => "onLink",
        "on_search" => "onSearch",
        _ => hook,
    }
    .to_string()
}

/// The runtime state of a loaded plugin.
#[derive(Debug, Clone)]
pub struct PluginState {
    pub manifest: PluginManifest,
    pub enabled: bool,
    pub wasm_bytes: Vec<u8>,
    /// Where the plugin was loaded from (vault-relative source dir).
    pub source_dir: PathBuf,
}

impl PluginState {
    /// Create a new `PluginState` from its components.
    pub fn new(manifest: PluginManifest, wasm_bytes: Vec<u8>, enabled: bool) -> Self {
        Self {
            manifest,
            enabled,
            wasm_bytes,
            source_dir: PathBuf::new(),
        }
    }

    /// Create a new `PluginState` with an explicit source directory.
    pub fn new_in(
        manifest: PluginManifest,
        wasm_bytes: Vec<u8>,
        enabled: bool,
        source_dir: PathBuf,
    ) -> Self {
        Self {
            manifest,
            enabled,
            wasm_bytes,
            source_dir,
        }
    }
}

/// Parsed representation of a raw manifest (legacy/on-disk format).
///
/// Accepts both the legacy `name` key and the canonical `id` key; canonical
/// manifests carry `id`. The `permissions` field is the string list form per
/// §2.3 — conversion into a [`PermissionSet`] validates it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawManifest {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_entry")]
    pub entry: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub hooks: HashMap<String, bool>,
}

fn default_schema_version() -> u32 {
    1
}
fn default_version() -> String {
    "0.2.0".to_string()
}

impl RawManifest {
    /// Convert a `RawManifest` into a validated `PluginManifest`.
    ///
    /// Rejects unknown permission strings (spec §2.3: "Unknown permission ⇒
    /// ConfigError").
    pub fn into_manifest(self) -> Result<PluginManifest, String> {
        for p in &self.permissions {
            if Permission::parse(p).is_none() {
                return Err(format!("unknown permission `{}`", p));
            }
        }
        // Canonical manifests use `id`; legacy sidecars use `name` as identity.
        let id = self.id.unwrap_or_else(|| self.name.clone());
        let permissions = PermissionSet::from_config_strings(&self.permissions);
        let manifest = PluginManifest {
            schema_version: self.schema_version,
            id,
            name: self.name,
            version: self.version,
            author: self.author,
            description: self.description,
            entry: self.entry,
            permissions,
            hooks: self.hooks,
        };
        manifest.validate()?;
        Ok(manifest)
    }
}

// ---------------------------------------------------------------------------
// Embedded manifest extraction
// ---------------------------------------------------------------------------

/// Extract the embedded `stratum:manifest` custom section from WASM bytes.
///
/// The WASM binary format is parsed minimally: we walk the top-level sections,
/// skipping every section that is not the `stratum:manifest` custom section.
/// Returns `Ok(None)` when no such section exists (caller falls back to a
/// sidecar manifest, spec §2.2).
pub fn extract_embedded_manifest(wasm_bytes: &[u8]) -> Result<Option<String>, String> {
    // Magic + version (8 bytes).
    if wasm_bytes.len() < 8 {
        return Err("not a WASM module (too short)".to_string());
    }
    if &wasm_bytes[0..4] != b"\0asm" || wasm_bytes[4..8] != [0x01, 0x00, 0x00, 0x00] {
        return Err("not a WASM module (bad magic/version)".to_string());
    }
    let mut pos = 8usize;

    while pos < wasm_bytes.len() {
        let section_id = wasm_bytes[pos];
        pos += 1;
        let Some(section_len) = read_u32_le(wasm_bytes, &mut pos) else {
            return Err("malformed WASM section length".to_string());
        };
        let section_start = pos;
        let section_end = pos
            .checked_add(section_len as usize)
            .ok_or_else(|| "WASM section length overflow".to_string())?;
        if section_end > wasm_bytes.len() {
            return Err("WASM section out of bounds".to_string());
        }

        // Custom section (id 0): name is a LEB128 length + UTF-8 name.
        if section_id == 0 {
            let body = &wasm_bytes[section_start..section_end];
            let mut bpos = 0usize;
            let Some(name_len) = read_u32_le(body, &mut bpos) else {
                return Err("malformed custom section name".to_string());
            };
            let name_end = bpos + name_len as usize;
            if name_end > body.len() {
                return Err("custom section name out of bounds".to_string());
            }
            let name = String::from_utf8_lossy(&body[bpos..name_end]).into_owned();
            if name == "stratum:manifest" {
                let payload = body[name_end..].to_vec();
                return Ok(Some(String::from_utf8_lossy(&payload).into_owned()));
            }
        }

        pos = section_end;
    }
    Ok(None)
}

/// Read a little-endian u32 uLEB128 (the WASM format's unsigned integer).
fn read_u32_le(bytes: &[u8], pos: &mut usize) -> Option<u32> {
    let mut result: u32 = 0;
    let mut shift = 0u32;
    loop {
        let byte = *bytes.get(*pos)?;
        *pos += 1;
        result |= u32::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Some(result);
        }
        shift += 7;
        if shift >= 35 {
            return None;
        }
    }
}

// ---------------------------------------------------------------------------
// PluginRegistry
// ---------------------------------------------------------------------------

/// Manages the lifecycle of plugins: discovery, loading, enabling, disabling.
///
/// The registry is keyed by plugin **id** (canonical identity). It loads from
/// either an embedded `stratum:manifest` custom section (canonical) or a legacy
/// `<entry>.wasm.manifest.json` sidecar (read-only compatibility), preferring
/// the embedded manifest when both exist (spec §2.2).
pub struct PluginRegistry {
    plugins: HashMap<String, PluginState>,
    /// Plugins that failed to load, keyed by id: `id -> error`.
    /// Recorded by [`PluginRegistry::scan_vault`]; surfaced by `plugins_list`.
    failed: HashMap<String, String>,
}

impl PluginRegistry {
    /// Create an empty plugin registry.
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            failed: HashMap::new(),
        }
    }

    /// Discover and load plugins from a list of `PluginConfig` entries.
    ///
    /// Each config entry specifies a WASM file path and enable/disable state.
    /// The manifest is read from the embedded `stratum:manifest` custom section,
    /// falling back to a legacy `<wasm_path>.wasm.manifest.json` sidecar.
    pub fn from_config(configs: &[PluginConfig]) -> Result<Self, RegistryError> {
        let mut registry = Self::new();
        for cfg in configs {
            let wasm_path = &cfg.wasm_path;
            if !wasm_path.exists() {
                return Err(RegistryError::NotFound(format!(
                    "WASM file not found: {}",
                    wasm_path.display()
                )));
            }

            let wasm_bytes = std::fs::read(wasm_path)?;
            let manifest = registry.read_manifest(&wasm_bytes, wasm_path, &cfg.name)?;

            let state = PluginState::new_in(manifest, wasm_bytes, cfg.enabled, wasm_path.clone());
            registry.insert(state);
        }
        Ok(registry)
    }

    /// Load a single plugin from a WASM file path, enabled by default.
    ///
    /// Returns the parsed, validated `PluginManifest`. The plugin is keyed by
    /// manifest `id`.
    pub fn load_plugin(&mut self, wasm_path: &Path) -> Result<PluginManifest, RegistryError> {
        if !wasm_path.exists() {
            return Err(RegistryError::NotFound(format!(
                "WASM file not found: {}",
                wasm_path.display()
            )));
        }

        let wasm_bytes = std::fs::read(wasm_path)?;
        let manifest = self.read_manifest(&wasm_bytes, wasm_path, "")?;

        let state =
            PluginState::new_in(manifest.clone(), wasm_bytes, true, wasm_path.to_path_buf());
        self.insert(state);

        info!("Loaded plugin: {} ({})", manifest.id, wasm_path.display());
        Ok(manifest)
    }

    /// Install a plugin from a source WASM file into the vault plugin directory.
    ///
    /// Copies `<src>.wasm` (and an optional sibling `<src>.wasm.manifest.json`)
    /// into `<dest_plugins_dir>/<id>/` using the canonical layout (`plugin.wasm`
    /// [+ sidecar]), reading the manifest (embedded preferred, else sidecar,
    /// else the source file stem as fallback identity) to determine `id`. The
    /// plugin is registered **disabled** by default (spec §7.3: a plugin absent
    /// from the config enable list is loaded disabled). Re-installing an
    /// existing id overwrites the previous installation (upgrade path).
    ///
    /// Returns the canonical manifest of the installed plugin.
    pub fn install_from_path(
        &mut self,
        src_wasm: &Path,
        dest_plugins_dir: &Path,
    ) -> Result<PluginManifest, RegistryError> {
        if !src_wasm.is_file() {
            return Err(RegistryError::NotFound(format!(
                "source WASM not found: {}",
                src_wasm.display()
            )));
        }
        let wasm_bytes = std::fs::read(src_wasm)?;
        // Fall back to the source file stem when no manifest is present so a
        // bare `.wasm` can still be installed (a synthesized minimal manifest).
        let fallback = src_wasm
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let manifest = self.read_manifest(&wasm_bytes, src_wasm, &fallback)?;
        let id = manifest.id.clone();

        // Canonical destination: `<plugins_dir>/<id>/plugin.wasm`. The id is
        // filesystem-safe (validated: ASCII alphanumeric + `._-`).
        let dest_dir = dest_plugins_dir.join(&id);
        std::fs::create_dir_all(&dest_dir)?;
        let dest_wasm = dest_dir.join("plugin.wasm");
        std::fs::copy(src_wasm, &dest_wasm)?;
        // Copy the sidecar manifest as well when present, so the canonical
        // install is self-describing on disk.
        let src_sidecar = src_wasm.with_extension("wasm.manifest.json");
        if src_sidecar.is_file() {
            std::fs::copy(&src_sidecar, dest_dir.join("plugin.wasm.manifest.json"))?;
        }

        let state = PluginState::new_in(manifest.clone(), wasm_bytes, false, dest_dir);
        self.insert(state);
        info!(
            "Installed plugin: {} ({}) -> {}",
            id,
            manifest.name,
            dest_wasm.display()
        );
        Ok(manifest)
    }

    /// Read and validate a manifest from embedded or sidecar sources.
    fn read_manifest(
        &self,
        wasm_bytes: &[u8],
        wasm_path: &Path,
        fallback_name: &str,
    ) -> Result<PluginManifest, RegistryError> {
        // Prefer the embedded manifest (canonical format).
        if let Some(payload) =
            extract_embedded_manifest(wasm_bytes).map_err(RegistryError::Invalid)?
        {
            let raw: RawManifest = serde_json::from_str(&payload)?;
            let manifest = raw
                .into_manifest()
                .map_err(|e| RegistryError::Invalid(format!("embedded manifest: {e}")))?;
            if manifest.id.is_empty() && !fallback_name.is_empty() {
                // Legacy embedded manifests may omit `id`; fall back to the name.
                return self.build_with_id(manifest, fallback_name);
            }
            self.validate_manifest(&manifest)
                .map_err(RegistryError::Invalid)?;
            return Ok(manifest);
        }

        // Legacy sidecar (`<entry>.wasm.manifest.json` next to the .wasm).
        let manifest_path = wasm_path.with_extension("wasm.manifest.json");
        if manifest_path.exists() {
            let bytes = std::fs::read(&manifest_path)?;
            let raw: RawManifest = serde_json::from_slice(&bytes)?;
            let manifest = raw
                .into_manifest()
                .map_err(|e| RegistryError::Invalid(format!("sidecar manifest: {e}")))?;
            if manifest.id.is_empty() && !fallback_name.is_empty() {
                return self.build_with_id(manifest, fallback_name);
            }
            self.validate_manifest(&manifest)
                .map_err(RegistryError::Invalid)?;
            return Ok(manifest);
        }

        // No manifest source: build a minimal one from the config fallback name.
        if !fallback_name.is_empty() {
            let manifest = PluginManifest {
                schema_version: 1,
                id: fallback_name.to_string(),
                name: fallback_name.to_string(),
                version: "0.2.0".to_string(),
                author: String::new(),
                description: String::new(),
                entry: default_entry(),
                permissions: PermissionSet::new(),
                hooks: HashMap::new(),
            };
            self.validate_manifest(&manifest)
                .map_err(RegistryError::Invalid)?;
            return Ok(manifest);
        }

        Err(RegistryError::Invalid(
            "no manifest found (embedded or sidecar)".to_string(),
        ))
    }

    /// Fill a canonical id/name from a config fallback entry.
    fn build_with_id(
        &self,
        mut manifest: PluginManifest,
        fallback: &str,
    ) -> Result<PluginManifest, RegistryError> {
        if manifest.name.is_empty() {
            manifest.name = fallback.to_string();
        }
        if manifest.id.is_empty() {
            manifest.id = manifest.name.clone();
        }
        self.validate_manifest(&manifest)
            .map_err(RegistryError::Invalid)?;
        Ok(manifest)
    }

    fn validate_manifest(&self, manifest: &PluginManifest) -> Result<(), String> {
        manifest.validate()
    }

    /// Insert a plugin state into the registry, keyed by its id.
    fn insert(&mut self, state: PluginState) {
        let id = state.manifest.id.clone();
        self.plugins.insert(id, state);
    }

    /// Enable an already-loaded plugin by id.
    pub fn enable(&mut self, id: &str) -> Result<(), RegistryError> {
        let state = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| RegistryError::NotFound(format!("Plugin '{}' not loaded", id)))?;
        state.enabled = true;
        info!("Enabled plugin: {}", id);
        Ok(())
    }

    /// Disable a loaded plugin by id.
    pub fn disable(&mut self, id: &str) {
        if let Some(state) = self.plugins.get_mut(id) {
            state.enabled = false;
            info!("Disabled plugin: {}", id);
        }
    }

    /// List the current state of all loaded plugins.
    pub fn list(&self) -> Vec<&PluginState> {
        let mut all: Vec<&PluginState> = self.plugins.values().collect();
        all.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
        all
    }

    /// List only enabled plugins.
    pub fn list_enabled(&self) -> Vec<&PluginState> {
        self.plugins.values().filter(|s| s.enabled).collect()
    }

    /// Unload (remove) a plugin from the registry.
    pub fn unload(&mut self, id: &str) {
        if self.plugins.remove(id).is_some() {
            info!("Unloaded plugin: {}", id);
        }
    }

    /// Uninstall a plugin by id: unload it from the registry, clear any
    /// recorded load failure, and remove its persisted directory from the
    /// vault plugin directory (`<plugins_dir>/<id>/`).
    ///
    /// Removing a plugin that is not loaded is still considered a success for
    /// the directory removal portion. Returns whether the directory existed.
    pub fn uninstall(&mut self, id: &str, plugins_dir: &Path) -> Result<bool, RegistryError> {
        self.plugins.remove(id);
        self.failed.remove(id);
        let dir = plugins_dir.join(id);
        let existed = dir.is_dir();
        if existed {
            std::fs::remove_dir_all(&dir)?;
            info!("Uninstalled plugin: {}", id);
        } else {
            info!("Uninstall `{}`: no plugin directory present", id);
        }
        Ok(existed)
    }

    /// Get a reference to a loaded plugin's state by id.
    pub fn get(&self, id: &str) -> Option<&PluginState> {
        self.plugins.get(id)
    }

    /// Get a mutable reference to a loaded plugin's state by id.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut PluginState> {
        self.plugins.get_mut(id)
    }

    /// Return the number of loaded plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Ids of plugins that failed to load during the last [`scan_vault`],
    /// with their error messages (spec: a failed load is visible via
    /// `plugins_list` as status `error`, contract §6.4).
    ///
    /// [`scan_vault`]: PluginRegistry::scan_vault
    pub fn failed_ids(&self) -> Vec<(String, String)> {
        let mut all: Vec<(String, String)> = self
            .failed
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        all.sort_by(|a, b| a.0.cmp(&b.0));
        all
    }

    /// Id of a plugin that failed to load, if any.
    pub fn failed_error(&self, id: &str) -> Option<&str> {
        self.failed.get(id).map(|s| s.as_str())
    }

    /// Record (or update) a plugin load failure in the failed map.
    pub fn record_failure(&mut self, id: impl Into<String>, error: impl Into<String>) {
        self.failed.insert(id.into(), error.into());
    }

    /// Remove a recorded failure for a plugin id (successful reload clears it).
    pub fn clear_failure(&mut self, id: &str) {
        self.failed.remove(id);
    }

    /// Scan a vault plugin directory (`<vault>/.pkm/plugins/`) and load every
    /// discovered plugin (spec §7.3).
    ///
    /// Layout: `<vault>/.pkm/plugins/<name>/plugin.wasm` (embedded manifest),
    /// or `<vault>/.pkm/plugins/<name>/<entry>.wasm.manifest.json` (legacy).
    /// Load failures are logged and skipped — a broken plugin must never abort
    /// loading the rest (spec §6.4, §7.2).
    ///
    /// `enabled_ids` is the set of plugin ids the vault config lists as enabled.
    /// A discovered plugin is enabled iff its id is in the set.
    pub fn scan_vault(
        &mut self,
        plugins_dir: &Path,
        enabled_ids: &std::collections::HashSet<String>,
    ) -> usize {
        if !plugins_dir.is_dir() {
            return 0;
        }
        let mut loaded = 0usize;
        let Ok(entries) = std::fs::read_dir(plugins_dir) else {
            return 0;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let mut wasm_path = path.join("plugin.wasm");
            if !wasm_path.exists() {
                // Legacy layout: a single `<name>.wasm` plus sidecar manifest.
                let legacy = path.join(format!(
                    "{}.wasm",
                    path.file_name()
                        .map(|s| s.to_string_lossy())
                        .unwrap_or_default()
                ));
                if legacy.exists() {
                    wasm_path = legacy;
                } else {
                    continue;
                }
            }
            let manifest = match self.load_plugin(&wasm_path) {
                Ok(m) => {
                    // A successful reload from disk clears any previous failure.
                    self.failed.remove(&m.id);
                    m
                }
                Err(e) => {
                    warn!(
                        "[stratum] Failed to load plugin at {}: {e}",
                        wasm_path.display()
                    );
                    // A plugin that fails to load is still visible (status
                    // `error`) through `plugins_list` (spec §6.4, §7.2); it is
                    // never replaced by a partial plugin.
                    if let Some(id) = wasm_path
                        .parent()
                        .and_then(|d| d.file_name())
                        .map(|s| s.to_string_lossy().into_owned())
                    {
                        self.record_failure(id, e.to_string());
                    }
                    continue;
                }
            };
            let enabled = enabled_ids.contains(&manifest.id);
            // load_plugin enables by default; align with the config.
            if let Some(state) = self.plugins.get_mut(&manifest.id) {
                state.enabled = enabled;
            }
            loaded += 1;
        }
        loaded
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// RegistryError
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Registry error: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON deserialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid plugin: {0}")]
    Invalid(String),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Minimal valid WASM module (empty module).
    pub const MINIMAL_WASM_MODULE: &[u8] = &[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

    fn write_sidecar(dir: &TempDir, name: &str, id: &str, permissions: &[&str], hooks: &[&str]) {
        let mut hookmap = HashMap::new();
        for h in hooks {
            hookmap.insert((*h).to_string(), true);
        }
        let raw = RawManifest {
            schema_version: 1,
            id: Some(id.to_string()),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            author: "Test Author".to_string(),
            description: "A test plugin".to_string(),
            entry: format!("{}.wasm", name),
            permissions: permissions.iter().map(|s| s.to_string()).collect(),
            hooks: hookmap,
        };
        let json = serde_json::to_string_pretty(&raw).unwrap();
        let manifest_path = dir.path().join(format!("{}.wasm.manifest.json", name));
        std::fs::write(&manifest_path, json).unwrap();
    }

    fn write_dummy_wasm(dir: &TempDir, name: &str) {
        let wasm_path = dir.path().join(format!("{}.wasm", name));
        std::fs::write(&wasm_path, MINIMAL_WASM_MODULE).unwrap();
    }

    #[test]
    fn test_create_manifest() {
        let mut hooks = HashMap::new();
        hooks.insert("onSave".to_string(), true);
        let manifest = PluginManifest {
            schema_version: 1,
            id: "com.example.test".to_string(),
            name: "test-plugin".to_string(),
            version: "0.2.0".to_string(),
            author: "Alice".to_string(),
            description: "A test".to_string(),
            entry: "plugin.wasm".to_string(),
            permissions: PermissionSet::from_permissions(&[Permission::FileRead]),
            hooks,
        };
        assert_eq!(manifest.id, "com.example.test");
        assert_eq!(manifest.name, "test-plugin");
        assert!(manifest.hook_enabled("onSave"));
        assert!(!manifest.hook_enabled("onOpen"));
        assert!(manifest.permissions.check(&Permission::FileRead));
        assert!(!manifest.permissions.check(&Permission::Network));
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_manifest_validate_rejects_bad_id() {
        let m = PluginManifest {
            schema_version: 1,
            id: "bad id with spaces!".to_string(),
            name: "x".to_string(),
            version: "1.0.0".to_string(),
            author: String::new(),
            description: String::new(),
            entry: "plugin.wasm".to_string(),
            permissions: PermissionSet::new(),
            hooks: HashMap::new(),
        };
        assert!(m.validate().is_err());
    }

    #[test]
    fn test_manifest_validate_rejects_unknown_permission() {
        // `PermissionSet` silently drops unknown strings at construction, so an
        // unknown permission must be rejected at the raw-manifest boundary
        // (`RawManifest::into_manifest`), which validates the raw strings.
        let raw = RawManifest {
            schema_version: 1,
            id: Some("com.example.x".to_string()),
            name: "x".to_string(),
            version: "1.0.0".to_string(),
            author: String::new(),
            description: String::new(),
            entry: "plugin.wasm".to_string(),
            permissions: vec!["file:read".to_string(), "bogus".to_string()],
            hooks: HashMap::new(),
        };
        let err = raw.into_manifest().unwrap_err();
        assert!(
            err.contains("bogus"),
            "error should name the bad permission: {err}"
        );
    }

    #[test]
    fn test_registry_new_and_empty() {
        let registry = PluginRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_from_config() {
        let dir = TempDir::new().unwrap();
        write_sidecar(
            &dir,
            "alpha",
            "com.example.alpha",
            &["file:read", "network"],
            &["onSave"],
        );
        write_dummy_wasm(&dir, "alpha");

        let configs = vec![PluginConfig {
            name: "alpha".to_string(),
            enabled: true,
            wasm_path: dir.path().join("alpha.wasm"),
            permissions: vec!["file:read".to_string(), "network".to_string()],
        }];

        let registry = PluginRegistry::from_config(&configs).unwrap();
        assert_eq!(registry.len(), 1);

        let state = registry.get("com.example.alpha").unwrap();
        assert!(state.enabled);
        assert_eq!(state.manifest.id, "com.example.alpha");
        assert_eq!(state.manifest.name, "alpha");
        assert_eq!(state.manifest.version, "1.0.0");
        assert!(state.manifest.permissions.check(&Permission::FileRead));
        assert!(state.manifest.permissions.check(&Permission::Network));
    }

    #[test]
    fn test_enable_disable() {
        let dir = TempDir::new().unwrap();
        write_sidecar(&dir, "test", "com.example.test", &[], &[]);
        write_dummy_wasm(&dir, "test");

        let mut registry = PluginRegistry::new();
        registry.load_plugin(&dir.path().join("test.wasm")).unwrap();

        assert!(registry.get("com.example.test").unwrap().enabled);

        registry.disable("com.example.test");
        assert!(!registry.get("com.example.test").unwrap().enabled);

        registry.enable("com.example.test").unwrap();
        assert!(registry.get("com.example.test").unwrap().enabled);
    }

    #[test]
    fn test_list_states() {
        let dir = TempDir::new().unwrap();
        write_sidecar(&dir, "a", "com.example.a", &[], &[]);
        write_sidecar(&dir, "b", "com.example.b", &[], &[]);
        write_dummy_wasm(&dir, "a");
        write_dummy_wasm(&dir, "b");

        let mut registry = PluginRegistry::new();
        registry.load_plugin(&dir.path().join("a.wasm")).unwrap();
        registry.load_plugin(&dir.path().join("b.wasm")).unwrap();

        assert_eq!(registry.list().len(), 2);
        assert_eq!(registry.list_enabled().len(), 2);

        registry.disable("com.example.a");
        assert_eq!(registry.list_enabled().len(), 1);
    }

    #[test]
    fn test_unload() {
        let dir = TempDir::new().unwrap();
        write_sidecar(&dir, "x", "com.example.x", &[], &[]);
        write_dummy_wasm(&dir, "x");

        let mut registry = PluginRegistry::new();
        registry.load_plugin(&dir.path().join("x.wasm")).unwrap();
        assert_eq!(registry.len(), 1);

        registry.unload("com.example.x");
        assert!(registry.is_empty());
    }

    #[test]
    fn test_load_plugin_not_found() {
        let mut registry = PluginRegistry::new();
        let err = registry
            .load_plugin(Path::new("/nonexistent/plugin.wasm"))
            .unwrap_err();
        assert!(matches!(err, RegistryError::NotFound(_)));
    }

    #[test]
    fn test_enable_not_found() {
        let mut registry = PluginRegistry::new();
        let err = registry.enable("ghost");
        assert!(err.is_err());
    }

    #[test]
    fn test_disable_non_existent_is_noop() {
        let mut registry = PluginRegistry::new();
        registry.disable("ghost"); // should not panic
        assert!(registry.is_empty());
    }

    #[test]
    fn test_raw_manifest_into_manifest() {
        let mut hooks = HashMap::new();
        hooks.insert("onOpen".to_string(), true);
        let raw = RawManifest {
            schema_version: 1,
            id: Some("com.example.raw".to_string()),
            name: "raw-test".to_string(),
            version: "2.0.0".to_string(),
            author: "Bob".to_string(),
            description: "From raw".to_string(),
            entry: "custom.wasm".to_string(),
            permissions: vec!["exec".to_string()],
            hooks,
        };
        let manifest = raw.into_manifest().unwrap();
        assert_eq!(manifest.name, "raw-test");
        assert_eq!(manifest.version, "2.0.0");
        assert!(manifest.permissions.check(&Permission::Exec));
        assert!(manifest.hook_enabled("onOpen"));
    }

    #[test]
    fn test_plugin_state_new() {
        let manifest = PluginManifest {
            schema_version: 1,
            id: "com.example.state".to_string(),
            name: "state-test".to_string(),
            version: "0.2.0".to_string(),
            author: String::new(),
            description: String::new(),
            entry: "p.wasm".to_string(),
            permissions: PermissionSet::new(),
            hooks: HashMap::new(),
        };
        let state = PluginState::new(manifest.clone(), vec![0u8; 16], false);
        assert!(!state.enabled);
        assert_eq!(state.wasm_bytes.len(), 16);
        assert_eq!(state.manifest.id, "com.example.state");
    }

    #[test]
    fn test_extract_embedded_manifest_absent() {
        let out = extract_embedded_manifest(MINIMAL_WASM_MODULE).unwrap();
        assert!(out.is_none());
    }

    /// Build a minimal WASM binary that embeds a `stratum:manifest` custom section.
    /// The custom section payload begins with the (LEB128) name length + name +
    /// the JSON manifest bytes.
    fn wasm_with_embedded_manifest(json: &str) -> Vec<u8> {
        // LEB128-encode an unsigned integer (the WASM section/payload sizes use
        // ULEB128, and JSON manifests routinely exceed 127 bytes).
        fn uleb128(mut v: usize) -> Vec<u8> {
            let mut out = Vec::new();
            loop {
                let mut byte = (v & 0x7f) as u8;
                v >>= 7;
                if v != 0 {
                    byte |= 0x80;
                }
                out.push(byte);
                if v == 0 {
                    break;
                }
            }
            out
        }

        let name = "stratum:manifest".as_bytes();
        let mut payload = Vec::new();
        payload.extend_from_slice(&uleb128(name.len()));
        payload.extend_from_slice(name);
        payload.extend_from_slice(json.as_bytes());

        let mut section = vec![0x00u8]; // custom section id
        section.extend_from_slice(&uleb128(payload.len()));
        section.extend_from_slice(&payload);

        let mut out = MINIMAL_WASM_MODULE.to_vec();
        out.extend_from_slice(&section);
        out
    }

    #[test]
    fn test_extract_embedded_manifest_present() {
        let json = r#"{"schema_version":1,"id":"com.example.emb","name":"Emb","version":"1.0.0","permissions":["file:read"],"hooks":{"onSave":true}}"#;
        let wasm = wasm_with_embedded_manifest(json);
        let out = extract_embedded_manifest(&wasm).unwrap();
        assert!(out.is_some());
        let manifest_raw: RawManifest = serde_json::from_str(out.as_deref().unwrap()).unwrap();
        let manifest = manifest_raw.into_manifest().unwrap();
        assert_eq!(manifest.id, "com.example.emb");
        assert!(manifest.hook_enabled("onSave"));
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_load_plugin_prefers_embedded_manifest() {
        let dir = TempDir::new().unwrap();
        let json = r#"{"schema_version":1,"id":"com.example.pref","name":"Pref","version":"3.0.0","permissions":[],"hooks":{}}"#;
        let embedded = wasm_with_embedded_manifest(json);
        let wasm_path = dir.path().join("plugin.wasm");
        std::fs::write(&wasm_path, &embedded).unwrap();

        let mut registry = PluginRegistry::new();
        let manifest = registry.load_plugin(&wasm_path).unwrap();
        assert_eq!(manifest.id, "com.example.pref");
        assert_eq!(manifest.version, "3.0.0");
    }

    #[test]
    fn test_scan_vault_loads_plugins() {
        let dir = TempDir::new().unwrap();
        // Canonical layout: .pkm/plugins/<name>/plugin.wasm with embedded manifest.
        let plugins_root = dir.path().join(".pkm").join("plugins");
        let plugin_dir = plugins_root.join("myplugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        let json = r#"{"schema_version":1,"id":"com.example.scan","name":"Scan","version":"1.0.0","permissions":["file:read"],"hooks":{"onSave":true}}"#;
        std::fs::write(
            plugin_dir.join("plugin.wasm"),
            wasm_with_embedded_manifest(json),
        )
        .unwrap();

        let mut registry = PluginRegistry::new();
        let enabled: std::collections::HashSet<String> =
            ["com.example.scan".to_string()].into_iter().collect();
        let count = registry.scan_vault(&plugins_root, &enabled);
        assert_eq!(count, 1);
        let state = registry.get("com.example.scan").unwrap();
        assert!(state.enabled);
        assert_eq!(state.manifest.name, "Scan");
    }

    #[test]
    fn test_scan_vault_honors_disable() {
        let dir = TempDir::new().unwrap();
        let plugins_root = dir.path().join(".pkm").join("plugins");
        let plugin_dir = plugins_root.join("myplugin");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        let json = r#"{"schema_version":1,"id":"com.example.scan2","name":"Scan2","version":"1.0.0","permissions":[],"hooks":{}}"#;
        std::fs::write(
            plugin_dir.join("plugin.wasm"),
            wasm_with_embedded_manifest(json),
        )
        .unwrap();

        let mut registry = PluginRegistry::new();
        // Config lists it as disabled (not in the enabled set).
        let enabled: std::collections::HashSet<String> = std::collections::HashSet::new();
        let count = registry.scan_vault(&plugins_root, &enabled);
        assert_eq!(count, 1);
        assert!(!registry.get("com.example.scan2").unwrap().enabled);
    }

    #[test]
    fn test_manifest_serialization_roundtrip() {
        let mut hooks = HashMap::new();
        hooks.insert("onSave".to_string(), true);
        let json = serde_json::to_string(&PluginManifest {
            schema_version: 1,
            id: "com.example.round".to_string(),
            name: "Round".to_string(),
            version: "1.0.0".to_string(),
            author: String::new(),
            description: String::new(),
            entry: "plugin.wasm".to_string(),
            permissions: PermissionSet::from_config_strings(&["file:read".to_string()]),
            hooks,
        })
        .unwrap();
        let back: PluginManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "com.example.round");
        assert!(back.hook_enabled("onSave"));
    }

    // -----------------------------------------------------------------------
    // install_from_path / uninstall
    // -----------------------------------------------------------------------

    #[test]
    fn test_install_from_path_copies_and_registers_disabled() {
        let dir = TempDir::new().unwrap();
        let plugins_root = dir.path().join(".pkm").join("plugins");

        // Source: a wasm with an embedded manifest, plus a sibling source file.
        let json = r#"{"schema_version":1,"id":"com.example.inst","name":"Inst","version":"1.0.0","permissions":["file:read"],"hooks":{"onSave":true}}"#;
        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let src_wasm = src_dir.join("myplugin.wasm");
        std::fs::write(&src_wasm, wasm_with_embedded_manifest(json)).unwrap();

        let mut registry = PluginRegistry::new();
        let manifest = registry
            .install_from_path(&src_wasm, &plugins_root)
            .expect("install must succeed");
        assert_eq!(manifest.id, "com.example.inst");
        assert_eq!(manifest.name, "Inst");

        // Registered, keyed by id, disabled by default.
        assert_eq!(registry.len(), 1);
        let state = registry
            .get("com.example.inst")
            .expect("must be registered");
        assert!(
            !state.enabled,
            "new installs are disabled by default (spec §7.3)"
        );
        assert_eq!(state.manifest.entry, "plugin.wasm");

        // Files copied into the canonical canonical layout.
        let dest_wasm = plugins_root.join("com.example.inst").join("plugin.wasm");
        assert!(dest_wasm.is_file(), "canonical plugin.wasm must be copied");
        // The source wasm still exists.
        assert!(src_wasm.is_file());
    }

    #[test]
    fn test_install_from_path_missing_source_errors() {
        let dir = TempDir::new().unwrap();
        let mut registry = PluginRegistry::new();
        let missing = dir.path().join("does-not-exist.wasm");
        let err = registry
            .install_from_path(&missing, &dir.path().join("plugins"))
            .expect_err("missing source must fail");
        assert!(matches!(err, RegistryError::NotFound(_)));
    }

    #[test]
    fn test_install_reinstall_overwrites_previous() {
        let dir = TempDir::new().unwrap();
        let plugins_root = dir.path().join(".pkm").join("plugins");
        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let src_wasm = src_dir.join("upgrade.wasm");

        let v1 = r#"{"schema_version":1,"id":"com.example.upg","name":"Upg","version":"1.0.0","permissions":[],"hooks":{}}"#;
        std::fs::write(&src_wasm, wasm_with_embedded_manifest(v1)).unwrap();
        let mut registry = PluginRegistry::new();
        registry
            .install_from_path(&src_wasm, &plugins_root)
            .expect("first install");
        assert_eq!(registry.len(), 1);

        // Re-install with a newer version from the same id.
        let v2 = r#"{"schema_version":1,"id":"com.example.upg","name":"Upg","version":"2.0.0","permissions":[],"hooks":{}}"#;
        std::fs::write(&src_wasm, wasm_with_embedded_manifest(v2)).unwrap();
        registry
            .install_from_path(&src_wasm, &plugins_root)
            .expect("reinstall");
        assert_eq!(registry.len(), 1, "reinstall must not duplicate");
        let state = registry.get("com.example.upg").unwrap();
        assert_eq!(state.manifest.version, "2.0.0");
    }

    #[test]
    fn test_uninstall_removes_registry_and_directory() {
        let dir = TempDir::new().unwrap();
        let plugins_root = dir.path().join(".pkm").join("plugins");
        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let src_wasm = src_dir.join("kill.wasm");
        let json = r#"{"schema_version":1,"id":"com.example.kill","name":"Kill","version":"1.0.0","permissions":[],"hooks":{}}"#;
        std::fs::write(&src_wasm, wasm_with_embedded_manifest(json)).unwrap();

        let mut registry = PluginRegistry::new();
        registry
            .install_from_path(&src_wasm, &plugins_root)
            .expect("install before uninstall");

        assert!(plugins_root
            .join("com.example.kill")
            .join("plugin.wasm")
            .is_file());
        let existed = registry
            .uninstall("com.example.kill", &plugins_root)
            .expect("uninstall must succeed");
        assert!(existed);
        assert!(registry.is_empty());
        assert!(!plugins_root.join("com.example.kill").exists());
    }

    #[test]
    fn test_uninstall_unknown_id_is_noop() {
        let dir = TempDir::new().unwrap();
        let mut registry = PluginRegistry::new();
        let existed = registry
            .uninstall("com.example.unknown", &dir.path().join("plugins"))
            .expect("uninstall of unknown id must not error");
        assert!(!existed);
        assert!(registry.is_empty());
    }
}
