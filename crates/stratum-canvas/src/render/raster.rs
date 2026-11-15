//! Raster export for canvas scenes.
//!
//! [`render_scene`] rasterises the SVG produced by [`crate::render::svg`] with
//! `resvg`, using only the embedded Inter face (no system fonts). The result
//! can be encoded as PNG or lossless WebP and wrapped as a `data:` URL for the
//! board preview.
//!
//! Raster sizes are bounded by [`MAX_RASTER_EDGE`]; larger boards are scaled
//! down so a single export cannot exhaust memory. Thumbnails never upscale and
//! are capped at [`THUMBNAIL_MAX_EDGE`], matching the shipped ~300 px preview.
//!
//! WebP output is VP8L lossless only (`image-webp` has no lossy encoder). The
//! scene always paints an opaque background first, so the premultiplied
//! tiny-skia buffer and WebP's straight-alpha expectation agree.
//!
//! [`ink_coverage`] decodes a rendered PNG and reports the fraction of pixels
//! that differ from the background; tests use it to prove a render is not
//! blank.

use base64::Engine;
use pkm_core::{PkmError, PkmResult};
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;

use crate::document::BoardDocument;
use crate::render::color::parse_color;
use crate::render::fonts::INTER_REGULAR;
use crate::render::scene::{Scene, SceneOptions};
use crate::render::svg::scene_svg;
use crate::theme::{theme_background, PreviewTheme};

/// Maximum raster edge in pixels.
pub const MAX_RASTER_EDGE: u32 = 4096;
/// Maximum thumbnail edge in pixels (shipped preview size).
pub const THUMBNAIL_MAX_EDGE: u32 = 300;
/// Channel difference above which a pixel counts as ink.
const INK_THRESHOLD: u8 = 8;

/// A rasterised scene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedImage {
    /// PNG bytes.
    pub png: Vec<u8>,
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
}

/// A rendered board preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thumbnail {
    /// Lossless WebP `data:` URL.
    pub data_url: String,
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
}

/// Rasterise `scene` as PNG.
///
/// `max_edge` caps the longest output edge; `None` renders at one pixel per
/// world unit, still clamped to [`MAX_RASTER_EDGE`]. Thumbnails never upscale.
///
/// # Errors
///
/// Returns [`PkmError::Canvas`] for a zero edge cap, an SVG parse failure or a
/// zero-sized pixmap allocation.
pub fn render_scene(scene: &Scene, max_edge: Option<u32>) -> PkmResult<RenderedImage> {
    let svg = scene_svg(scene);
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_font_data(INTER_REGULAR.to_vec());
    let tree = usvg::Tree::from_str(&svg, &options)
        .map_err(|error| PkmError::Canvas(format!("svg parse: {error}")))?;
    let size = tree.size();
    let width = f64::from(size.width());
    let height = f64::from(size.height());
    let longest = width.max(height);
    if longest <= 0.0 || !longest.is_finite() {
        return Err(PkmError::Canvas("scene has no raster extent".to_string()));
    }
    let scale = raster_scale(longest, max_edge)?;
    let pixel_width = ((width * scale).round() as u32).max(1);
    let pixel_height = ((height * scale).round() as u32).max(1);
    let mut pixmap = Pixmap::new(pixel_width, pixel_height)
        .ok_or_else(|| PkmError::Canvas("raster allocation failed".to_string()))?;
    resvg::render(
        &tree,
        Transform::from_scale(scale as f32, scale as f32),
        &mut pixmap.as_mut(),
    );
    let png = pixmap
        .encode_png()
        .map_err(|error| PkmError::Canvas(format!("png encode: {error}")))?;
    Ok(RenderedImage {
        png,
        width: pixel_width,
        height: pixel_height,
    })
}

/// Compute the raster scale for a scene of `longest` world units.
fn raster_scale(longest: f64, max_edge: Option<u32>) -> PkmResult<f64> {
    let mut scale = match max_edge {
        Some(0) => {
            return Err(PkmError::Canvas(
                "raster max edge must be greater than zero".to_string(),
            ))
        }
        Some(max) => (f64::from(max) / longest).min(1.0),
        None => 1.0,
    };
    let guard = f64::from(MAX_RASTER_EDGE) / longest;
    if scale > guard {
        scale = guard;
    }
    Ok(scale)
}

/// Render `doc` as PNG.
///
/// # Errors
///
/// Propagates scene-build and rasterisation errors.
pub fn export_png(
    doc: &BoardDocument,
    options: &SceneOptions,
    max_edge: Option<u32>,
) -> PkmResult<Vec<u8>> {
    let scene = Scene::build(doc, &export_options(options))?;
    Ok(render_scene(&scene, max_edge)?.png)
}

/// Render `doc` as a lossless WebP byte stream.
///
/// # Errors
///
/// Propagates scene-build and rasterisation errors.
pub fn export_webp(
    doc: &BoardDocument,
    options: &SceneOptions,
    max_edge: Option<u32>,
) -> PkmResult<Vec<u8>> {
    let scene = Scene::build(doc, &export_options(options))?;
    let image = render_scene(&scene, max_edge)?;
    encode_webp(&image)
}

/// Export options derived from `options`: exports never adapt the document's
/// colours, even when the caller asks for a preview theme.
///
/// The gallery thumbnail is the only surface that applies the shipped theme
/// adaptation; SVG/PNG/WebP must match Excalidraw's own `exportToSvg` output.
fn export_options(options: &SceneOptions) -> SceneOptions {
    SceneOptions {
        adapt_theme: false,
        ..*options
    }
}

/// Encode a rendered image as lossless WebP.
///
/// # Errors
///
/// Returns [`PkmError::Canvas`] when the encoder fails.
pub fn encode_webp(image: &RenderedImage) -> PkmResult<Vec<u8>> {
    let pixmap = Pixmap::decode_png(&image.png)
        .map_err(|error| PkmError::Canvas(format!("png decode: {error}")))?;
    let mut out = Vec::new();
    image_webp::WebPEncoder::new(&mut out)
        .encode(
            pixmap.data(),
            image.width,
            image.height,
            image_webp::ColorType::Rgba8,
        )
        .map_err(|error| PkmError::Canvas(format!("webp encode: {error}")))?;
    Ok(out)
}

/// Wrap WebP bytes in a `data:` URL.
pub fn webp_data_url(bytes: &[u8]) -> String {
    format!(
        "data:image/webp;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

/// Render a ~300 px preview for `doc` in the requested theme.
///
/// # Errors
///
/// Propagates scene-build, rasterisation and encoding errors.
pub fn thumbnail(doc: &BoardDocument, theme: PreviewTheme) -> PkmResult<Thumbnail> {
    let options = SceneOptions {
        theme,
        // The gallery preview is the only surface that applies the shipped
        // theme adaptation; exports keep the document's raw colours.
        adapt_theme: true,
        ..SceneOptions::default()
    };
    let mut scene = Scene::build(doc, &options)?;
    // The shipped gallery always paints light previews on #ffffff and dark
    // previews on #232329, regardless of the board's canvas colour.
    scene.background = theme_background(theme);
    let image = render_scene(&scene, Some(THUMBNAIL_MAX_EDGE))?;
    let bytes = encode_webp(&image)?;
    Ok(Thumbnail {
        data_url: webp_data_url(&bytes),
        width: image.width,
        height: image.height,
    })
}

/// Fraction of non-background pixels in a rendered PNG.
///
/// Returns `0.0` for a fully blank render. The background colour is parsed
/// from `background`; unparseable colours fall back to opaque white.
///
/// # Errors
///
/// Returns [`PkmError::Canvas`] when the PNG cannot be decoded.
pub fn ink_coverage(png: &[u8], background: &str) -> PkmResult<f64> {
    let pixmap = Pixmap::decode_png(png)
        .map_err(|error| PkmError::Canvas(format!("png decode: {error}")))?;
    let (br, bg, bb, _) = parse_color(background).unwrap_or((255, 255, 255, 255));
    let total = u64::from(pixmap.width()) * u64::from(pixmap.height());
    if total == 0 {
        return Ok(0.0);
    }
    let mut ink = 0_u64;
    for pixel in pixmap.pixels() {
        let color = pixel.demultiply();
        let diff = color
            .red()
            .abs_diff(br)
            .max(color.green().abs_diff(bg))
            .max(color.blue().abs_diff(bb));
        if diff > INK_THRESHOLD {
            ink += 1;
        }
    }
    Ok(ink as f64 / total as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::sample_board;

    #[test]
    fn renders_png_and_webp() {
        let doc = sample_board();
        let png = export_png(&doc, &SceneOptions::default(), Some(300)).unwrap();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        let webp = export_webp(&doc, &SceneOptions::default(), Some(300)).unwrap();
        assert_eq!(&webp[..4], b"RIFF");
        assert_eq!(&webp[8..12], b"WEBP");
    }

    #[test]
    fn thumbnail_is_a_webp_data_url_within_the_edge_cap() {
        let doc = sample_board();
        let thumb = thumbnail(&doc, PreviewTheme::Light).unwrap();
        assert!(thumb.data_url.starts_with("data:image/webp;base64,"));
        assert!(thumb.width <= THUMBNAIL_MAX_EDGE);
        assert!(thumb.height <= THUMBNAIL_MAX_EDGE);
    }

    #[test]
    fn ink_coverage_detects_content_and_blank_renders() {
        let doc = sample_board();
        let png = export_png(&doc, &SceneOptions::default(), Some(300)).unwrap();
        let coverage = ink_coverage(&png, "#ffffff").unwrap();
        assert!(coverage > 0.01);
        let empty = BoardDocument::empty();
        let blank = export_png(&empty, &SceneOptions::default(), Some(100)).unwrap();
        assert_eq!(ink_coverage(&blank, "#ffffff").unwrap(), 0.0);
    }

    #[test]
    fn rejects_zero_edge_cap() {
        let doc = sample_board();
        assert!(export_png(&doc, &SceneOptions::default(), Some(0)).is_err());
    }
}
