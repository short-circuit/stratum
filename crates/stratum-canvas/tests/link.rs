//! Deep-link tests for [`stratum_canvas`]: parsing, percent encoding,
//! resolution against loaded boards, route generation and failures.

use std::path::PathBuf;

use pkm_core::PkmError;
use stratum_canvas::{decode_component, encode_component, BoardDocument, BoardLink, DeepLink};

/// Load one board fixture.
fn board(name: &str) -> BoardDocument {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(name);
    let bytes = std::fs::read(&path).unwrap();
    BoardDocument::load(&bytes).unwrap()
}

#[test]
fn parses_board_and_element_links() {
    let link = BoardLink::parse("  Architecture # Frame One  ").unwrap();
    assert_eq!(link.board, "Architecture");
    assert_eq!(link.element.as_deref(), Some("Frame One"));
    assert_eq!(link.to_string(), "Architecture#Frame One");

    let plain = BoardLink::parse("Architecture").unwrap();
    assert_eq!(plain.board, "Architecture");
    assert!(plain.element.is_none());
    assert_eq!(plain.to_string(), "Architecture");

    // An empty element segment is treated as absent.
    let trailing = BoardLink::parse("Architecture#").unwrap();
    assert!(trailing.element.is_none());
}

#[test]
fn query_encoding_round_trips_special_characters() {
    let link = BoardLink::parse("Board name#Frame & Co/100%").unwrap();
    let query = link.to_query();
    assert!(query.starts_with("board=Board%20name&element="));
    assert!(query.contains("%26"));
    assert!(query.contains("%2F"));
    assert!(query.contains("%25"));
    assert_eq!(BoardLink::from_query(&query).unwrap(), Some(link));

    // Leading '?' and '+' as space are accepted when decoding.
    let parsed = BoardLink::from_query("?board=Board+name&element=Frame+%26+Co").unwrap();
    assert_eq!(parsed.unwrap().board, "Board name");
    assert_eq!(encode_component("a b"), "a%20b");
    assert_eq!(decode_component("a%20b").unwrap(), "a b");
    assert_eq!(decode_component("a+b").unwrap(), "a b");
}

#[test]
fn resolves_element_ids_and_frames() {
    let shapes = board("shapes.excalidraw");
    let link = BoardLink::parse("Shapes#shape-rect").unwrap();
    let resolved = DeepLink::resolve(&link, &shapes).unwrap();
    assert_eq!(resolved.board, "Shapes");
    assert_eq!(resolved.element_id.as_deref(), Some("shape-rect"));
    assert!(resolved.frame_id.is_none());

    let frame_doc = board("frame.excalidraw");
    let by_name = BoardLink::parse("Frame#architecture").unwrap();
    let resolved = DeepLink::resolve(&by_name, &frame_doc).unwrap();
    assert!(resolved.element_id.is_none());
    assert_eq!(resolved.frame_id.as_deref(), Some("frame-1"));

    // A frame id also resolves to frame_id.
    let by_id = BoardLink::parse("Frame#frame-1").unwrap();
    let resolved = DeepLink::resolve(&by_id, &frame_doc).unwrap();
    assert_eq!(resolved.element_id.as_deref(), Some("frame-1"));
    assert_eq!(resolved.frame_id.as_deref(), Some("frame-1"));
}

#[test]
fn resolves_text_targets_case_insensitively() {
    let shapes = board("shapes.excalidraw");
    let link = BoardLink::parse("Shapes#hello").unwrap();
    let resolved = DeepLink::resolve(&link, &shapes).unwrap();
    assert_eq!(resolved.element_id.as_deref(), Some("shape-text"));
    assert!(resolved.frame_id.is_none());
}

#[test]
fn route_uses_the_shell_query_shape() {
    let frame_doc = board("frame.excalidraw");
    let link = BoardLink::parse("Frame#Architecture").unwrap();
    let resolved = DeepLink::resolve(&link, &frame_doc).unwrap();
    assert_eq!(resolved.route(), "/whiteboards?board=Frame&element=frame-1");

    let plain = BoardLink::parse("Board name").unwrap();
    let resolved = DeepLink::resolve(&plain, &frame_doc).unwrap();
    assert_eq!(resolved.route(), "/whiteboards?board=Board%20name");
}

#[test]
fn unknown_targets_and_malformed_links_fail() {
    let shapes = board("shapes.excalidraw");
    let missing = BoardLink::parse("Shapes#nope").unwrap();
    let err = DeepLink::resolve(&missing, &shapes).unwrap_err();
    assert!(matches!(err, PkmError::NotFound(_)));

    assert!(matches!(
        BoardLink::parse("   ").unwrap_err(),
        PkmError::Validation(_)
    ));
    assert!(matches!(
        BoardLink::parse("#element").unwrap_err(),
        PkmError::Validation(_)
    ));
    assert!(matches!(
        decode_component("%ZZ").unwrap_err(),
        PkmError::Validation(_)
    ));
    assert!(matches!(
        decode_component("%2").unwrap_err(),
        PkmError::Validation(_)
    ));
    assert_eq!(BoardLink::from_query("").unwrap(), None);
    assert_eq!(BoardLink::from_query("element=x").unwrap(), None);
    assert!(matches!(
        BoardLink::from_query("board=%20").unwrap_err(),
        PkmError::Validation(_)
    ));
}

#[test]
fn deleted_elements_fall_through_to_frames_and_text() {
    let mut shapes = board("shapes.excalidraw");
    shapes.element_mut("shape-text").unwrap().set_deleted(true);
    // The deleted text no longer resolves as a text target.
    let link = BoardLink::parse("Shapes#Hello").unwrap();
    let err = DeepLink::resolve(&link, &shapes).unwrap_err();
    assert!(matches!(err, PkmError::NotFound(_)));

    // Deleting a frame removes it from frame-name resolution too.
    let mut frame_doc = board("frame.excalidraw");
    frame_doc.element_mut("frame-1").unwrap().set_deleted(true);
    let by_name = BoardLink::parse("Frame#Architecture").unwrap();
    let err = DeepLink::resolve(&by_name, &frame_doc).unwrap_err();
    assert!(matches!(err, PkmError::NotFound(_)));
}
