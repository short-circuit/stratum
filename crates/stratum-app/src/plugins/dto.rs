//! Plugin DTOs (normative — ADR-0004 §9).
//!
//! Command names, argument names and DTO shapes are ABI-frozen for the
//! v0.7.x series.

use pkm_plugin::PluginState;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    /// `"ready"` | `"disabled"` | `"error"`
    pub status: String,
    pub enabled: bool,
    pub permissions: Vec<String>,
    /// Hooks declared by the manifest, mapped to enablement (spec §8). Keys
    /// are `on*` hook names (`onSave`, `onOpen`, `onLink`, `onSearch`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<String>,
    /// Display metadata from the plugin manifest (spec §2).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginListResult {
    pub plugins: Vec<PluginInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginNoteDto {
    pub path: String,
    pub content: String,
    pub mtime: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginHttpResponseDto {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl PluginInfo {
    pub(crate) fn from_state(state: &PluginState) -> Self {
        let status = if state.enabled { "ready" } else { "disabled" };
        Self {
            id: state.manifest.id.clone(),
            name: state.manifest.name.clone(),
            version: state.manifest.version.clone(),
            status: status.to_string(),
            enabled: state.enabled,
            permissions: state.manifest.permission_names(),
            hooks: state.manifest.enabled_hook_names(),
            author: state.manifest.author.clone(),
            description: state.manifest.description.clone(),
            error: None,
        }
    }

    pub(crate) fn from_failure(id: String, error: String) -> Self {
        let name = id.clone();
        Self {
            id,
            name,
            version: String::new(),
            status: "error".to_string(),
            enabled: false,
            permissions: Vec::new(),
            hooks: Vec::new(),
            author: String::new(),
            description: String::new(),
            error: Some(error),
        }
    }
}
