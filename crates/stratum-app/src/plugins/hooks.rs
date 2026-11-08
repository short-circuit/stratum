//! Plugin hook dispatch helpers (spec §8).
//!
//! Called from page/backlink/search flows after a note is saved or opened, a
//! link is created, or a search runs. Payload shapes are frozen by the plugin
//! ABI (see `docs/advanced/plugins.md`).

use super::PluginManager;

/// Dispatch `onSave` to all enabled plugins that declare it. Payload is
/// `{"path": ..., "content": ...}`.
pub fn dispatch_on_save(manager: &PluginManager, path: &str, content: &str) {
    if !manager.has_hook("onSave") {
        return;
    }
    let payload = serde_json::json!({ "path": path, "content": content });
    manager.dispatch_all("onSave", &payload.to_string());
}

/// Dispatch `onOpen` to all enabled plugins that declare it. Payload is
/// `{"path": "…"}` (spec §8). Returns the per-plugin dispatch results.
pub fn dispatch_on_open(manager: &PluginManager, path: &str) -> Vec<(String, Option<String>)> {
    if !manager.has_hook("onOpen") {
        return Vec::new();
    }
    let payload = serde_json::json!({ "path": path });
    manager.dispatch_all("onOpen", &payload.to_string())
}

/// Dispatch `onLink` to all enabled plugins that declare it. `links` is the
/// list of wiki-link targets extracted from the saved note content. Payload is
/// `{"path": "…", "links": ["…"]}` (spec §8).
pub fn dispatch_on_link(
    manager: &PluginManager,
    path: &str,
    links: &[String],
) -> Vec<(String, Option<String>)> {
    if !manager.has_hook("onLink") {
        return Vec::new();
    }
    let payload = serde_json::json!({ "path": path, "links": links });
    manager.dispatch_all("onLink", &payload.to_string())
}

/// Dispatch `onSearch` to all enabled plugins that declare it. Payload is
/// `{"query": "…", "limit": N}` (spec §8).
pub fn dispatch_on_search(
    manager: &PluginManager,
    query: &str,
    limit: usize,
) -> Vec<(String, Option<String>)> {
    if !manager.has_hook("onSearch") {
        return Vec::new();
    }
    let payload = serde_json::json!({ "query": query, "limit": limit });
    manager.dispatch_all("onSearch", &payload.to_string())
}

/// Addressable handle for hook dispatch from other modules.
pub fn dispatch_hook(manager: &PluginManager, hook: &str, payload: &str) {
    manager.dispatch_all(hook, payload);
}
