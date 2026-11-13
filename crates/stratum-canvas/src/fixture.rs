//! Deterministic fixture boards for tests, captures and future provider
//! wiring.
//!
//! The builders are pure: they never touch the filesystem or the clock, and
//! every id/geometry/seed is fixed so snapshots and assertions stay stable.
//! Use [`sample_board`] for a small framed architecture sketch and
//! [`sample_library`] for a matching library item.

use crate::document::BoardDocument;
use crate::element::{ArrowBinding, Binding, Element};
use crate::element_kind::{Arrowhead, ElementKind, FillStyle, StrokeStyle};
use crate::geometry::Point;
use crate::library::{Library, LibraryItem};

/// Build the deterministic sample board (frame, rectangle, bound text,
/// ellipse, arrow, freedraw).
pub fn sample_board() -> BoardDocument {
    let mut doc = BoardDocument::empty();

    let mut frame = Element::new(
        ElementKind::Frame,
        "fixture-frame",
        100.0,
        100.0,
        420.0,
        300.0,
    );
    frame.set_frame_name(Some("Architecture"));
    let _ = doc.add_element(frame);

    let mut rectangle = Element::new(
        ElementKind::Rectangle,
        "fixture-rect",
        140.0,
        160.0,
        140.0,
        70.0,
    );
    rectangle.set_frame_id(Some("fixture-frame"));
    rectangle.set_bound_elements(&[Binding {
        element_type: "text".to_string(),
        id: "fixture-text".to_string(),
    }]);
    let _ = doc.add_element(rectangle);

    let mut text = Element::new(ElementKind::Text, "fixture-text", 160.0, 180.0, 100.0, 30.0);
    text.set_text("API");
    text.set_extra("originalText", serde_json::json!("API"));
    text.set_extra("containerId", serde_json::json!("fixture-rect"));
    text.set_frame_id(Some("fixture-frame"));
    let _ = doc.add_element(text);

    let mut ellipse = Element::new(
        ElementKind::Ellipse,
        "fixture-ellipse",
        330.0,
        160.0,
        120.0,
        80.0,
    );
    ellipse.set_frame_id(Some("fixture-frame"));
    ellipse.set_style(&crate::element::Style {
        background_color: "#a5d8ff".to_string(),
        fill_style: FillStyle::Solid,
        ..crate::element::Style::default()
    });
    let _ = doc.add_element(ellipse);

    let mut arrow = Element::new(ElementKind::Arrow, "fixture-arrow", 280.0, 195.0, 50.0, 0.0);
    arrow.set_points(&[Point::new(0.0, 0.0), Point::new(50.0, 0.0)]);
    arrow.set_end_arrowhead(Some(Arrowhead::Arrow));
    arrow.set_start_binding(Some(&ArrowBinding {
        element_id: "fixture-rect".to_string(),
        focus: 0.0,
        gap: 4.0,
    }));
    arrow.set_end_binding(Some(&ArrowBinding {
        element_id: "fixture-ellipse".to_string(),
        focus: 0.0,
        gap: 4.0,
    }));
    arrow.set_frame_id(Some("fixture-frame"));
    let _ = doc.add_element(arrow);

    let mut freedraw = Element::new(
        ElementKind::Freedraw,
        "fixture-draw",
        140.0,
        330.0,
        60.0,
        20.0,
    );
    freedraw.set_points(&[
        Point::new(0.0, 0.0),
        Point::new(20.0, 10.0),
        Point::new(40.0, 2.0),
        Point::new(60.0, 20.0),
    ]);
    freedraw.set_style(&crate::element::Style {
        stroke_style: StrokeStyle::Dashed,
        ..crate::element::Style::default()
    });
    let _ = doc.add_element(freedraw);

    doc
}

/// Build a library containing one framed item cloned from [`sample_board`].
pub fn sample_library() -> Library {
    let board = sample_board();
    let elements = board
        .elements()
        .iter()
        .filter(|element| element.kind() != ElementKind::Frame)
        .map(Element::to_json)
        .collect::<Vec<_>>();
    let item = LibraryItem::from_json(serde_json::json!({
        "id": "fixture-item",
        "status": "published",
        "created": 0,
        "name": "Architecture sketch",
        "elements": elements,
    }))
    .expect("fixture library item is valid");
    let mut library = Library::empty();
    library.push(item);
    library
}
