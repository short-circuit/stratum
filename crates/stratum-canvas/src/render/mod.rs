//! Canvas rendering: rough scene generation, previews and exports.
//!
//! The module turns a [`crate::document::BoardDocument`] into renderer-ready
//! geometry and serialised output:
//!
//! * [`scene`] builds the z-ordered [`Scene`] (rough paths, text, images,
//!   frames) and caches it per document revision; [`scene::SceneCache::visible`]
//!   is the per-frame visibility query.
//! * [`rough`] ports Excalidraw's `generateRoughOptions` to `roughr`, including
//!   arrowheads ([`rough_arrow`]), freedraw polylines and the roughness clamp.
//! * [`svg_path`] is the structural path parser/serialiser shared by the rough
//!   generator and the SVG writer.
//! * [`svg`] writes deterministic SVG 1.1; [`raster`] renders that SVG to
//!   PNG/WebP with the embedded Inter face and no system fonts.
//! * [`actions`] exposes the [`crate::tools::CanvasState`] export/preview
//!   methods used by the native panel.
//! * [`perf`] carries the frame timer and perf report used by `canvas_perf`
//!   and the 5k/500-element gates.
//!
//! [`color`] and [`fonts`] are internal helpers (wire colour parsing and the
//! bundled font mapping); [`rounded`] holds the local shape primitives for the
//! rough generator. Scene colours are the document's raw values; only the
//! gallery preview opts into the shipped theme adaptation through
//! [`scene::SceneOptions::adapt_theme`], so exports match Excalidraw's own
//! `exportToSvg` output.

pub mod actions;
pub mod color;
pub mod fonts;
pub mod perf;
pub mod raster;
pub mod rough;
pub mod rough_arrow;
pub mod scene;
pub mod svg;
pub mod svg_path;

mod rounded;
mod scene_build;
mod scene_cache;

pub use actions::CanvasThumbnail;
pub use raster::{
    encode_webp, export_png, export_webp, ink_coverage, render_scene, thumbnail, webp_data_url,
    RenderedImage, Thumbnail, MAX_RASTER_EDGE, THUMBNAIL_MAX_EDGE,
};
pub use scene::{
    Scene, SceneCache, SceneCounts, SceneFrame, SceneImage, SceneOptions, ScenePath, SceneText,
    VisibleSet,
};
pub use svg::{export_svg, scene_svg};
