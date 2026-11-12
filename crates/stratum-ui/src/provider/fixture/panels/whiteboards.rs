//! Whiteboard seed data and state helpers for the deterministic fixture.
//!
//! Rust merges trait impls per crate, but a trait may only be implemented for
//! a type in a single `impl` block, so the fixture's [`PanelData`] methods stay
//! in [`super::impls`] and delegate here. Keeping the board/library seed data
//! and the mutating helpers in this module leaves every provider file under the
//! repository length limit while the gallery/editor round-trip stays
//! observable without a vault.

use stratum_canvas::{fixture, BoardDocument, Element, ElementKind, Library};

use super::{panel_state, FixtureProvider};
use crate::provider::panels::{LibraryItemRow, WhiteboardRow};

/// Three deterministic whiteboards: the framed sample sketch, a small flow
/// sketch and an empty canvas.
pub(super) fn fixture_whiteboards() -> Vec<WhiteboardRow> {
    vec![
        whiteboard_row("Architecture sketch", fixture::sample_board()),
        whiteboard_row("Flow sketch", flow_board()),
        whiteboard_row("Blank canvas", BoardDocument::empty()),
    ]
}

/// Build one fixture whiteboard row from a document.
fn whiteboard_row(name: &str, mut doc: BoardDocument) -> WhiteboardRow {
    let content = doc.save_string().unwrap_or_default();
    WhiteboardRow::new(name, content)
}

/// A small deterministic flow sketch with two boxes and an arrow.
fn flow_board() -> BoardDocument {
    let mut doc = BoardDocument::empty();
    let mut left = Element::new(
        ElementKind::Rectangle,
        "fixture-flow-left",
        60.0,
        60.0,
        120.0,
        70.0,
    );
    left.set_style(&stratum_canvas::Style {
        background_color: "#d0ebff".to_string(),
        fill_style: stratum_canvas::FillStyle::Solid,
        ..stratum_canvas::Style::default()
    });
    let _ = doc.add_element(left);
    let mut right = Element::new(
        ElementKind::Rectangle,
        "fixture-flow-right",
        280.0,
        60.0,
        120.0,
        70.0,
    );
    right.set_style(&stratum_canvas::Style {
        background_color: "#b2f2bb".to_string(),
        fill_style: stratum_canvas::FillStyle::Solid,
        ..stratum_canvas::Style::default()
    });
    let _ = doc.add_element(right);
    let mut arrow = Element::new(
        ElementKind::Arrow,
        "fixture-flow-arrow",
        180.0,
        95.0,
        100.0,
        0.0,
    );
    arrow.set_points(&[
        stratum_canvas::Point::new(0.0, 0.0),
        stratum_canvas::Point::new(100.0, 0.0),
    ]);
    arrow.set_end_arrowhead(Some(stratum_canvas::Arrowhead::Arrow));
    let _ = doc.add_element(arrow);
    doc
}

/// The deterministic fixture library item.
pub(super) fn fixture_whiteboard_library() -> Vec<LibraryItemRow> {
    let library = fixture::sample_library();
    library
        .items()
        .iter()
        .map(LibraryItemRow::from_item)
        .collect()
}

/// List the fixture whiteboards.
pub(super) fn whiteboards(provider: &FixtureProvider) -> Result<Vec<WhiteboardRow>, String> {
    Ok(panel_state(provider)?.whiteboards.clone())
}

/// Load one fixture whiteboard's bytes.
pub(super) fn load_whiteboard(provider: &FixtureProvider, name: &str) -> Result<String, String> {
    panel_state(provider)?
        .whiteboards
        .iter()
        .find(|board| board.name == name)
        .map(|board| board.content.clone())
        .ok_or_else(|| format!("whiteboard not found: {name}"))
}

/// Persist board bytes, validating that they parse as a board document.
pub(super) fn save_whiteboard(
    provider: &FixtureProvider,
    name: &str,
    content: &str,
) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("whiteboard name is required".to_string());
    }
    BoardDocument::load(content.as_bytes()).map_err(|error| error.to_string())?;
    let mut state = panel_state(provider)?;
    let row = WhiteboardRow::new(name, content.to_string());
    match state
        .whiteboards
        .iter_mut()
        .find(|board| board.name == name)
    {
        Some(existing) => *existing = row,
        None => state.whiteboards.push(row),
    }
    Ok(())
}

/// Create an empty board and return its row.
pub(super) fn create_whiteboard(
    provider: &FixtureProvider,
    name: &str,
) -> Result<WhiteboardRow, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("whiteboard name is required".to_string());
    }
    let mut state = panel_state(provider)?;
    if state.whiteboards.iter().any(|board| board.name == name) {
        return Err(format!("whiteboard already exists: {name}"));
    }
    let content = BoardDocument::empty()
        .save_string()
        .map_err(|error| error.to_string())?;
    let row = WhiteboardRow::new(name, content);
    state.whiteboards.push(row.clone());
    Ok(row)
}

/// Rename a fixture board and refresh its vault-relative path.
pub(super) fn rename_whiteboard(
    provider: &FixtureProvider,
    old_name: &str,
    new_name: &str,
) -> Result<(), String> {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err("whiteboard name is required".to_string());
    }
    let mut state = panel_state(provider)?;
    if new_name != old_name && state.whiteboards.iter().any(|board| board.name == new_name) {
        return Err(format!("whiteboard already exists: {new_name}"));
    }
    let row = state
        .whiteboards
        .iter_mut()
        .find(|board| board.name == old_name)
        .ok_or_else(|| format!("whiteboard not found: {old_name}"))?;
    row.name = new_name.to_string();
    row.path = format!("whiteboards/{new_name}.excalidraw");
    Ok(())
}

/// Delete the named fixture boards.
pub(super) fn delete_whiteboards(
    provider: &FixtureProvider,
    names: Vec<String>,
) -> Result<(), String> {
    let mut state = panel_state(provider)?;
    state
        .whiteboards
        .retain(|board| !names.iter().any(|name| name == &board.name));
    Ok(())
}

/// List the fixture whiteboard library items.
pub(super) fn whiteboard_library(
    provider: &FixtureProvider,
) -> Result<Vec<LibraryItemRow>, String> {
    Ok(panel_state(provider)?.whiteboard_library.clone())
}

/// Replace the fixture library with the items in `json`.
pub(super) fn save_whiteboard_library(
    provider: &FixtureProvider,
    json: &str,
) -> Result<(), String> {
    let library = Library::load(json.as_bytes()).map_err(|error| error.to_string())?;
    let rows = library
        .items()
        .iter()
        .map(LibraryItemRow::from_item)
        .collect();
    panel_state(provider)?.whiteboard_library = rows;
    Ok(())
}

/// Record an exported canvas under `whiteboards/exports/`.
pub(super) fn write_canvas_export(
    provider: &FixtureProvider,
    name: &str,
    extension: &str,
    bytes: Vec<u8>,
) -> Result<String, String> {
    let path = format!("whiteboards/exports/{}", export_file_name(name, extension)?);
    panel_state(provider)?
        .whiteboard_exports
        .insert(path.clone(), bytes);
    Ok(path)
}

/// Validate an export name/extension pair and build the file name.
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

    #[test]
    fn export_name_rejects_bad_input() {
        assert_eq!(export_file_name("board", "PNG").unwrap(), "board.png");
        assert!(export_file_name("", "svg").is_err());
        assert!(export_file_name("a/b", "svg").is_err());
        assert!(export_file_name("board", "gif").is_err());
    }

    #[test]
    fn library_item_from_row_is_parseable() {
        let rows = fixture_whiteboard_library();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].to_item().is_some());
    }
}
