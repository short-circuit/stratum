//! Rendering integration tests: scene generation, SVG/PNG/WebP exports,
//! thumbnails and export actions against the fixture corpus.
//!
//! Fixtures live in `tests/fixtures/` and are the same boards used by the
//! round-trip tests, so exports are checked against real `.excalidraw` input.

use std::path::PathBuf;

use base64::Engine;
use stratum_canvas::render::raster::{export_png, ink_coverage, thumbnail};
use stratum_canvas::render::rough::element_paths;
use stratum_canvas::render::svg::export_svg;
use stratum_canvas::render::svg_path::PathOp;
use stratum_canvas::render::{
    CanvasThumbnail, Scene, SceneCache, SceneOptions, THUMBNAIL_MAX_EDGE,
};
use stratum_canvas::{
    BoardDocument, CanvasState, Element, ElementKind, FillStyle, PreviewTheme, Roundness, Style,
};

/// Embedded payload of `tests/fixtures/image.excalidraw`.
const IMAGE_FIXTURE_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAYAAACp8Z5+AAAAEklEQVR42mM4oaHxHxkzkC4AAGw4IXGxLAXgAAAAAElFTkSuQmCC";

/// Load a fixture board by file name.
fn fixture(name: &str) -> BoardDocument {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    BoardDocument::load(&bytes).unwrap_or_else(|e| panic!("load {}: {e}", path.display()))
}

#[test]
fn exports_are_deterministic() {
    let doc = fixture("shapes.excalidraw");
    let options = SceneOptions::default();
    assert_eq!(
        export_svg(&doc, &options).unwrap(),
        export_svg(&doc, &options).unwrap()
    );
    assert_eq!(
        export_png(&doc, &options, Some(200)).unwrap(),
        export_png(&doc, &options, Some(200)).unwrap()
    );
}

#[test]
fn frame_fixture_clips_children_and_renders_the_title() {
    let doc = fixture("frame.excalidraw");
    let svg = export_svg(&doc, &SceneOptions::default()).unwrap();
    assert!(svg.contains("<clipPath id=\"clip-0\">"));
    assert!(svg.contains("clip-path=\"url(#clip-0)\""));
    assert!(svg.contains(">Architecture</text>"));
    // Excalidraw frame chrome: a rounded #bbb outline and a plain #999999
    // title with no filled tab.
    assert!(svg.contains("stroke=\"#bbb\""));
    assert!(svg.contains("stroke-width=\"2\""));
    assert!(svg.contains("fill=\"#999999\""));
    assert!(svg.contains("font-size=\"14\""));
    assert!(svg.contains("text-anchor=\"start\""));
    assert!(svg.contains("rx=\"8\" ry=\"8\""));
    assert!(!svg.contains("#e9ecef"));
    assert!(!svg.contains("#ced4da"));
    let scene = Scene::build(&doc, &SceneOptions::default()).unwrap();
    assert_eq!(scene.frames.len(), 1);
    assert!(scene.paths.iter().any(|path| path.clip.is_some()));
}

#[test]
fn frame_fixture_paths_stay_continuous() {
    let doc = fixture("frame.excalidraw");
    let scene = Scene::build(&doc, &SceneOptions::default()).unwrap();
    assert!(!scene.paths.is_empty());
    for path in &scene.paths {
        assert_continuous(&path.ops);
    }
    let png = export_png(&doc, &SceneOptions::default(), Some(400)).unwrap();
    assert!(ink_coverage(&png, "#ffffff").unwrap() > 0.001);
}

#[test]
fn image_fixture_renders_its_embedded_payload() {
    let doc = fixture("image.excalidraw");
    let svg = export_svg(&doc, &SceneOptions::default()).unwrap();
    assert!(svg.contains("<image"));
    assert!(svg.contains("href=\"data:image/png;base64,"));
    assert!(svg.contains(IMAGE_FIXTURE_BASE64));
    let png = export_png(&doc, &SceneOptions::default(), Some(200)).unwrap();
    assert!(ink_coverage(&png, "#ffffff").unwrap() > 0.001);
}

#[test]
fn image_fixture_payload_decodes() {
    let doc = fixture("image.excalidraw");
    let entry =
        stratum_canvas::files::file_entry(doc.files(), "image-file-1").expect("image file entry");
    let data_url = entry["dataURL"].as_str().expect("dataURL");
    let payload = data_url
        .strip_prefix("data:image/png;base64,")
        .expect("png data URL");
    assert_eq!(payload, IMAGE_FIXTURE_BASE64);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .expect("fixture payload is valid base64");
    let image = image::load_from_memory(&bytes).expect("fixture payload decodes");
    assert_eq!((image.width(), image.height()), (4, 4));
}

#[test]
fn rounded_rect_stroke_keeps_its_multi_strokes_apart() {
    let rect = rounded_element(ElementKind::Rectangle, 140.0, 70.0);
    let perimeter = 2.0 * (rect.width() + rect.height());
    let paths = element_paths(&rect).unwrap();
    let stroke = paths
        .iter()
        .find(|path| path.stroke.as_deref() == Some("#1e1e1e"))
        .expect("rounded rect stroke");
    assert_eq!(stroke.stroke_width, 2.0);
    let sub_paths = sub_paths(&stroke.ops);
    // roughr emits one sub-path per double-stroked segment; a `Move` written
    // as `L` would merge them all into a single sub-path with chords.
    assert!(
        sub_paths.len() >= 2,
        "expected multiple sub-paths, got {}",
        sub_paths.len()
    );
    for sub_path in &sub_paths {
        assert!(sub_path_length(sub_path) <= perimeter * 1.2);
    }
    assert_continuous(&stroke.ops);
    let fill = paths
        .iter()
        .find(|path| path.fill.as_deref() == Some("#a5d8ff"))
        .expect("rounded rect fill");
    assert!(fill.stroke.is_none());
}

#[test]
fn rounded_diamond_stroke_keeps_its_multi_strokes_apart() {
    let diamond = rounded_element(ElementKind::Diamond, 140.0, 90.0);
    let perimeter = 2.0 * (diamond.width() + diamond.height());
    let paths = element_paths(&diamond).unwrap();
    let stroke = paths
        .iter()
        .find(|path| path.stroke.as_deref() == Some("#1e1e1e"))
        .expect("rounded diamond stroke");
    let sub_paths = sub_paths(&stroke.ops);
    assert!(sub_paths.len() >= 2);
    for sub_path in &sub_paths {
        assert!(sub_path_length(sub_path) <= perimeter * 1.2);
    }
    assert_continuous(&stroke.ops);
}

/// Rounded, filled shape used by the multi-stroke serialisation regression.
fn rounded_element(kind: ElementKind, width: f64, height: f64) -> Element {
    let mut element = Element::new(kind, "shape", 0.0, 0.0, width, height);
    element.set_style(&Style {
        background_color: "#a5d8ff".to_string(),
        stroke_color: "#1e1e1e".to_string(),
        fill_style: FillStyle::Solid,
        roundness: Some(Roundness {
            kind: 1,
            value: None,
        }),
        ..Style::default()
    });
    element
}

/// Split ops into sub-paths at every `MoveTo`.
fn sub_paths(ops: &[PathOp]) -> Vec<Vec<PathOp>> {
    let mut sub_paths: Vec<Vec<PathOp>> = Vec::new();
    for op in ops {
        if matches!(op, PathOp::MoveTo(_)) {
            sub_paths.push(Vec::new());
        }
        let sub_path = sub_paths.last_mut().expect("path starts with MoveTo");
        sub_path.push(*op);
    }
    sub_paths
}

/// Total polyline length of one sub-path.
fn sub_path_length(sub_path: &[PathOp]) -> f64 {
    let mut length = 0.0;
    for pair in sub_path.windows(2) {
        let start = op_end(&pair[0]);
        let end = op_end(&pair[1]);
        length += ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();
    }
    length
}

/// Assert that every path op belongs to a sub-path that starts with `MoveTo`,
/// continues from the previous end point and never draws a chord longer than
/// the path's own bounding box perimeter.
fn assert_continuous(ops: &[PathOp]) {
    let (width, height) = ops_bounds(ops);
    let max_segment = 1.2 * (width + height);
    let max_sub_path = 2.4 * (width + height);
    let sub_paths = sub_paths(ops);
    assert!(!sub_paths.is_empty());
    for sub_path in &sub_paths {
        let mut cursor = match sub_path.first() {
            Some(PathOp::MoveTo(point)) => *point,
            other => panic!("sub-path does not start with MoveTo: {other:?}"),
        };
        let mut length = 0.0;
        for op in sub_path.iter().skip(1) {
            let end = op_end(op);
            let segment = ((end.x - cursor.x).powi(2) + (end.y - cursor.y).powi(2)).sqrt();
            assert!(
                segment <= max_segment,
                "segment {segment} exceeds {max_segment} (chord between strokes)"
            );
            length += segment;
            cursor = end;
        }
        assert!(
            length <= max_sub_path,
            "sub-path {length} exceeds {max_sub_path} (strokes merged)"
        );
    }
}

/// Bounding box size of a path's points (including curve control points).
fn ops_bounds(ops: &[PathOp]) -> (f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut point = |p: stratum_canvas::geometry::Point| {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    };
    for op in ops {
        match op {
            PathOp::MoveTo(p) | PathOp::LineTo(p) => point(*p),
            PathOp::CurveTo { c1, c2, to } => {
                point(*c1);
                point(*c2);
                point(*to);
            }
            PathOp::Close => {}
        }
    }
    (max_x - min_x, max_y - min_y)
}

/// End point of one path op.
fn op_end(op: &PathOp) -> stratum_canvas::geometry::Point {
    match op {
        PathOp::MoveTo(p) | PathOp::LineTo(p) => *p,
        PathOp::CurveTo { to, .. } => *to,
        PathOp::Close => stratum_canvas::geometry::Point::new(0.0, 0.0),
    }
}

#[test]
fn empty_and_legacy_fixtures_still_export() {
    let empty = fixture("empty.excalidraw");
    let svg = export_svg(&empty, &SceneOptions::default()).unwrap();
    assert!(svg.contains("width=\"100.000\""));
    let legacy = fixture("legacy.excalidraw");
    let scene = Scene::build(&legacy, &SceneOptions::default()).unwrap();
    assert!(!scene.paths.is_empty());
}

#[test]
fn text_is_xml_escaped() {
    let mut doc = BoardDocument::empty();
    let mut text = Element::new(ElementKind::Text, "t", 10.0, 10.0, 200.0, 25.0);
    text.set_text("a < b & c > d");
    text.set_extra("originalText", serde_json::json!("a < b & c > d"));
    doc.add_element(text).unwrap();
    let svg = export_svg(&doc, &SceneOptions::default()).unwrap();
    assert!(svg.contains("a &lt; b &amp; c &gt; d"));
    assert!(!svg.contains("a < b & c > d"));
}

#[test]
fn raster_edge_caps_are_respected() {
    let doc = fixture("shapes.excalidraw");
    let png = export_png(&doc, &SceneOptions::default(), Some(120)).unwrap();
    let image = stratum_canvas::render::render_scene(
        &Scene::build(&doc, &SceneOptions::default()).unwrap(),
        Some(120),
    )
    .unwrap();
    assert_eq!(&png[..4], b"\x89PNG");
    assert!(image.width.max(image.height) <= 120);
    assert!(stratum_canvas::render::export_png(&doc, &SceneOptions::default(), Some(0)).is_err());
}

#[test]
fn thumbnail_is_webp_within_the_preview_cap() {
    let doc = fixture("shapes.excalidraw");
    let thumb = thumbnail(&doc, PreviewTheme::Light).unwrap();
    assert!(thumb.data_url.starts_with("data:image/webp;base64,"));
    assert!(thumb.width <= THUMBNAIL_MAX_EDGE);
    assert!(thumb.height <= THUMBNAIL_MAX_EDGE);
}

#[test]
fn canvas_state_actions_export_and_store_previews() {
    let mut state = CanvasState::new(fixture("shapes.excalidraw"));
    let options = SceneOptions::default();
    let svg = state.export_svg(&options).unwrap();
    assert!(svg.contains("<svg"));
    let png = state.export_png(&options, Some(300)).unwrap();
    assert_eq!(&png[..4], b"\x89PNG");
    let webp = state.export_webp(&options, Some(300)).unwrap();
    assert_eq!(&webp[..4], b"RIFF");
    let thumb: CanvasThumbnail = state.refresh_thumbnail(PreviewTheme::Dark).unwrap();
    assert_eq!(state.document().preview(), Some(thumb.data_url.as_str()));
    assert_eq!(state.document().preview_theme(), Some(PreviewTheme::Dark));
    let set = state
        .visible(stratum_canvas::geometry::Rect::new(
            0.0, 0.0, 2000.0, 2000.0,
        ))
        .unwrap();
    assert!(!set.paths.is_empty());
}

#[test]
fn scene_cache_reuses_the_revision() {
    let doc = fixture("shapes.excalidraw");
    let options = SceneOptions::default();
    let mut cache = SceneCache::new();
    let first = cache.scene(&doc, &options).unwrap().revision;
    let second = cache.scene(&doc, &options).unwrap().revision;
    assert_eq!(first, second);
    let scene = cache.scene(&doc, &options).unwrap();
    let counts = scene.counts();
    assert!(counts.paths > 0);
    assert!(counts.texts > 0);
}
