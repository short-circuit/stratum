//! Colour-contract tests for canvas exports and gallery thumbnails.
//!
//! Exports must match Excalidraw's own `exportToSvg` output: the document's raw
//! colours and canvas background, with no gallery-preview theme adaptation.
//! Thumbnails are the only surface that applies the shipped
//! `editorUtils.adaptElementsTheme` rule.
//!
//! Fixtures live in `tests/fixtures/`; the raw colour set is checked against
//! `.hermes/e7/shapes-excalidraw.svg` (generated with Excalidraw's exporter).

use std::path::PathBuf;

use base64::Engine;
use stratum_canvas::render::raster::{
    export_png, export_webp, ink_coverage, render_scene, thumbnail,
};
use stratum_canvas::render::svg::export_svg;
use stratum_canvas::render::{Scene, SceneOptions};
use stratum_canvas::{
    BoardDocument, CanvasState, Element, ElementKind, FillStyle, PreviewTheme, Style,
};

/// Load a fixture board by file name.
fn fixture(name: &str) -> BoardDocument {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    BoardDocument::load(&bytes).unwrap_or_else(|e| panic!("load {}: {e}", path.display()))
}

/// Decode a PNG payload into RGBA8 pixels.
fn decode_png(png: &[u8]) -> Vec<[u8; 4]> {
    let image = image::load_from_memory(png).expect("png decodes");
    image.to_rgba8().pixels().map(|pixel| pixel.0).collect()
}

/// Parse a `#RRGGBB` (or `#RGB`) colour into an opaque RGBA pixel.
fn rgba(hex: &str) -> [u8; 4] {
    let clean = hex.strip_prefix('#').expect("colour starts with #");
    // Expand CSS shorthand so `#bbb` and `#bbbbbb` parse identically.
    let expanded = if clean.len() == 3 {
        clean.chars().flat_map(|ch| [ch, ch]).collect::<String>()
    } else {
        clean.to_string()
    };
    let channel =
        |index: usize| u8::from_str_radix(&expanded[index..index + 2], 16).expect("hex channel");
    [channel(0), channel(2), channel(4), 255]
}

/// Whether `pixel` is within `tolerance` of the opaque `hex` colour.
fn near(pixel: &[u8; 4], hex: &str, tolerance: u8) -> bool {
    let expected = rgba(hex);
    pixel[3] == 255
        && pixel[..3]
            .iter()
            .zip(&expected[..3])
            .all(|(actual, want)| actual.abs_diff(*want) <= tolerance)
}

/// Decode a lossless WebP payload into RGBA8 pixels.
fn decode_webp(bytes: &[u8]) -> Vec<[u8; 4]> {
    let mut decoder =
        image_webp::WebPDecoder::new(std::io::Cursor::new(bytes)).expect("webp decoder");
    let mut buffer = vec![0; decoder.output_buffer_size().expect("webp buffer size")];
    decoder.read_image(&mut buffer).expect("webp decodes");
    let (pixels, _remainder) = buffer.as_chunks::<4>();
    pixels.to_vec()
}

#[test]
fn shapes_fixture_exports_keep_the_document_colours() {
    let doc = fixture("shapes.excalidraw");
    let options = SceneOptions::default();
    let svg = export_svg(&doc, &options).unwrap();
    assert!(svg.contains("<path"));
    assert!(svg.contains("<text"));
    assert!(svg.contains("data-excalidraw-font="));
    // Exports keep the document's raw colours (Excalidraw's own `exportToSvg`
    // output), not the gallery-preview theme adaptation.
    for colour in ["#a5d8ff", "#ffec99", "#1971c2", "#2f9e44"] {
        assert!(
            svg.contains(&format!("fill=\"{colour}\""))
                || svg.contains(&format!("stroke=\"{colour}\"")),
            "raw colour {colour} missing from the SVG export"
        );
    }
    assert!(
        svg.contains(&format!("fill=\"{}\"", doc.view_background_color())),
        "the SVG export must keep the document's canvas background"
    );
    assert!(
        !svg.contains("#5a2700"),
        "light preview inversion leaked into the export"
    );
    assert!(
        !svg.contains("#001366"),
        "light preview inversion leaked into the export"
    );
    let png = export_png(&doc, &options, Some(600)).unwrap();
    assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    assert!(ink_coverage(&png, &doc.view_background_color()).unwrap() > 0.01);
    let pixels = decode_png(&png);
    for colour in ["#a5d8ff", "#ffec99", "#1971c2", "#2f9e44"] {
        assert!(
            pixels.iter().any(|pixel| pixel == &rgba(colour)),
            "raw colour {colour} missing from the PNG export"
        );
    }
    let webp = export_webp(&doc, &options, Some(600)).unwrap();
    assert_eq!(&webp[..4], b"RIFF");
    assert_eq!(&webp[8..12], b"WEBP");
}

#[test]
fn dark_theme_uses_the_shipped_background_when_adapting() {
    let doc = fixture("shapes.excalidraw");
    let options = SceneOptions {
        theme: PreviewTheme::Dark,
        adapt_theme: true,
        ..SceneOptions::default()
    };
    let scene = stratum_canvas::render::Scene::build(&doc, &options).unwrap();
    assert_eq!(scene.background, "#232329");
    // `export_svg` never applies the gallery theme adaptation: exported files
    // keep the document's own canvas background.
    let svg = export_svg(&doc, &options).unwrap();
    assert!(svg.contains("fill=\"#ffffff\""));
}

#[test]
fn thumbnail_adapts_the_theme_but_exports_do_not() {
    let doc = fixture("shapes.excalidraw");
    let adapted = SceneOptions {
        theme: PreviewTheme::Dark,
        adapt_theme: true,
        ..SceneOptions::default()
    };
    let scene = stratum_canvas::render::Scene::build(&doc, &adapted).unwrap();
    assert_eq!(scene.background, "#232329");
    // Dark preview adaptation inverts dark strokes so they stay visible on the
    // dark canvas; the export path keeps the document's raw stroke colours.
    assert!(scene.paths.iter().any(|path| {
        path.stroke.as_deref() == Some("#e1e1e1") || path.fill.as_deref() == Some("#e1e1e1")
    }));
    let svg = export_svg(&doc, &adapted).unwrap();
    assert!(svg.contains("fill=\"#ffffff\""));
    assert!(svg.contains("stroke=\"#1e1e1e\""));
}

#[test]
fn exports_ignore_the_preview_theme_and_adaptation() {
    let doc = fixture("shapes.excalidraw");
    let raw = export_svg(&doc, &SceneOptions::default()).unwrap();
    for options in [
        SceneOptions {
            theme: PreviewTheme::Dark,
            ..SceneOptions::default()
        },
        SceneOptions {
            theme: PreviewTheme::Dark,
            adapt_theme: true,
            ..SceneOptions::default()
        },
        SceneOptions {
            theme: PreviewTheme::Light,
            adapt_theme: true,
            ..SceneOptions::default()
        },
    ] {
        assert_eq!(
            export_svg(&doc, &options).unwrap(),
            raw,
            "SVG export must ignore PreviewTheme and adapt_theme"
        );
        assert_eq!(
            export_png(&doc, &options, Some(200)).unwrap(),
            export_png(&doc, &SceneOptions::default(), Some(200)).unwrap(),
            "PNG export must ignore PreviewTheme and adapt_theme"
        );
        assert_eq!(
            export_webp(&doc, &options, Some(200)).unwrap(),
            export_webp(&doc, &SceneOptions::default(), Some(200)).unwrap(),
            "WebP export must ignore PreviewTheme and adapt_theme"
        );
    }
}

#[test]
fn canvas_state_exports_ignore_the_preview_theme() {
    let mut state = CanvasState::new(fixture("shapes.excalidraw"));
    let raw = state.export_svg(&SceneOptions::default()).unwrap();
    let adapted = SceneOptions {
        theme: PreviewTheme::Dark,
        adapt_theme: true,
        ..SceneOptions::default()
    };
    assert_eq!(state.export_svg(&adapted).unwrap(), raw);
    assert_eq!(
        state.export_png(&adapted, Some(200)).unwrap(),
        state
            .export_png(&SceneOptions::default(), Some(200))
            .unwrap()
    );
    assert_eq!(
        state.export_webp(&adapted, Some(200)).unwrap(),
        state
            .export_webp(&SceneOptions::default(), Some(200))
            .unwrap()
    );
}

#[test]
fn thumbnails_apply_the_gallery_theme_adaptation() {
    let doc = dark_stroke_board();
    let light = thumbnail(&doc, PreviewTheme::Light).unwrap();
    let dark = thumbnail(&doc, PreviewTheme::Dark).unwrap();
    let light_pixels = decode_webp(&thumbnail_bytes(&light.data_url));
    let dark_pixels = decode_webp(&thumbnail_bytes(&dark.data_url));
    // Light preview inverts the light fill and keeps the dark stroke.
    assert!(
        light_pixels.contains(&rgba("#5a2700")),
        "light fill adapted"
    );
    assert!(light_pixels.contains(&rgba("#1e1e1e")), "light stroke kept");
    assert!(
        light_pixels.contains(&rgba("#ffffff")),
        "light preview background"
    );
    // Dark preview keeps the light fill, inverts the dark stroke and paints
    // the shipped dark canvas colour.
    assert!(dark_pixels.contains(&rgba("#a5d8ff")), "dark fill kept");
    assert!(
        dark_pixels.contains(&rgba("#e1e1e1")),
        "dark stroke adapted"
    );
    assert!(
        dark_pixels.contains(&rgba("#232329")),
        "dark preview background"
    );
}

#[test]
fn frame_fixture_renders_excalidraw_frame_chrome() {
    let doc = fixture("frame.excalidraw");
    let svg = export_svg(&doc, &SceneOptions::default()).unwrap();
    assert!(svg.contains("stroke=\"#bbb\""));
    assert!(svg.contains("stroke-width=\"2\""));
    assert!(svg.contains("fill=\"#999999\""));
    assert!(svg.contains("font-size=\"14\""));
    assert!(svg.contains("rx=\"8\" ry=\"8\""));
    assert!(svg.contains(">Architecture</text>"));
    assert!(!svg.contains("#e9ecef"));

    let scene = Scene::build(&doc, &SceneOptions::default()).unwrap();
    let bounds = scene.bounds.expect("frame fixture has bounds");
    let image = render_scene(&scene, Some(400)).unwrap();
    assert!(ink_coverage(&image.png, "#ffffff").unwrap() > 0.001);
    let pixels = decode_png(&image.png);
    assert!(
        pixels.iter().any(|pixel| near(pixel, "#bbb", 6)),
        "the rounded #bbb frame border must paint"
    );
    assert!(
        pixels.iter().any(|pixel| near(pixel, "#a5d8ff", 6)),
        "the frame child shape must paint"
    );
    assert!(
        !pixels.iter().any(|pixel| near(pixel, "#e9ecef", 3)),
        "the old title-tab fill must not paint"
    );

    let scale = f64::from(image.width) / bounds.w;
    let frame = &scene.frames[0];
    let frame_top = ((frame.rect.y - bounds.y) * scale).floor() as usize;
    let row_width = image.width as usize;
    // The frame title is the only ink above the frame's top edge.
    let title_rows = &pixels[..frame_top * row_width];
    let darkest = title_rows
        .iter()
        .filter(|pixel| pixel[3] == 255)
        .map(|pixel| pixel[..3].iter().copied().max().unwrap_or(255))
        .min()
        .unwrap_or(255);
    assert!(
        darkest < 200,
        "the frame title must render above the frame top edge"
    );
    // A square border would paint the frame's exact top-left corner; the
    // rounded border leaves it as background.
    let left = ((frame.rect.x - bounds.x) * scale).floor() as usize;
    let corner = pixels[frame_top * row_width + left];
    assert!(
        near(&corner, "#ffffff", 8),
        "the frame corner must be rounded, got {corner:?}"
    );
    // The middle of the left edge is border ink.
    let middle = ((frame.rect.y + frame.rect.h / 2.0 - bounds.y) * scale).floor() as usize;
    let edge = pixels[middle * row_width + left];
    assert!(
        near(&edge, "#bbb", 8),
        "the frame border must paint its left edge, got {edge:?}"
    );
}

/// Board with a large solid light fill and a dark stroke.
fn dark_stroke_board() -> BoardDocument {
    let mut doc = BoardDocument::empty();
    let mut rect = Element::new(ElementKind::Rectangle, "board", 0.0, 0.0, 400.0, 300.0);
    rect.set_style(&Style {
        background_color: "#a5d8ff".to_string(),
        stroke_color: "#1e1e1e".to_string(),
        fill_style: FillStyle::Solid,
        ..Style::default()
    });
    doc.add_element(rect).unwrap();
    doc
}

/// Decode the base64 payload of a WebP `data:` URL.
fn thumbnail_bytes(data_url: &str) -> Vec<u8> {
    let payload = data_url
        .strip_prefix("data:image/webp;base64,")
        .expect("webp data URL");
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .expect("thumbnail payload decodes")
}
