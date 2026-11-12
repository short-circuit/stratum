//! Round-trip tests for `.excalidraw` boards and `.excalidrawlib` libraries.
//!
//! These cover the two save guarantees: an untouched file is returned
//! byte-for-byte, and an edited file preserves unknown element fields, unknown
//! top-level keys, soft-deleted entries and embedded image files. Library
//! tests cover both shipped shapes and the `insert_into` id/group remapping.

use std::path::PathBuf;

use pkm_core::PkmError;
use serde_json::Value;
use stratum_canvas::{BoardDocument, Library, LibraryShape};

/// Read one fixture from `tests/fixtures`.
fn fixture(name: &str) -> Vec<u8> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn untouched_boards_save_byte_identical() {
    for name in [
        "shapes.excalidraw",
        "frame.excalidraw",
        "image.excalidraw",
        "legacy.excalidraw",
        "empty.excalidraw",
    ] {
        let bytes = fixture(name);
        let mut doc = BoardDocument::load(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(!doc.is_dirty(), "{name} should load clean");
        let saved = doc.save().unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(saved, bytes, "{name} must save byte-identically");
    }
}

#[test]
fn untouched_libraries_save_byte_identical() {
    for name in ["library.excalidrawlib", "library_array.excalidrawlib"] {
        let bytes = fixture(name);
        let mut library = Library::load(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        let saved = library.save().unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(saved, bytes, "{name} must save byte-identically");
    }
}

#[test]
fn unknown_fields_survive_a_rewrite() {
    let bytes = fixture("shapes.excalidraw");
    let mut doc = BoardDocument::load(&bytes).unwrap();
    doc.element_mut("shape-rect").unwrap().set_x(999.0);
    let saved = doc.save().unwrap();

    let value: Value = serde_json::from_slice(&saved).unwrap();
    // Unknown top-level keys and unknown element keys are both preserved.
    assert_eq!(value["pluginData"]["custom"], Value::Bool(true));
    assert_eq!(value["pluginData"]["version"], Value::from(7));
    let rect = value["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|element| element["id"] == "shape-rect")
        .unwrap();
    assert_eq!(rect["x"], Value::from(999.0));
    assert_eq!(rect["customData"]["note"], "keep-me");
    assert_eq!(rect["customData"]["nested"], serde_json::json!([1, 2, 3]));
    // The producer header is carried over, not replaced.
    assert_eq!(value["type"], "excalidraw");
    assert_eq!(value["version"], Value::from(2));
    assert_eq!(value["source"], "https://excalidraw.com");
}

#[test]
fn soft_deleted_elements_survive_a_round_trip() {
    let bytes = fixture("shapes.excalidraw");
    let mut doc = BoardDocument::load(&bytes).unwrap();
    let before = doc.visible_elements().count();
    doc.element_mut("shape-ellipse").unwrap().set_deleted(true);
    let saved = doc.save().unwrap();

    let reloaded = BoardDocument::load(&saved).unwrap();
    let deleted = reloaded.element("shape-ellipse").expect("entry retained");
    assert!(deleted.is_deleted());
    assert_eq!(reloaded.visible_elements().count(), before - 1);
    // Soft delete keeps the entry in z-order instead of dropping it.
    assert_eq!(reloaded.elements().len(), doc.elements().len());
}

#[test]
fn embedded_image_files_survive_a_rewrite() {
    let bytes = fixture("image.excalidraw");
    let mut doc = BoardDocument::load(&bytes).unwrap();
    doc.element_mut("image-1").unwrap().set_x(60.0);
    let saved = doc.save().unwrap();
    let reloaded = BoardDocument::load(&saved).unwrap();
    let entry = reloaded.files()["image-file-1"].clone();
    assert_eq!(entry["mimeType"], "image/png");
    assert_eq!(
        entry["dataURL"],
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAYAAACp8Z5+AAAAEklEQVR42mM4oaHxHxkzkC4AAGw4IXGxLAXgAAAAAElFTkSuQmCC"
    );
}

#[test]
fn library_object_shape_and_unknown_keys_survive() {
    let bytes = fixture("library.excalidrawlib");
    let mut library = Library::load(&bytes).unwrap();
    assert_eq!(library.shape(), LibraryShape::ObjectV2);
    library.remove("lib-item-1");
    let saved = library.save().unwrap();

    let value: Value = serde_json::from_slice(&saved).unwrap();
    assert!(value.is_object());
    assert_eq!(value["type"], "excalidrawlib");
    assert_eq!(value["version"], Value::from(2));
    assert_eq!(value["source"], "https://excalidraw.com");
    assert_eq!(value["customMeta"]["owner"], "stratum");
    assert_eq!(value["libraryItems"], serde_json::json!([]));
}

#[test]
fn library_array_shape_survives_a_rewrite() {
    let bytes = fixture("library_array.excalidrawlib");
    let mut library = Library::load(&bytes).unwrap();
    assert_eq!(library.shape(), LibraryShape::BareArray);
    library.remove("array-item-1");
    let saved = library.save().unwrap();
    let value: Value = serde_json::from_slice(&saved).unwrap();
    assert!(value.is_array());
    assert_eq!(value, serde_json::json!([]));
}

#[test]
fn library_insert_into_remaps_ids_and_bindings() {
    let library = Library::load(&fixture("library.excalidrawlib")).unwrap();
    let item = &library.items()[0];
    assert_eq!(item.elements().len(), 3);

    let mut doc = BoardDocument::empty();
    let ids = item.insert_into(&mut doc, 100.0, 200.0).unwrap();
    assert_eq!(ids.len(), 3);
    assert!(!ids.iter().any(|id| id.starts_with("lib-")));

    let rect = doc.element(&ids[0]).unwrap();
    let text = doc.element(&ids[1]).unwrap();
    let arrow = doc.element(&ids[2]).unwrap();

    // Positions are offset so the item's top-left lands at (100, 200).
    assert_eq!((rect.x(), rect.y()), (100.0, 200.0));
    assert_eq!((text.x(), text.y()), (110.0, 210.0));

    // Text container, bound element and arrow binding all point at the clone.
    assert_eq!(text.container_id().as_deref(), Some(ids[0].as_str()));
    assert_eq!(rect.bound_elements()[0].id, ids[1]);
    assert_eq!(
        arrow.start_binding().unwrap().element_id,
        ids[0],
        "arrow binding must be remapped to the cloned rectangle"
    );
    assert!(arrow.end_binding().is_none());

    // Group ids are remapped to a fresh group, shared by the cloned set.
    let groups = rect.group_ids();
    assert_eq!(groups.len(), 1);
    assert_ne!(groups[0], "lib-group");
    assert_eq!(text.group_ids(), groups);
    assert_eq!(arrow.group_ids(), groups);
}

#[test]
fn library_insert_into_avoids_document_id_clashes() {
    let library = Library::load(&fixture("library_array.excalidrawlib")).unwrap();
    let item = &library.items()[0];
    let mut doc = BoardDocument::empty();
    // Force a clash by seeding a document id equal to the item's element id;
    // insert_into remaps to fresh uuids, so this must still succeed.
    let mut seeded = stratum_canvas::Element::new(
        stratum_canvas::ElementKind::Rectangle,
        "array-ellipse",
        0.0,
        0.0,
        10.0,
        10.0,
    );
    seeded.set_x(0.0);
    doc.add_element(seeded).unwrap();
    let ids = item.insert_into(&mut doc, 0.0, 0.0).unwrap();
    assert_eq!(ids.len(), 1);
    assert_ne!(ids[0], "array-ellipse");
    assert_eq!(doc.elements().len(), 2);
}

#[test]
fn malformed_payloads_are_canvas_errors() {
    let err = BoardDocument::load(b"[]").unwrap_err();
    assert!(matches!(err, PkmError::Canvas(_)));
    let err = BoardDocument::load(b"{\"type\":\"excalidraw\"}").unwrap_err();
    assert!(matches!(err, PkmError::Canvas(_)));
    let err = Library::load(b"{}").unwrap_err();
    assert!(matches!(err, PkmError::Canvas(_)));
}
