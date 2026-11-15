//! Render scene model for the native whiteboard surface.
//!
//! [`Scene`] is the z-ordered, renderer-ready description of a board: rough
//! stroke paths, text blocks, embedded images and frame outlines. It is built
//! once per document revision by [`SceneCache`] (see `scene_cache.rs`) and then
//! queried per frame for the visible subset, which is what keeps pan/zoom
//! inside the 60 fps budget.
//!
//! Geometry stays in world coordinates. Frame children carry a `clip` rect so
//! both the gpui panel and the SVG writer can clip them to their frame.
//! Colours are the document's raw values by default; only the gallery preview
//! opts into the shipped theme adaptation through
//! [`SceneOptions::adapt_theme`]. Elements with `isDeleted` are skipped; text
//! glyphs are text nodes rather than rough paths; frames are outlines plus a
//! title label; images reference the document's embedded `files` map.

use pkm_core::PkmResult;

use crate::document::BoardDocument;
use crate::geometry::{Rect, Viewport};
use crate::theme::{theme_background, PreviewTheme};

use crate::render::svg_path::PathOp;

pub use crate::render::scene_cache::{SceneCache, VisibleSet};

/// One stroked/filled path in the scene.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenePath {
    /// Owning element id.
    pub element_id: String,
    /// Path operations in world coordinates.
    pub ops: Vec<PathOp>,
    /// Fill colour, when the path is a fill.
    pub fill: Option<String>,
    /// Stroke colour, when the path is stroked.
    pub stroke: Option<String>,
    /// Stroke width in world units.
    pub stroke_width: f64,
    /// Dash pattern for dashed/dotted strokes.
    pub dash: Option<Vec<f64>>,
    /// Opacity in `0.0..=1.0`.
    pub opacity: f64,
    /// Frame clip rect for frame children.
    pub clip: Option<Rect>,
}

/// One text block in the scene.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneText {
    /// Owning element id.
    pub element_id: String,
    /// Text origin x in world coordinates.
    pub x: f64,
    /// Text origin y in world coordinates.
    pub y: f64,
    /// Element width in world units.
    pub width: f64,
    /// Element height in world units.
    pub height: f64,
    /// Wrapped lines.
    pub lines: Vec<String>,
    /// Excalidraw `fontFamily` wire value.
    pub font_family: u8,
    /// Font size in world units.
    pub font_size: f64,
    /// Horizontal alignment (`left`/`center`/`right`).
    pub align: String,
    /// Vertical alignment (`top`/`middle`/`bottom`).
    pub vertical_align: String,
    /// Text colour.
    pub color: String,
    /// Opacity in `0.0..=1.0`.
    pub opacity: f64,
    /// Frame clip rect for frame children.
    pub clip: Option<Rect>,
}

/// One embedded image in the scene.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneImage {
    /// Owning element id.
    pub element_id: String,
    /// Image origin x in world coordinates.
    pub x: f64,
    /// Image origin y in world coordinates.
    pub y: f64,
    /// Image width in world units.
    pub width: f64,
    /// Image height in world units.
    pub height: f64,
    /// Embedded data URL, when the document carries one.
    pub data_url: Option<String>,
    /// Opacity in `0.0..=1.0`.
    pub opacity: f64,
    /// Frame clip rect for frame children.
    pub clip: Option<Rect>,
}

/// One frame outline and title in the scene.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneFrame {
    /// Owning element id.
    pub element_id: String,
    /// Frame rectangle in world coordinates.
    pub rect: Rect,
    /// Frame name, when set.
    pub name: Option<String>,
    /// Opacity in `0.0..=1.0`.
    pub opacity: f64,
}

/// Counts of scene items, used by tests and UI status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneCounts {
    /// Number of paths.
    pub paths: usize,
    /// Number of text blocks.
    pub texts: usize,
    /// Number of images.
    pub images: usize,
    /// Number of frames.
    pub frames: usize,
}

/// Options controlling scene construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneOptions {
    /// Preview theme used for colour adaptation and background.
    ///
    /// Only consulted when [`Self::adapt_theme`] is set; exports ignore it and
    /// keep the document's own colours and canvas background.
    pub theme: PreviewTheme,
    /// Whether to apply the shipped gallery-preview theme adaptation. Exports
    /// keep the document's own colours; only thumbnails adapt them.
    pub adapt_theme: bool,
    /// Padding added around the content bounds.
    pub padding: f64,
    /// Include text blocks.
    pub include_text: bool,
    /// Include embedded images.
    pub include_images: bool,
    /// Include frame outlines.
    pub include_frames: bool,
}

impl Default for SceneOptions {
    fn default() -> Self {
        Self {
            theme: PreviewTheme::Light,
            // Exports and the live editor surface show the document as drawn.
            // `raster::thumbnail` opts in explicitly.
            adapt_theme: false,
            padding: 10.0,
            include_text: true,
            include_images: true,
            include_frames: true,
        }
    }
}

/// Renderer-ready scene.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    /// Paths in document z-order.
    pub paths: Vec<ScenePath>,
    /// Text blocks in document z-order.
    pub texts: Vec<SceneText>,
    /// Images in document z-order.
    pub images: Vec<SceneImage>,
    /// Frames in document z-order.
    pub frames: Vec<SceneFrame>,
    /// Content bounds including padding, when the board is not empty.
    pub bounds: Option<Rect>,
    /// Background colour for the requested theme.
    pub background: String,
    /// Document revision the scene was built from.
    pub revision: u64,
}

impl Scene {
    /// Build the full scene in document z-order.
    ///
    /// Elements with `isDeleted` are skipped. Frame children are clipped to
    /// their frame by recording the frame rect in `clip`. Text uses the
    /// deterministic layout in [`crate::text`]; images resolve their data URL
    /// from the document's `files` map.
    ///
    /// # Errors
    ///
    /// Propagates [`pkm_core::PkmError::Canvas`] when a rough path cannot be
    /// generated or parsed.
    pub fn build(doc: &BoardDocument, options: &SceneOptions) -> PkmResult<Scene> {
        crate::render::scene_build::build(doc, options)
    }

    /// Counts for tests/UI status.
    pub fn counts(&self) -> SceneCounts {
        SceneCounts {
            paths: self.paths.len(),
            texts: self.texts.len(),
            images: self.images.len(),
            frames: self.frames.len(),
        }
    }
}

/// Background colour for a scene.
///
/// With [`SceneOptions::adapt_theme`] the gallery preview contract applies:
/// dark previews use the shipped dark canvas colour and light previews keep
/// the document's `viewBackgroundColor`. Without adaptation the document's own
/// canvas colour is always used, so an export keeps the board as drawn.
pub fn export_background(doc: &BoardDocument, options: &SceneOptions) -> String {
    if !options.adapt_theme {
        return doc.view_background_color();
    }
    match options.theme {
        PreviewTheme::Dark => theme_background(PreviewTheme::Dark),
        // A light preview keeps the board's canvas colour: element colours are
        // adapted (light strokes darken), but the background itself must not
        // flip — a white board would otherwise preview black. Thumbnails use
        // the fixed preview colours instead (see `raster::thumbnail`).
        PreviewTheme::Light => doc.view_background_color(),
    }
}

/// Stable document revision used by [`SceneCache`].
///
/// The hash covers every field that can change rendered output (geometry,
/// style, text, points, frame membership, files). It is intentionally cheap:
/// scene rebuilds happen on open/thumbnail/export, not per frame.
pub(crate) fn document_revision(doc: &BoardDocument) -> u64 {
    let mut hash = Fnv::new();
    hash.str(doc.source());
    hash.u64(doc.version());
    hash.str(&doc.view_background_color());
    hash.u64(doc.elements().len() as u64);
    for element in doc.elements() {
        hash.str(element.id());
        hash.str(element.kind().as_wire());
        hash.f64(element.x());
        hash.f64(element.y());
        hash.f64(element.width());
        hash.f64(element.height());
        hash.f64(element.angle());
        hash.bool(element.is_deleted());
        let style = element.style();
        hash.str(&style.stroke_color);
        hash.str(&style.background_color);
        hash.str(style.fill_style.as_wire());
        hash.f64(style.stroke_width);
        hash.str(style.stroke_style.as_wire());
        hash.f64(style.roughness);
        hash.f64(style.opacity);
        hash.u64(style.seed);
        hash.u64(style.font_family.as_wire());
        hash.f64(style.font_size);
        hash.str(style.text_align.as_wire());
        hash.str(style.vertical_align.as_wire());
        hash.f64(style.line_height);
        if let Some(roundness) = &style.roundness {
            hash.u64(u64::from(roundness.kind));
            hash.f64(roundness.value.unwrap_or(0.0));
        }
        if let Some(text) = element.text() {
            hash.str(&text);
        }
        if let Some(name) = element.frame_name() {
            hash.str(&name);
        }
        if let Some(frame) = element.frame_id() {
            hash.str(&frame);
        }
        if let Some(file) = element.file_id() {
            hash.str(&file);
        }
        let points = element.points();
        hash.u64(points.len() as u64);
        for point in points {
            hash.f64(point.x);
            hash.f64(point.y);
        }
        if let Some(head) = element.start_arrowhead() {
            hash.str(head.as_wire());
        }
        if let Some(head) = element.end_arrowhead() {
            hash.str(head.as_wire());
        }
    }
    if let Some(files) = doc.files().as_object() {
        for (key, value) in files {
            hash.str(key);
            if let Some(url) = value.get("dataURL").and_then(|v| v.as_str()) {
                hash.u64(url.len() as u64);
                hash.str(url);
            }
        }
    }
    hash.finish()
}

/// Small FNV-1a hasher used for the revision signature.
struct Fnv(u64);

impl Fnv {
    /// Start a hash.
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    /// Mix one byte.
    fn byte(&mut self, byte: u8) {
        self.0 ^= u64::from(byte);
        self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
    }

    /// Mix a string.
    fn str(&mut self, value: &str) {
        self.u64(value.len() as u64);
        for byte in value.as_bytes() {
            self.byte(*byte);
        }
    }

    /// Mix an integer.
    fn u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.byte(byte);
        }
    }

    /// Mix a float bit pattern.
    fn f64(&mut self, value: f64) {
        self.u64(value.to_bits());
    }

    /// Mix a boolean.
    fn bool(&mut self, value: bool) {
        self.byte(u8::from(value));
    }

    /// Finish the hash.
    fn finish(self) -> u64 {
        self.0
    }
}

/// Screen rectangle helper for the cache.
pub(crate) fn screen_rect(screen: Rect) -> Rect {
    Rect::new(0.0, 0.0, screen.w.max(1.0), screen.h.max(1.0))
}

/// World rectangle covered by a viewport and a screen size.
pub(crate) fn visible_rect(viewport: &Viewport, screen: Rect) -> Rect {
    let screen = screen_rect(screen);
    viewport.visible_world_rect(screen.w, screen.h)
}

/// Bounds of a path, computed from its operations without allocating.
pub(crate) fn path_bounds(ops: &[PathOp]) -> Rect {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut visit = |x: f64, y: f64| {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    };
    for op in ops {
        match op {
            PathOp::MoveTo(p) | PathOp::LineTo(p) => visit(p.x, p.y),
            PathOp::CurveTo { c1, c2, to } => {
                visit(c1.x, c1.y);
                visit(c2.x, c2.y);
                visit(to.x, to.y);
            }
            PathOp::Close => {}
        }
    }
    if min_x.is_finite() {
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    } else {
        Rect::new(0.0, 0.0, 0.0, 0.0)
    }
}
