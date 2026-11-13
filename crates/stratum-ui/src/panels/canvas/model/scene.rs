//! Scene-space helpers shared by the editor session and the fixture helpers.
//!
//! [`screen_scene`] rebuilds a small viewport-sized scene from cached scene
//! items so pan/zoom never rebuilds a 5k-element document.

use stratum_canvas::render::svg_path::PathData;
use stratum_canvas::render::{
    export_svg as scene_svg_for_document, Scene, SceneOptions, VisibleSet,
};
use stratum_canvas::{Affine, BoardDocument, Point, Rect};

/// Build a screen-space scene from the cached scene and visible indices.
pub(super) fn screen_scene(
    scene: &Scene,
    visible: &VisibleSet,
    affine: &Affine,
    zoom: f64,
    width: f64,
    height: f64,
) -> Scene {
    let transform = |rect: Rect| -> Rect {
        let top_left = affine.apply(Point::new(rect.x, rect.y));
        Rect::new(top_left.x, top_left.y, rect.w * zoom, rect.h * zoom)
    };
    Scene {
        paths: visible
            .paths
            .iter()
            .filter_map(|index| scene.paths.get(*index))
            .map(|path| stratum_canvas::render::ScenePath {
                ops: PathData {
                    ops: path.ops.clone(),
                }
                .transformed(affine)
                .ops,
                stroke_width: path.stroke_width * zoom,
                clip: path.clip.map(transform),
                ..path.clone()
            })
            .collect(),
        texts: visible
            .texts
            .iter()
            .filter_map(|index| scene.texts.get(*index))
            .map(|text| {
                let origin = affine.apply(Point::new(text.x, text.y));
                stratum_canvas::render::SceneText {
                    x: origin.x,
                    y: origin.y,
                    width: text.width * zoom,
                    height: text.height * zoom,
                    font_size: text.font_size * zoom,
                    clip: text.clip.map(transform),
                    ..text.clone()
                }
            })
            .collect(),
        images: visible
            .images
            .iter()
            .filter_map(|index| scene.images.get(*index))
            .map(|image| {
                let origin = affine.apply(Point::new(image.x, image.y));
                stratum_canvas::render::SceneImage {
                    x: origin.x,
                    y: origin.y,
                    width: image.width * zoom,
                    height: image.height * zoom,
                    clip: image.clip.map(transform),
                    ..image.clone()
                }
            })
            .collect(),
        frames: visible
            .frames
            .iter()
            .filter_map(|index| scene.frames.get(*index))
            .map(|frame| stratum_canvas::render::SceneFrame {
                rect: transform(frame.rect),
                ..frame.clone()
            })
            .collect(),
        bounds: Some(Rect::new(0.0, 0.0, width, height)),
        background: scene.background.clone(),
        revision: scene.revision,
    }
}

/// Export a document as SVG through the canvas crate (fixture helper).
pub fn document_svg(doc: &BoardDocument) -> Result<String, String> {
    scene_svg_for_document(doc, &SceneOptions::default()).map_err(|error| error.to_string())
}
