use std::collections::HashMap;
use std::path::Path;

use pkm_core::config::PluginConfig;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::permissions::PermissionSet;
#[cfg(test)]
use crate::permissions::Permission;

/// Metadata describing a plugin's identity and capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub permissions: PermissionSet,
    pub entry_point: String,
    pub hooks: Vec<String>,
}

/// The runtime state of a loaded plugin.
#[derive(Debug, Clone)]
pub struct PluginState {
    pub manifest: PluginManifest,
    pub enabled: bool,
    pub wasm_bytes: Vec<u8>,
}

impl PluginState {
    /// Create a new `PluginState` from its components.
    pub fn new(manifest: PluginManifest, wasm_bytes: Vec<u8>, enabled: bool) -> Self {
        Self {
            manifest,
            enabled,
            wasm_bytes,
        }
    }
}

/// Parsed representation of a raw manifest (as read from a JSON/YAML sidecar).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawManifest {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default = "default_entry_point")]
    pub entry_point: String,
    #[serde(default)]
    pub hooks: Vec<String>,
}

fn default_version() -> String {
    "0.2.0".to_string()
}
fn default_entry_point() -> String {
    "plugin.wasm".to_string()
}

impl RawManifest {
    /// Convert a `RawManifest` into a validated `PluginManifest`.
    pub fn into_manifest(self) -> PluginManifest {
        let permissions = PermissionSet::from_config_strings(&self.permissions);
        PluginManifest {
            name: self.name,
            version: self.version,
            author: self.author,
            description: self.description,
            permissions,
            entry_point: self.entry_point,
            hooks: self.hooks,
        }
    }
}

// ---------------------------------------------------------------------------
// PluginRegistry
// ---------------------------------------------------------------------------

/// Manages the lifecycle of plugins: discovery, loading, enabling, disabling.
pub struct PluginRegistry {
    plugins: HashMap<String, PluginState>,
}

impl PluginRegistry {
    /// Create an empty plugin registry.
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Discover and load plugins from a list of `PluginConfig` entries.
    ///
    /// Each config entry specifies a name, WASM file path, and permissions.
    /// The manifest is expected as a JSON sidecar at `<wasm_path>.manifest.json`.
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

            let manifest_path = wasm_path.with_extension("wasm.manifest.json");
            let manifest = if manifest_path.exists() {
                let bytes = std::fs::read(&manifest_path)?;
                let raw: RawManifest = serde_json::from_slice(&bytes)?;
                raw.into_manifest()
            } else {
                // Build a minimal manifest from the PluginConfig
                PluginManifest {
                    name: cfg.name.clone(),
                    version: "0.2.0".to_string(),
                    author: String::new(),
                    description: String::new(),
                    permissions: PermissionSet::from_config_strings(&cfg.permissions),
                    entry_point: cfg
                        .wasm_path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "plugin.wasm".to_string()),
                    hooks: Vec::new(),
                }
            };

            let wasm_bytes = std::fs::read(wasm_path)?;

            let state = PluginState::new(manifest, wasm_bytes, cfg.enabled);
            registry.plugins.insert(cfg.name.clone(), state);
        }
        Ok(registry)
    }

    /// Load a single plugin from a WASM file path.
    ///
    /// The manifest is expected at `<wasm_path>.manifest.json`. Returns the
    /// parsed `PluginManifest`.
    pub fn load_plugin(&mut self, wasm_path: &Path) -> Result<PluginManifest, RegistryError> {
        if !wasm_path.exists() {
            return Err(RegistryError::NotFound(format!(
                "WASM file not found: {}",
                wasm_path.display()
            )));
        }

        let manifest_path = wasm_path.with_extension("wasm.manifest.json");
        let raw_bytes = std::fs::read(&manifest_path)?;
        let raw: RawManifest = serde_json::from_slice(&raw_bytes)?;
        let manifest = raw.into_manifest();
        let name = manifest.name.clone();

        let wasm_bytes = std::fs::read(wasm_path)?;

        let state = PluginState::new(manifest.clone(), wasm_bytes, true);
        self.plugins.insert(name, state);

        info!("Loaded plugin: {}", manifest.name);
        Ok(manifest)
    }

    /// Enable an already-loaded plugin by name.
    pub fn enable(&mut self, name: &str) -> Result<(), RegistryError> {
        let state = self
            .plugins
            .get_mut(name)
            .ok_or_else(|| RegistryError::NotFound(format!("Plugin '{}' not loaded", name)))?;
        state.enabled = true;
        info!("Enabled plugin: {}", name);
        Ok(())
    }

    /// Disable a loaded plugin by name.
    pub fn disable(&mut self, name: &str) {
        if let Some(state) = self.plugins.get_mut(name) {
            state.enabled = false;
            info!("Disabled plugin: {}", name);
        }
    }

    /// List the current state of all loaded plugins.
    pub fn list(&self) -> Vec<&PluginState> {
        self.plugins.values().collect()
    }

    /// List only enabled plugins.
    pub fn list_enabled(&self) -> Vec<&PluginState> {
        self.plugins
            .values()
            .filter(|s| s.enabled)
            .collect()
    }

    /// Unload (remove) a plugin from the registry.
    pub fn unload(&mut self, name: &str) {
        if self.plugins.remove(name).is_some() {
            info!("Unloaded plugin: {}", name);
        }
    }

    /// Get a reference to a loaded plugin's state.
    pub fn get(&self, name: &str) -> Option<&PluginState> {
        self.plugins.get(name)
    }

    /// Get a mutable reference to a loaded plugin's state.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut PluginState> {
        self.plugins.get_mut(name)
    }

    /// Return the number of loaded plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
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
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_manifest(dir: &TempDir, name: &str, permissions: &[&str], hooks: &[&str]) {
        let raw = RawManifest {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            author: "Test Author".to_string(),
            description: "A test plugin".to_string(),
            permissions: permissions.iter().map(|s| s.to_string()).collect(),
            entry_point: format!("{}.wasm", name),
            hooks: hooks.iter().map(|s| s.to_string()).collect(),
        };
        let json = serde_json::to_string_pretty(&raw).unwrap();
        let manifest_path = dir.path().join(format!("{}.wasm.manifest.json", name));
        std::fs::write(&manifest_path, json).unwrap();
    }

    fn write_dummy_wasm(dir: &TempDir, name: &str) {
        let wasm_path = dir.path().join(format!("{}.wasm", name));
        // Minimal valid WASM binary (empty module)
        let wasm_bytes = wasm::MINIMAL_WASM_MODULE;
        std::fs::write(&wasm_path, wasm_bytes).unwrap();
    }

    mod wasm {
        /// Minimal valid WASM module (binary encoding of an empty module).
        /// Structure:
        ///   - Magic number: 0x00 0x61 0x73 0x6D
        ///   - Version: 0x01 0x00 0x00 0x00
        /// (8 bytes total — the shortest valid WASM binary)
        pub const MINIMAL_WASM_MODULE: &[u8] = &[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
    }

    #[test]
    fn test_create_manifest() {
        let manifest = PluginManifest {
            name: "test-plugin".to_string(),
            version: "0.2.0".to_string(),
            author: "Alice".to_string(),
            description: "A test".to_string(),
            permissions: PermissionSet::from_permissions(&[Permission::FileRead]),
            entry_point: "plugin.wasm".to_string(),
            hooks: vec!["onSave".to_string()],
        };
        assert_eq!(manifest.name, "test-plugin");
        assert_eq!(manifest.hooks, vec!["onSave"]);
        assert!(manifest.permissions.check(&Permission::FileRead));
        assert!(!manifest.permissions.check(&Permission::Network));
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
        write_manifest(&dir, "alpha", &["file:read", "network"], &["onSave"]);
        write_dummy_wasm(&dir, "alpha");

        let configs = vec![PluginConfig {
            name: "alpha".to_string(),
            enabled: true,
            wasm_path: dir.path().join("alpha.wasm"),
            permissions: vec!["file:read".to_string(), "network".to_string()],
        }];

        let registry = PluginRegistry::from_config(&configs).unwrap();
        assert_eq!(registry.len(), 1);

        let state = registry.get("alpha").unwrap();
        assert!(state.enabled);
        assert_eq!(state.manifest.name, "alpha");
        assert_eq!(state.manifest.version, "1.0.0");
        assert!(state.manifest.permissions.check(&Permission::FileRead));
        assert!(state.manifest.permissions.check(&Permission::Network));
    }

    #[test]
    fn test_enable_disable() {
        let dir = TempDir::new().unwrap();
        write_manifest(&dir, "test", &[], &[]);
        write_dummy_wasm(&dir, "test");

        let mut registry = PluginRegistry::new();
        registry
            .load_plugin(&dir.path().join("test.wasm"))
            .unwrap();

        assert!(registry.get("test").unwrap().enabled);

        registry.disable("test");
        assert!(!registry.get("test").unwrap().enabled);

        registry.enable("test").unwrap();
        assert!(registry.get("test").unwrap().enabled);
    }

    #[test]
    fn test_list_states() {
        let dir = TempDir::new().unwrap();
        write_manifest(&dir, "a", &[], &[]);
        write_manifest(&dir, "b", &[], &[]);
        write_dummy_wasm(&dir, "a");
        write_dummy_wasm(&dir, "b");

        let mut registry = PluginRegistry::new();
        registry.load_plugin(&dir.path().join("a.wasm")).unwrap();
        registry.load_plugin(&dir.path().join("b.wasm")).unwrap();

        assert_eq!(registry.list().len(), 2);
        assert_eq!(registry.list_enabled().len(), 2);

        registry.disable("a");
        assert_eq!(registry.list_enabled().len(), 1);
    }

    #[test]
    fn test_unload() {
        let dir = TempDir::new().unwrap();
        write_manifest(&dir, "x", &[], &[]);
        write_dummy_wasm(&dir, "x");

        let mut registry = PluginRegistry::new();
        registry.load_plugin(&dir.path().join("x.wasm")).unwrap();
        assert_eq!(registry.len(), 1);

        registry.unload("x");
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
        let raw = RawManifest {
            name: "raw-test".to_string(),
            version: "2.0.0".to_string(),
            author: "Bob".to_string(),
            description: "From raw".to_string(),
            permissions: vec!["exec".to_string()],
            entry_point: "custom.wasm".to_string(),
            hooks: vec!["onOpen".to_string()],
        };
        let manifest = raw.into_manifest();
        assert_eq!(manifest.name, "raw-test");
        assert_eq!(manifest.version, "2.0.0");
        assert!(manifest.permissions.check(&Permission::Exec));
        assert!(manifest.hooks.contains(&"onOpen".to_string()));
    }

    #[test]
    fn test_plugin_state_new() {
        let manifest = PluginManifest {
            name: "state-test".to_string(),
            version: "0.2.0".to_string(),
            author: String::new(),
            description: String::new(),
            permissions: PermissionSet::new(),
            entry_point: "p.wasm".to_string(),
            hooks: vec![],
        };
        let state = PluginState::new(manifest.clone(), vec![0u8; 16], false);
        assert!(!state.enabled);
        assert_eq!(state.wasm_bytes.len(), 16);
        assert_eq!(state.manifest.name, "state-test");
    }
}
