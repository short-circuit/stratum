//! Whiteboard commands.
//!
//! Whiteboards are stored as `.excalidraw` files (Excalidraw JSON format) in the vault.

use crate::state::StratumService;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WhiteboardDto {
    pub name: String,
    pub path: String,
    pub content: String,
}

impl StratumService {
    /// List all whiteboards in `<vault>/whiteboards/`.
    pub async fn list_whiteboards(&self) -> Result<Vec<WhiteboardDto>, String> {
        let state = self.lock()?;
        let wb_dir = state.vault_path.join("whiteboards");
        if !wb_dir.exists() {
            return Ok(Vec::new());
        }

        let mut boards = Vec::new();
        let entries = std::fs::read_dir(&wb_dir).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("excalidraw") {
                let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("untitled")
                    .to_string();
                let rel_path = path
                    .strip_prefix(&state.vault_path)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                boards.push(WhiteboardDto {
                    name,
                    path: rel_path,
                    content,
                });
            }
        }

        Ok(boards)
    }

    /// Create or overwrite a whiteboard by name.
    pub async fn save_whiteboard(&self, name: String, content: String) -> Result<(), String> {
        let state = self.lock()?;
        let wb_dir = state.vault_path.join("whiteboards");
        std::fs::create_dir_all(&wb_dir).map_err(|e| e.to_string())?;
        let path = wb_dir.join(format!("{}.excalidraw", name));
        std::fs::write(&path, &content).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Rename a whiteboard.
    pub async fn rename_whiteboard(
        &self,
        old_name: String,
        new_name: String,
    ) -> Result<(), String> {
        let state = self.lock()?;
        let wb_dir = state.vault_path.join("whiteboards");
        let old_path = wb_dir.join(format!("{}.excalidraw", old_name));
        let new_path = wb_dir.join(format!("{}.excalidraw", new_name));
        if !old_path.exists() {
            return Err("Whiteboard not found".to_string());
        }
        if new_path.exists() {
            return Err("A whiteboard with that name already exists".to_string());
        }
        std::fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete a whiteboard by name (no-op when missing).
    pub async fn delete_whiteboard(&self, name: String) -> Result<(), String> {
        let state = self.lock()?;
        let path = state
            .vault_path
            .join("whiteboards")
            .join(format!("{}.excalidraw", name));
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Load a whiteboard's JSON, returning an empty canvas when missing.
    pub async fn load_whiteboard(&self, name: String) -> Result<String, String> {
        let state = self.lock()?;
        let path = state
            .vault_path
            .join("whiteboards")
            .join(format!("{}.excalidraw", name));
        if !path.exists() {
            return Ok(serde_json::json!({
                "elements": [],
                "appState": { "viewBackgroundColor": "#ffffff" }
            })
            .to_string());
        }
        std::fs::read_to_string(&path).map_err(|e| e.to_string())
    }

    /// Persist the user's personal Excalidraw library.
    pub async fn save_library(&self, content: String) -> Result<(), String> {
        let state = self.lock()?;
        let wb_dir = state.vault_path.join("whiteboards");
        std::fs::create_dir_all(&wb_dir).map_err(|e| e.to_string())?;
        let path = wb_dir.join("library.excalidrawlib");
        std::fs::write(&path, &content).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Load the user's personal library from `library.excalidrawlib`.
    pub async fn load_library(&self) -> Result<String, String> {
        let state = self.lock()?;
        let path = state
            .vault_path
            .join("whiteboards")
            .join("library.excalidrawlib");
        if !path.exists() {
            return Ok("[]".to_string());
        }
        std::fs::read_to_string(&path).map_err(|e| e.to_string())
    }

    /// Load additional `.excalidrawlib` files from the whiteboards folder (excluding
    /// `library.excalidrawlib`) and merge them into a single JSON array. Any library
    /// files placed in the folder are auto-loaded into the library panel.
    pub async fn load_extra_libraries(&self) -> Result<String, String> {
        let state = self.lock()?;
        let wb_dir = state.vault_path.join("whiteboards");
        if !wb_dir.exists() {
            return Ok("[]".to_string());
        }

        let mut merged: Vec<serde_json::Value> = Vec::new();
        let entries = std::fs::read_dir(&wb_dir).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            // Skip the personal library file — it's loaded separately
            if path.file_stem().and_then(|s| s.to_str()) == Some("library") {
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) == Some("excalidrawlib") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                        merged.extend(items);
                    }
                }
            }
        }

        serde_json::to_string(&merged).map_err(|e| e.to_string())
    }

    /// Write an exported whiteboard image under `<vault>/whiteboards/exports/`.
    ///
    /// `name` is the board's file stem; `extension` selects the format and must
    /// be one of `svg` or `png`. Path separators and empty names are rejected
    /// so an export can never escape the exports folder. Returns the vault-
    /// relative path of the written file.
    ///
    /// # Errors
    ///
    /// Returns a message when the name or extension is invalid or the file
    /// cannot be written.
    pub async fn write_whiteboard_export(
        &self,
        name: String,
        extension: String,
        bytes: Vec<u8>,
    ) -> Result<String, String> {
        let state = self.lock()?;
        let file_name = export_file_name(&name, &extension)?;
        let dir = state.vault_path.join("whiteboards").join("exports");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(&file_name);
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        let rel_path = path
            .strip_prefix(&state.vault_path)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        Ok(rel_path)
    }
}

/// Validate an export name/extension pair and build the file name.
///
/// The name must be non-empty, must not contain a path separator, and must not
/// be `.`/`..`. The extension is lowercased and limited to `svg`/`png`.
fn export_file_name(name: &str, extension: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']) {
        return Err(format!("invalid whiteboard export name: {name:?}"));
    }
    let extension = extension.trim().to_ascii_lowercase();
    if !matches!(extension.as_str(), "svg" | "png") {
        return Err(format!("unsupported whiteboard export format: {extension}"));
    }
    Ok(format!("{name}.{extension}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(fut)
    }

    fn service_for(vault: &Path) -> StratumService {
        std::fs::create_dir_all(vault.join(".pkm")).expect("create .pkm");
        StratumService::new(vault.to_path_buf())
    }

    #[test]
    fn export_name_rejects_traversal_and_unknown_formats() {
        assert_eq!(export_file_name("board", "svg").unwrap(), "board.svg");
        assert_eq!(export_file_name("board", "PNG").unwrap(), "board.png");
        assert!(export_file_name("", "svg").is_err());
        assert!(export_file_name("../board", "svg").is_err());
        assert!(export_file_name("a/b", "svg").is_err());
        assert!(export_file_name("board", "webp").is_err());
    }

    #[test]
    fn write_export_creates_the_exports_folder() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = service_for(dir.path());
        let rel = block_on(service.write_whiteboard_export(
            "board".to_string(),
            "svg".to_string(),
            b"<svg/>".to_vec(),
        ))
        .expect("write export");
        assert_eq!(rel, "whiteboards/exports/board.svg");
        let written = std::fs::read(dir.path().join(&rel)).expect("read export");
        assert_eq!(written, b"<svg/>");
    }

    #[test]
    fn write_export_rejects_bad_input() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = service_for(dir.path());
        assert!(block_on(service.write_whiteboard_export(
            "..".to_string(),
            "svg".to_string(),
            Vec::new(),
        ))
        .is_err());
        assert!(block_on(service.write_whiteboard_export(
            "board".to_string(),
            "gif".to_string(),
            Vec::new(),
        ))
        .is_err());
    }
}
