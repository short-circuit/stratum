//! Scene construction from a [`crate::document::BoardDocument`].
//!
//! Split out of `scene.rs` to keep every source file under the 500-line limit.
//! The builder walks the document in z-order, resolves frame clips, calls the
//! rough generator for shape geometry, lays out text with [`crate::text`],
//! resolves embedded image data URLs and accumulates the scene bounds.

use pkm_core::PkmResult;
use serde_json::Value;

use crate::document::BoardDocument;
use crate::element::Element;
use crate::element_kind::ElementKind;
use crate::files::file_entry;
use crate::geometry::{element_rect, Rect};
use crate::render::rough::{arrowhead_paths, element_paths};
use crate::render::scene::{
    export_background, path_bounds, Scene, SceneFrame, SceneImage, SceneOptions, ScenePath,
    SceneText,
};
use crate::text::layout;
use crate::theme::adapt_color_for_theme;

/// Frame title bar height in world units (matches `geometry.rs`).
const FRAME_TITLE_HEIGHT: f64 = 24.0;

/// Accumulates the scene bounds while elements are built.
#[derive(Default)]
struct Bounds {
    rect: Option<Rect>,
}

impl Bounds {
    /// Add one rectangle.
    fn add(&mut self, rect: Rect) {
        if rect.w <= 0.0 && rect.h <= 0.0 {
            return;
        }
        self.rect = Some(match self.rect {
            Some(current) => current.union(&rect),
            None => rect,
        });
    }

    /// Finish with padding applied.
    fn finish(self, padding: f64) -> Option<Rect> {
        self.rect
            .map(|rect| rect.inflate(padding.max(0.0)).normalized())
    }
}

/// Build a scene for `doc`.
pub(crate) fn build(doc: &BoardDocument, options: &SceneOptions) -> PkmResult<Scene> {
    let revision = super::scene::document_revision(doc);
    let mut scene = Scene {
        paths: Vec::new(),
        texts: Vec::new(),
        images: Vec::new(),
        frames: Vec::new(),
        bounds: None,
        background: export_background(doc, options),
        revision,
    };
    let frames = frame_clips(doc);
    let mut bounds = Bounds::default();

    for element in doc.visible_elements() {
        let clip = element
            .frame_id()
            .and_then(|id| frames.iter().find(|(frame, _)| *frame == id))
            .map(|(_, rect)| *rect);
        match element.kind() {
            ElementKind::Text => {
                if options.include_text {
                    if let Some(text) = text_item(element, options, clip) {
                        bounds.add(Rect::new(text.x, text.y, text.width, text.height));
                        scene.texts.push(text);
                    }
                }
            }
            ElementKind::Image => {
                if options.include_images {
                    if let Some(image) = image_item(doc, element, clip) {
                        bounds.add(Rect::new(image.x, image.y, image.width, image.height));
                        scene.images.push(image);
                    }
                }
            }
            ElementKind::Frame => {
                if options.include_frames {
                    if let Some(frame) = frame_item(element) {
                        let mut rect = frame.rect;
                        rect.y -= FRAME_TITLE_HEIGHT;
                        rect.h += FRAME_TITLE_HEIGHT;
                        bounds.add(rect);
                        scene.frames.push(frame);
                    }
                }
            }
            _ => {
                let mut paths = element_paths(element)?;
                paths.extend(arrowhead_paths(element));
                for path in paths {
                    let mut path = adapt_path(path, options);
                    path.clip = clip;
                    bounds.add(path_bounds(&path.ops));
                    scene.paths.push(path);
                }
            }
        }
    }

    scene.bounds = bounds.finish(options.padding);
    Ok(scene)
}

/// Frame id/rect pairs for clipping frame children.
fn frame_clips(doc: &BoardDocument) -> Vec<(String, Rect)> {
    doc.frames()
        .into_iter()
        .map(|frame| (frame.id().to_string(), element_rect(frame)))
        .collect()
}

/// Adapt a path's colours for the preview theme.
///
/// Exports and the live editor surface keep the document's raw colours; only
/// [`SceneOptions::adapt_theme`] (the gallery thumbnail path) rewrites them.
fn adapt_path(mut path: ScenePath, options: &SceneOptions) -> ScenePath {
    path.fill = path
        .fill
        .as_deref()
        .map(|color| adapt_color(color, options));
    path.stroke = path
        .stroke
        .as_deref()
        .map(|color| adapt_color(color, options));
    path
}

/// Adapt one colour for the requested preview theme, or keep it raw.
fn adapt_color(color: &str, options: &SceneOptions) -> String {
    if options.adapt_theme {
        adapt_color_for_theme(color, options.theme)
    } else {
        color.to_string()
    }
}

/// Build one text scene item, or `None` when the element has no text.
fn text_item(element: &Element, options: &SceneOptions, clip: Option<Rect>) -> Option<SceneText> {
    let text = element.text().or_else(|| element.original_text())?;
    if text.is_empty() {
        return None;
    }
    let style = element.style();
    let max_width = if element.width() > 0.0 {
        element.width()
    } else {
        0.0
    };
    let layout = layout(&text, style.font_size, max_width, style.line_height);
    let width = if element.width() > 0.0 {
        element.width()
    } else {
        layout.width
    };
    let height = if element.height() > 0.0 {
        element.height()
    } else {
        layout.height
    };
    Some(SceneText {
        element_id: element.id().to_string(),
        x: element.x(),
        y: element.y(),
        width,
        height,
        lines: layout.lines,
        font_family: style.font_family.as_wire() as u8,
        font_size: style.font_size,
        align: style.text_align.as_wire().to_string(),
        vertical_align: style.vertical_align.as_wire().to_string(),
        color: adapt_color(&style.stroke_color, options),
        opacity: opacity(style.opacity),
        clip,
    })
}

/// Build one image scene item, or `None` when the element has no size.
fn image_item(doc: &BoardDocument, element: &Element, clip: Option<Rect>) -> Option<SceneImage> {
    if element.width() <= 0.0 || element.height() <= 0.0 {
        return None;
    }
    let data_url = element
        .file_id()
        .and_then(|id| file_entry(doc.files(), &id))
        .and_then(|entry| entry.get("dataURL"))
        .and_then(Value::as_str)
        .map(str::to_string);
    Some(SceneImage {
        element_id: element.id().to_string(),
        x: element.x(),
        y: element.y(),
        width: element.width(),
        height: element.height(),
        data_url,
        opacity: opacity(element.style().opacity),
        clip,
    })
}

/// Build one frame scene item.
fn frame_item(element: &Element) -> Option<SceneFrame> {
    let rect = element_rect(element);
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return None;
    }
    Some(SceneFrame {
        element_id: element.id().to_string(),
        rect,
        name: element.frame_name(),
        opacity: opacity(element.style().opacity),
    })
}

/// Convert an Excalidraw opacity percentage into a `0.0..=1.0` factor.
pub(crate) fn opacity(percent: f64) -> f64 {
    (percent / 100.0).clamp(0.0, 1.0)
}
