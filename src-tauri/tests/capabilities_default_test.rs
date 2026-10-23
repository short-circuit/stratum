//! Guards the close-to-tray window capability: without `core:window:allow-hide`,
//! the frontend close handler's `getCurrentWindow().hide()` invoke is rejected by
//! the Tauri v2 ACL and the window close button silently does nothing.
//! See t_e51d3067.

use std::path::PathBuf;

fn capability_permissions(manifest: &str) -> Vec<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("capabilities")
        .join(manifest);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    let json: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path:?}: {e}"));
    json["permissions"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn desktop_capabilities_allow_window_hide_and_destroy() {
    let perms = capability_permissions("default.json");
    assert!(
        perms.iter().any(|p| p == "core:window:allow-hide"),
        "default.json must grant core:window:allow-hide (close-to-tray hide would be ACL-rejected otherwise)"
    );
    assert!(
        perms.iter().any(|p| p == "core:window:allow-destroy"),
        "default.json must grant core:window:allow-destroy"
    );
}

#[test]
fn mobile_capabilities_allow_window_hide_and_destroy() {
    let perms = capability_permissions("mobile.json");
    assert!(
        perms.iter().any(|p| p == "core:window:allow-hide"),
        "mobile.json must grant core:window:allow-hide (parity with desktop)"
    );
    assert!(
        perms.iter().any(|p| p == "core:window:allow-destroy"),
        "mobile.json must grant core:window:allow-destroy"
    );
}
