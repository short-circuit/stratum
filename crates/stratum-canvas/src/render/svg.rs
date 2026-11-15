//! Deterministic SVG export for canvas scenes.
//!
//! [`scene_svg`] serialises a [`Scene`] as SVG 1.1: rough stroke paths, frame
//! outlines/titles, embedded images and text blocks. Attribute order, number
//! precision (three decimals, except frame chrome values which keep
//! Excalidraw's bare-integer form) and id assignment are fixed so two exports
//! of the same scene are byte-identical, which makes exports diffable and lets
//! tests assert on substrings.
//!
//! Frame children are wrapped in a rounded `clipPath` derived from the frame
//! rect so they never paint outside their frame. Frame chrome matches
//! Excalidraw's own export: a rounded `#bbb` outline, no title tab and a plain
//! `#999999` name above the frame. Text keeps the Excalidraw family in
//! `data-excalidraw-font` while rendering with the embedded Inter face
//! (`font-family="Inter"`), so a reopened board still carries its original
//! font choice.
//!
//! An empty board has no content bounds; [`scene_svg`] then emits a
//! documented `100 x 100` background-only canvas instead of failing.

use pkm_core::PkmResult;

use crate::document::BoardDocument;
use crate::geometry::Rect;
use crate::render::scene::{Scene, SceneFrame, SceneImage, SceneOptions, ScenePath, SceneText};
use crate::render::svg_path::format_number;

/// Frame border stroke colour, matching Excalidraw's frame chrome.
pub const FRAME_BORDER_COLOR: &str = "#bbb";
/// Frame border stroke width in world units, matching Excalidraw's frame chrome.
pub const FRAME_BORDER_WIDTH: f64 = 2.0;
/// Corner radius of the frame border and the frame clip path in world units.
pub const FRAME_RADIUS: f64 = 8.0;
/// Frame title text colour, matching Excalidraw's frame chrome.
pub const FRAME_TITLE_COLOR: &str = "#999999";
/// Frame title font size in world units, matching Excalidraw's frame chrome.
pub const FRAME_TITLE_SIZE: f64 = 14.0;
/// Distance of the frame title baseline above the frame's top edge.
pub const FRAME_TITLE_OFFSET: f64 = 8.0;
/// Excalidraw's default line height multiplier.
const TEXT_LINE_HEIGHT: f64 = 1.25;
/// Baseline offset as a fraction of the font size (Inter ascent estimate).
const TEXT_BASELINE: f64 = 0.9;
/// Fallback canvas size for an empty board.
const DEFAULT_SIZE: f64 = 100.0;
/// Placeholder fill for images without an embedded data URL.
const IMAGE_PLACEHOLDER_FILL: &str = "#e9ecef";

/// Build a scene from `doc` and serialise it as SVG.
///
/// Exports keep the document's own colours and canvas background: the shipped
/// gallery theme adaptation is never applied here, even when `options` carries
/// a [`crate::theme::PreviewTheme`]. This matches Excalidraw's `exportToSvg`
/// output.
///
/// # Errors
///
/// Propagates [`pkm_core::PkmError::Canvas`] when the scene cannot be built.
pub fn export_svg(doc: &BoardDocument, options: &SceneOptions) -> PkmResult<String> {
    let options = SceneOptions {
        adapt_theme: false,
        ..*options
    };
    let scene = Scene::build(doc, &options)?;
    Ok(scene_svg(&scene))
}

/// Serialise a scene as a complete SVG document.
pub fn scene_svg(scene: &Scene) -> String {
    let bounds = scene
        .bounds
        .unwrap_or_else(|| Rect::new(0.0, 0.0, DEFAULT_SIZE, DEFAULT_SIZE));
    let mut clips = ClipRegistry::default();
    register_clips(&mut clips, scene);

    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\"");
    push_attr(&mut out, "width", &format_number(bounds.w));
    push_attr(&mut out, "height", &format_number(bounds.h));
    push_attr(
        &mut out,
        "viewBox",
        &format!(
            "{} {} {} {}",
            format_number(bounds.x),
            format_number(bounds.y),
            format_number(bounds.w),
            format_number(bounds.h)
        ),
    );
    out.push_str(">\n");
    write_clips(&mut out, &clips);
    write_background(&mut out, scene, &bounds);
    for frame in &scene.frames {
        write_frame(&mut out, frame);
    }
    for image in &scene.images {
        write_image(&mut out, image, &clips);
    }
    for path in &scene.paths {
        write_path(&mut out, path, &clips);
    }
    for text in &scene.texts {
        write_text(&mut out, text, &clips);
    }
    out.push_str("</svg>\n");
    out
}

/// Clip rect registry assigning deterministic ids in first-use order.
#[derive(Default)]
struct ClipRegistry {
    /// Registered rectangles.
    rects: Vec<Rect>,
}

impl ClipRegistry {
    /// Id for `rect`, adding it when it is new.
    fn id_for(&mut self, rect: Rect) -> usize {
        if let Some(index) = self.rects.iter().position(|existing| *existing == rect) {
            return index;
        }
        self.rects.push(rect);
        self.rects.len() - 1
    }

    /// Look up an already-registered id without mutating.
    fn lookup(&self, rect: Rect) -> Option<usize> {
        self.rects.iter().position(|existing| *existing == rect)
    }
}

/// Register every clip used by the scene in render order.
fn register_clips(clips: &mut ClipRegistry, scene: &Scene) {
    for image in &scene.images {
        if let Some(clip) = image.clip {
            clips.id_for(clip);
        }
    }
    for path in &scene.paths {
        if let Some(clip) = path.clip {
            clips.id_for(clip);
        }
    }
    for text in &scene.texts {
        if let Some(clip) = text.clip {
            clips.id_for(clip);
        }
    }
}

/// Emit the clipPath definitions.
///
/// Every registered clip comes from a frame membership lookup in
/// `scene_build`, so the clip rect uses the frame corner radius to match the
/// rounded frame border.
fn write_clips(out: &mut String, clips: &ClipRegistry) {
    if clips.rects.is_empty() {
        return;
    }
    out.push_str("<defs>\n");
    for (index, rect) in clips.rects.iter().enumerate() {
        out.push_str(&format!("<clipPath id=\"clip-{index}\">"));
        out.push_str("<rect");
        push_rect_attrs(out, rect);
        push_rounded_attrs(out, FRAME_RADIUS);
        out.push_str("/></clipPath>\n");
    }
    out.push_str("</defs>\n");
}

/// Emit the background rectangle.
fn write_background(out: &mut String, scene: &Scene, bounds: &Rect) {
    out.push_str("<rect");
    push_rect_attrs(out, bounds);
    push_attr(out, "fill", &escape(&scene.background));
    out.push_str("/>\n");
}

/// Emit one frame outline and title.
///
/// Mirrors Excalidraw's own frame export: the name is plain text left-aligned
/// at the frame's left edge with its baseline [`FRAME_TITLE_OFFSET`] above the
/// frame's top edge, and the outline is a rounded rectangle with no title tab
/// or background fill.
fn write_frame(out: &mut String, frame: &SceneFrame) {
    let rect = frame.rect;
    out.push_str("<g");
    push_opacity(out, frame.opacity);
    out.push_str(">\n");
    if let Some(name) = &frame.name {
        if !name.is_empty() {
            let x = rect.x;
            let y = rect.y - FRAME_TITLE_OFFSET;
            out.push_str("<text");
            push_attr(out, "x", &format_number(x));
            push_attr(out, "y", &format_number(y));
            push_attr(out, "font-family", "Inter");
            push_attr(out, "font-size", &format_compact(FRAME_TITLE_SIZE));
            push_attr(out, "fill", FRAME_TITLE_COLOR);
            push_attr(out, "text-anchor", "start");
            out.push('>');
            out.push_str(&escape(name));
            out.push_str("</text>\n");
        }
    }
    out.push_str("<rect");
    push_rect_attrs(out, &rect);
    push_rounded_attrs(out, FRAME_RADIUS);
    push_attr(out, "fill", "none");
    push_attr(out, "stroke", FRAME_BORDER_COLOR);
    push_attr(out, "stroke-width", &format_compact(FRAME_BORDER_WIDTH));
    out.push_str("/>\n");
    out.push_str("</g>\n");
}

/// Emit one embedded image (or a placeholder when it has no data URL).
fn write_image(out: &mut String, image: &SceneImage, clips: &ClipRegistry) {
    out.push_str("<g");
    push_opacity(out, image.opacity);
    if let Some(clip) = image.clip {
        push_clip(out, clips, clip);
    }
    out.push_str(">\n");
    match image.data_url.as_deref() {
        Some(url) if !url.is_empty() => {
            out.push_str("<image");
            push_attr(out, "x", &format_number(image.x));
            push_attr(out, "y", &format_number(image.y));
            push_attr(out, "width", &format_number(image.width));
            push_attr(out, "height", &format_number(image.height));
            push_attr(out, "href", &escape(url));
            push_attr(out, "preserveAspectRatio", "none");
            out.push_str("/>\n");
        }
        _ => {
            let rect = Rect::new(image.x, image.y, image.width, image.height);
            out.push_str("<rect");
            push_rect_attrs(out, &rect);
            push_attr(out, "fill", IMAGE_PLACEHOLDER_FILL);
            push_attr(out, "stroke", FRAME_BORDER_COLOR);
            push_attr(out, "stroke-dasharray", "4.000 4.000");
            out.push_str("/>\n");
        }
    }
    out.push_str("</g>\n");
}

/// Emit one rough stroke/fill path.
fn write_path(out: &mut String, path: &ScenePath, clips: &ClipRegistry) {
    let d = crate::render::svg_path::PathData {
        ops: path.ops.clone(),
    }
    .to_d();
    if d.is_empty() {
        return;
    }
    out.push_str("<path");
    push_attr(out, "d", &escape(&d));
    push_attr(out, "fill", path.fill.as_deref().unwrap_or("none"));
    match &path.stroke {
        Some(stroke) => {
            push_attr(out, "stroke", stroke);
            push_attr(out, "stroke-width", &format_number(path.stroke_width));
            if let Some(dash) = &path.dash {
                let pattern = dash
                    .iter()
                    .map(|value| format_number(*value))
                    .collect::<Vec<_>>()
                    .join(" ");
                push_attr(out, "stroke-dasharray", &pattern);
            }
            push_attr(out, "stroke-linecap", "round");
            push_attr(out, "stroke-linejoin", "round");
        }
        None => push_attr(out, "stroke", "none"),
    }
    push_opacity(out, path.opacity);
    if let Some(clip) = path.clip {
        push_clip(out, clips, clip);
    }
    out.push_str("/>\n");
}

/// Emit one text block with one `tspan` per wrapped line.
fn write_text(out: &mut String, text: &SceneText, clips: &ClipRegistry) {
    if text.lines.is_empty() {
        return;
    }
    let anchor = match text.align.as_str() {
        "center" => "middle",
        "right" => "end",
        _ => "start",
    };
    let base_x = match anchor {
        "middle" => text.x + text.width / 2.0,
        "end" => text.x + text.width,
        _ => text.x,
    };
    let total_height = text.lines.len() as f64 * text.font_size * TEXT_LINE_HEIGHT;
    let start_y = match text.vertical_align.as_str() {
        "middle" => text.y + (text.height - total_height) / 2.0,
        "bottom" => text.y + text.height - total_height,
        _ => text.y,
    };
    out.push_str("<text");
    push_attr(out, "font-family", "Inter");
    push_attr(out, "font-size", &format_number(text.font_size));
    push_attr(out, "fill", &escape(&text.color));
    push_attr(out, "text-anchor", anchor);
    push_attr(
        out,
        "data-excalidraw-font",
        crate::render::fonts::family_name(text.font_family),
    );
    push_opacity(out, text.opacity);
    if let Some(clip) = text.clip {
        push_clip(out, clips, clip);
    }
    out.push_str(">\n");
    for (index, line) in text.lines.iter().enumerate() {
        let y = start_y
            + index as f64 * text.font_size * TEXT_LINE_HEIGHT
            + TEXT_BASELINE * text.font_size;
        out.push_str("<tspan");
        push_attr(out, "x", &format_number(base_x));
        push_attr(out, "y", &format_number(y));
        out.push('>');
        out.push_str(&escape(line));
        out.push_str("</tspan>\n");
    }
    out.push_str("</text>\n");
}

/// Push a clip-path attribute for a registered rect.
fn push_clip(out: &mut String, clips: &ClipRegistry, rect: Rect) {
    if let Some(index) = clips.lookup(rect) {
        push_attr(out, "clip-path", &format!("url(#clip-{index})"));
    }
}

/// Push the rect geometry attributes in a fixed order.
fn push_rect_attrs(out: &mut String, rect: &Rect) {
    push_attr(out, "x", &format_number(rect.x));
    push_attr(out, "y", &format_number(rect.y));
    push_attr(out, "width", &format_number(rect.w));
    push_attr(out, "height", &format_number(rect.h));
}

/// Push matching `rx`/`ry` corner radius attributes.
fn push_rounded_attrs(out: &mut String, radius: f64) {
    push_attr(out, "rx", &format_compact(radius));
    push_attr(out, "ry", &format_compact(radius));
}

/// Format a frame-chrome value the way Excalidraw does: bare integers for
/// whole numbers, three decimals otherwise.
///
/// The frame border, title and clip use this instead of [`format_number`] so
/// the export carries `stroke-width="2"`, `font-size="14"` and `rx="8"` exactly
/// like Excalidraw's own `exportToSvg` output.
fn format_compact(value: f64) -> String {
    let formatted = format_number(value);
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Push an opacity attribute when the item is not fully opaque.
fn push_opacity(out: &mut String, opacity: f64) {
    if opacity < 1.0 {
        push_attr(out, "opacity", &format_number(opacity));
    }
}

/// Push one `name="value"` attribute.
fn push_attr(out: &mut String, name: &str, value: &str) {
    out.push(' ');
    out.push_str(name);
    out.push_str("=\"");
    out.push_str(value);
    out.push('"');
}

/// Escape XML text and attribute content.
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Element;
    use crate::element_kind::ElementKind;
    use crate::fixture::sample_board;

    #[test]
    fn exports_a_deterministic_document() {
        let doc = sample_board();
        let options = SceneOptions::default();
        let first = export_svg(&doc, &options).unwrap();
        let second = export_svg(&doc, &options).unwrap();
        assert_eq!(first, second);
        assert!(first.starts_with("<?xml"));
        assert!(first.contains("<svg xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(first.contains("viewBox="));
        assert!(first.contains("data-excalidraw-font=\"Helvetica\""));
        assert!(first.contains("Architecture"));
    }

    #[test]
    fn frame_children_get_clip_paths() {
        let doc = sample_board();
        let svg = export_svg(&doc, &SceneOptions::default()).unwrap();
        assert!(svg.contains("<clipPath id=\"clip-0\">"));
        assert!(svg.contains("clip-path=\"url(#clip-0)\""));
        // Frame clips are rounded to match the rounded frame border.
        assert!(svg.contains("<rect x=\""));
        assert!(svg.contains("rx=\"8\" ry=\"8\"/>"));
    }

    #[test]
    fn frame_chrome_matches_excalidraw() {
        let mut doc = BoardDocument::empty();
        let mut frame = Element::new(ElementKind::Frame, "frame-1", 100.0, 100.0, 400.0, 300.0);
        frame.set_frame_name(Some("A < B & C"));
        doc.add_element(frame).unwrap();
        let svg = export_svg(&doc, &SceneOptions::default()).unwrap();
        assert!(svg.contains("stroke=\"#bbb\""));
        assert!(svg.contains("stroke-width=\"2\""));
        assert!(svg.contains("fill=\"#999999\""));
        assert!(svg.contains("font-size=\"14\""));
        assert!(svg.contains("text-anchor=\"start\""));
        assert!(svg.contains("rx=\"8\" ry=\"8\""));
        // The title sits at the frame's left edge with an 8-unit baseline
        // offset above the frame top, and is XML-escaped.
        assert!(svg.contains("x=\"100.000\" y=\"92.000\""));
        assert!(svg.contains(">A &lt; B &amp; C</text>"));
        assert!(!svg.contains("A < B & C"));
        // No title tab and none of the old React frame colours.
        assert!(!svg.contains("#e9ecef"));
        assert!(!svg.contains("#ced4da"));
    }

    #[test]
    fn escapes_text_and_attributes() {
        assert_eq!(escape("a & b < c"), "a &amp; b &lt; c");
        assert_eq!(escape("\"x\""), "&quot;x&quot;");
    }

    #[test]
    fn empty_board_uses_the_default_canvas() {
        let doc = BoardDocument::empty();
        let svg = export_svg(&doc, &SceneOptions::default()).unwrap();
        assert!(svg.contains("width=\"100.000\""));
        assert!(svg.contains("height=\"100.000\""));
    }
}
