//! Pure whiteboard models: gallery rows, the editor session and the capture
//! start state.
//!
//! The gpui panel in [`super`] renders these models but never implements canvas
//! behaviour itself. Keeping the document editing, autosave scheduling,
//! exports, library round-trip and deep-link focus here means the whole surface
//! is testable without opening a window (see [`super::tests`]).
//!
//! Autosave follows the shipped React panel: edits are saved 800 ms after the
//! last mutation, the document is serialized byte-identically while untouched,
//! and a save refreshes the WebP preview stored in the `.excalidraw` metadata.
//! Exports go through [`crate::provider::PanelData::write_canvas_export`]; PNG
//! exports render at twice the board's world-unit size.

use std::time::Duration;

use stratum_canvas::{BoardDocument, CanvasState, PreviewTheme, Tool};

use crate::provider::PanelData;
use crate::router::RouteTarget;

mod scene;
mod session;

pub use scene::document_svg;
pub use session::{EditorSession, ViewportFrame};

/// Autosave delay used by the shipped React whiteboard panel.
pub const AUTOSAVE_DELAY: Duration = Duration::from_millis(800);

/// PNG export scale, matching the shipped React `exportToBlob` scale of 2.
pub const EXPORT_SCALE: f64 = 2.0;

/// Width of the canvas frame the editor rasterizes, in pixels.
pub const FRAME_WIDTH: f64 = 960.0;

/// Height of the canvas frame the editor rasterizes, in pixels.
pub const FRAME_HEIGHT: f64 = 600.0;

/// Deterministic start state for the capture harness.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CanvasStart {
    /// Board to open instead of showing the gallery.
    pub board: Option<String>,
    /// Initial tool.
    pub tool: Option<Tool>,
    /// Whether the library drawer starts open.
    pub library_open: bool,
    /// Whether the style panel starts open.
    pub style_open: bool,
    /// Export format to run once the board is open.
    pub export: Option<ExportFormat>,
    /// Element id or frame name to focus after opening.
    pub element: Option<String>,
}

impl CanvasStart {
    /// Build a start state from a `/whiteboards?...` deep link.
    ///
    /// Unknown parameters are ignored; the query keeps the shipped React
    /// names (`board`, `tool`, `library`, `style`, `export`, `element`).
    pub fn from_target(target: &RouteTarget) -> Self {
        Self {
            board: target.param("board").map(str::to_string),
            tool: target.param("tool").and_then(tool_from_env),
            library_open: target
                .param("library")
                .is_some_and(|value| matches!(value, "1" | "true")),
            style_open: target
                .param("style")
                .is_some_and(|value| matches!(value, "1" | "true")),
            export: target.param("export").and_then(ExportFormat::from_env),
            element: target.param("element").map(str::to_string),
        }
    }
}

/// Export format for the capture harness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    /// Deterministic SVG export.
    Svg,
    /// PNG export at twice the board's world-unit size.
    Png,
}

impl ExportFormat {
    /// File extension written by [`crate::provider::PanelData::write_canvas_export`].
    pub fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
        }
    }

    /// Parse `STRATUM_WHITEBOARD_EXPORT`.
    pub fn from_env(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "svg" => Some(Self::Svg),
            "png" => Some(Self::Png),
            _ => None,
        }
    }
}

/// Map `STRATUM_WHITEBOARD_TOOL` to a canvas tool.
pub fn tool_from_env(value: &str) -> Option<Tool> {
    match value.to_ascii_lowercase().as_str() {
        "select" => Some(Tool::Select),
        "hand" => Some(Tool::Hand),
        "rectangle" => Some(Tool::Rectangle),
        "diamond" => Some(Tool::Diamond),
        "ellipse" => Some(Tool::Ellipse),
        "arrow" => Some(Tool::Arrow),
        "line" => Some(Tool::Line),
        "freedraw" | "draw" => Some(Tool::Freedraw),
        "text" => Some(Tool::Text),
        "eraser" => Some(Tool::Eraser),
        "frame" => Some(Tool::Frame),
        _ => None,
    }
}

/// Localized key for a tool chip.
pub fn tool_label_key(tool: Tool) -> &'static str {
    match tool {
        Tool::Select => "whiteboard.tool_select",
        Tool::Hand => "whiteboard.tool_hand",
        Tool::Rectangle => "whiteboard.tool_rectangle",
        Tool::Diamond => "whiteboard.tool_diamond",
        Tool::Ellipse => "whiteboard.tool_ellipse",
        Tool::Arrow => "whiteboard.tool_arrow",
        Tool::Line => "whiteboard.tool_line",
        Tool::Freedraw => "whiteboard.tool_draw",
        Tool::Text => "whiteboard.tool_text",
        Tool::Eraser => "whiteboard.tool_eraser",
        Tool::Frame => "whiteboard.tool_frame",
    }
}

/// Save-status shown in the editor header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveState {
    /// The loaded board is untouched.
    Clean,
    /// Edits are waiting for the autosave deadline.
    Pending,
    /// The board was written to the vault.
    Saved,
    /// The last save failed.
    Failed(String),
}

/// One gallery row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GalleryRow {
    /// Board name (file stem).
    pub name: String,
    /// Vault-relative path.
    pub path: String,
    /// Number of visible elements.
    pub element_count: usize,
    /// Stored or freshly rendered preview data URL.
    pub preview: Option<String>,
    /// Multi-select state.
    pub selected: bool,
}

/// Gallery model behind the whiteboards list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GalleryModel {
    rows: Vec<GalleryRow>,
}

impl GalleryModel {
    /// Load every board from the provider, counting elements and reusing the
    /// stored preview when present.
    pub fn load(provider: &dyn PanelData) -> Result<Self, String> {
        let boards = provider.whiteboards()?;
        let rows = boards
            .iter()
            .map(|board| {
                let doc = BoardDocument::load(board.content.as_bytes())
                    .map_err(|error| error.to_string())?;
                Ok(GalleryRow {
                    name: board.name.clone(),
                    path: board.path.clone(),
                    element_count: doc.visible_elements().count(),
                    preview: doc.preview().map(str::to_string),
                    selected: false,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self { rows })
    }

    /// The current rows.
    pub fn rows(&self) -> &[GalleryRow] {
        &self.rows
    }

    /// Toggle a row's multi-select state.
    pub fn toggle_selection(&mut self, name: &str) {
        if let Some(row) = self.rows.iter_mut().find(|row| row.name == name) {
            row.selected = !row.selected;
        }
    }

    /// Clear the multi-select state.
    pub fn clear_selection(&mut self) {
        for row in &mut self.rows {
            row.selected = false;
        }
    }

    /// Names currently selected.
    pub fn selected_names(&self) -> Vec<String> {
        self.rows
            .iter()
            .filter(|row| row.selected)
            .map(|row| row.name.clone())
            .collect()
    }

    /// Render previews for rows that do not carry a stored thumbnail.
    ///
    /// Previews are kept in memory only; the next save stores them in the
    /// document metadata.
    pub fn generate_previews(&mut self, provider: &dyn PanelData) {
        for row in &mut self.rows {
            if row.preview.is_some() {
                continue;
            }
            let Ok(content) = provider.load_whiteboard(&row.name) else {
                continue;
            };
            let Ok(doc) = BoardDocument::load(content.as_bytes()) else {
                continue;
            };
            let state = CanvasState::new(doc);
            if let Ok(thumb) = state.thumbnail(PreviewTheme::Light) {
                row.preview = Some(thumb.data_url);
            }
        }
    }

    /// Create a board and append its row.
    pub fn create(&mut self, provider: &dyn PanelData, name: &str) -> Result<(), String> {
        let row = provider.create_whiteboard(name)?;
        self.rows.push(GalleryRow {
            name: row.name,
            path: row.path,
            element_count: 0,
            preview: None,
            selected: false,
        });
        Ok(())
    }

    /// Rename a board and update the row.
    pub fn rename(
        &mut self,
        provider: &dyn PanelData,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), String> {
        provider.rename_whiteboard(old_name, new_name)?;
        let new_name = new_name.trim();
        if let Some(row) = self.rows.iter_mut().find(|row| row.name == old_name) {
            row.name = new_name.to_string();
            row.path = format!("whiteboards/{new_name}.excalidraw");
        }
        Ok(())
    }

    /// Delete the selected rows.
    pub fn delete_selected(&mut self, provider: &dyn PanelData) -> Result<usize, String> {
        let names = self.selected_names();
        if names.is_empty() {
            return Ok(0);
        }
        provider.delete_whiteboards(names.clone())?;
        self.rows
            .retain(|row| !names.iter().any(|name| name == &row.name));
        Ok(names.len())
    }

    /// Open a board as an editor session.
    pub fn open(
        provider: &dyn PanelData,
        name: &str,
        theme: PreviewTheme,
    ) -> Result<EditorSession, String> {
        let content = provider.load_whiteboard(name)?;
        EditorSession::open(name, &content, theme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_names_round_trip_from_env() {
        assert_eq!(tool_from_env("rectangle"), Some(Tool::Rectangle));
        assert_eq!(tool_from_env("FREEDRAW"), Some(Tool::Freedraw));
        assert_eq!(tool_from_env("nope"), None);
    }

    #[test]
    fn export_formats_parse() {
        assert_eq!(ExportFormat::from_env("SVG"), Some(ExportFormat::Svg));
        assert_eq!(ExportFormat::from_env("png"), Some(ExportFormat::Png));
        assert_eq!(ExportFormat::from_env("gif"), None);
        assert_eq!(ExportFormat::Png.extension(), "png");
    }

    #[test]
    fn start_state_reads_deep_link_parameters() {
        let target = crate::router::parse_target(
            "/whiteboards?board=Flow%20sketch&tool=frame&library=1&style=1&export=png&element=fixture-frame",
        )
        .expect("route parses");
        let start = CanvasStart::from_target(&target);
        assert_eq!(start.board.as_deref(), Some("Flow sketch"));
        assert_eq!(start.tool, Some(Tool::Frame));
        assert!(start.library_open);
        assert!(start.style_open);
        assert_eq!(start.export, Some(ExportFormat::Png));
        assert_eq!(start.element.as_deref(), Some("fixture-frame"));
        assert_eq!(
            CanvasStart::from_target(&crate::router::parse_target("/whiteboards").expect("route")),
            CanvasStart::default()
        );
    }
}
