//! Export and preview actions layered on [`CanvasState`].
//!
//! The native whiteboard panel calls these methods instead of talking to the
//! scene/raster modules directly: they build (and cache) the scene for the
//! current document revision, expose the per-frame visible set, and produce
//! SVG, PNG and WebP previews.
//!
//! Thumbnails use the shipped preview contract: a lossless WebP `data:` URL of
//! at most 300 px plus the theme it was rendered for, stored on the document
//! through [`BoardDocument::set_preview`]. [`CanvasState::refresh_thumbnail`]
//! is the save-time entry point; [`CanvasState::thumbnail`] only renders and
//! leaves the document untouched.
//!
//! All failures use [`pkm_core::PkmError`].

use pkm_core::PkmResult;

use crate::document::BoardDocument;
use crate::geometry::Rect;
use crate::render::raster::{self, THUMBNAIL_MAX_EDGE};
use crate::render::scene::{Scene, SceneOptions, VisibleSet};
use crate::render::svg;
use crate::theme::PreviewTheme;
use crate::tools::CanvasState;

/// Export options derived from `options`: exports never apply the gallery
/// theme adaptation.
fn export_options(options: &SceneOptions) -> SceneOptions {
    SceneOptions {
        adapt_theme: false,
        ..*options
    }
}

/// A rendered board preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasThumbnail {
    /// Lossless WebP `data:` URL.
    pub data_url: String,
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// Theme the preview was rendered for.
    pub theme: PreviewTheme,
}

impl CanvasState {
    /// Build or reuse the cached scene for the current document.
    ///
    /// The cache is keyed by the document revision and the options, so
    /// repeated calls after a mutation rebuild exactly once.
    ///
    /// # Errors
    ///
    /// Propagates [`pkm_core::PkmError::Canvas`] when the scene cannot be
    /// built.
    pub fn render_scene(&mut self, options: &SceneOptions) -> PkmResult<&Scene> {
        self.scene_cache.scene(&self.doc, options)
    }

    /// Indices of scene items intersecting the current viewport.
    ///
    /// `screen` supplies the viewport pixel size; its origin is ignored. The
    /// scene must have been built at least once (through [`Self::render_scene`]
    /// or an export method).
    ///
    /// # Errors
    ///
    /// Returns [`pkm_core::PkmError::Canvas`] when no scene is cached yet.
    pub fn visible(&mut self, screen: Rect) -> PkmResult<VisibleSet> {
        self.scene_cache.visible(&self.viewport, screen)
    }

    /// Export the current board as a deterministic SVG document.
    ///
    /// Exports always use the document's raw colours and canvas background;
    /// [`SceneOptions::adapt_theme`] is forced off regardless of the options
    /// passed in.
    ///
    /// # Errors
    ///
    /// Propagates scene-build errors.
    pub fn export_svg(&mut self, options: &SceneOptions) -> PkmResult<String> {
        let scene = self
            .scene_cache
            .scene(&self.doc, &export_options(options))?;
        Ok(svg::scene_svg(scene))
    }

    /// Export the current board as PNG.
    ///
    /// Exports always use the document's raw colours and canvas background;
    /// [`SceneOptions::adapt_theme`] is forced off regardless of the options
    /// passed in.
    ///
    /// # Errors
    ///
    /// Propagates scene-build and rasterisation errors.
    pub fn export_png(
        &mut self,
        options: &SceneOptions,
        max_edge: Option<u32>,
    ) -> PkmResult<Vec<u8>> {
        let scene = self
            .scene_cache
            .scene(&self.doc, &export_options(options))?;
        Ok(raster::render_scene(scene, max_edge)?.png)
    }

    /// Export the current board as a lossless WebP byte stream.
    ///
    /// Exports always use the document's raw colours and canvas background;
    /// [`SceneOptions::adapt_theme`] is forced off regardless of the options
    /// passed in.
    ///
    /// # Errors
    ///
    /// Propagates scene-build, rasterisation and encoding errors.
    pub fn export_webp(
        &mut self,
        options: &SceneOptions,
        max_edge: Option<u32>,
    ) -> PkmResult<Vec<u8>> {
        let scene = self
            .scene_cache
            .scene(&self.doc, &export_options(options))?;
        let image = raster::render_scene(scene, max_edge)?;
        raster::encode_webp(&image)
    }

    /// Render a preview thumbnail without modifying the document.
    ///
    /// # Errors
    ///
    /// Propagates scene-build, rasterisation and encoding errors.
    pub fn thumbnail(&self, theme: PreviewTheme) -> PkmResult<CanvasThumbnail> {
        let thumb = raster::thumbnail(&self.doc, theme)?;
        Ok(CanvasThumbnail {
            data_url: thumb.data_url,
            width: thumb.width,
            height: thumb.height,
            theme,
        })
    }

    /// Render a preview thumbnail and store it on the document.
    ///
    /// This is the save-time entry point: it writes `preview` and
    /// `previewTheme` (marking the document dirty) exactly like the React
    /// panel does.
    ///
    /// # Errors
    ///
    /// Propagates scene-build, rasterisation and encoding errors.
    pub fn refresh_thumbnail(&mut self, theme: PreviewTheme) -> PkmResult<CanvasThumbnail> {
        let thumb = self.thumbnail(theme)?;
        self.doc.set_preview(&thumb.data_url, theme);
        Ok(thumb)
    }

    /// The maximum thumbnail edge used by [`Self::thumbnail`].
    pub fn thumbnail_max_edge(&self) -> u32 {
        THUMBNAIL_MAX_EDGE
    }

    /// Read-only document access for export callers.
    pub fn export_document(&self) -> &BoardDocument {
        &self.doc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::sample_board;

    #[test]
    fn exports_through_the_state() {
        let mut state = CanvasState::new(sample_board());
        let options = SceneOptions::default();
        let svg = state.export_svg(&options).unwrap();
        assert!(svg.contains("<svg"));
        let png = state.export_png(&options, Some(300)).unwrap();
        assert_eq!(&png[..4], b"\x89PNG");
        let webp = state.export_webp(&options, Some(300)).unwrap();
        assert_eq!(&webp[..4], b"RIFF");
    }

    #[test]
    fn refresh_thumbnail_stores_the_preview() {
        let mut state = CanvasState::new(sample_board());
        let thumb = state.refresh_thumbnail(PreviewTheme::Dark).unwrap();
        assert_eq!(state.document().preview(), Some(thumb.data_url.as_str()));
        assert_eq!(state.document().preview_theme(), Some(PreviewTheme::Dark));
        assert!(state.document().is_dirty());
    }

    #[test]
    fn visible_requires_a_built_scene() {
        let mut state = CanvasState::new(sample_board());
        assert!(state.visible(Rect::new(0.0, 0.0, 10.0, 10.0)).is_err());
        state.render_scene(&SceneOptions::default()).unwrap();
        let set = state.visible(Rect::new(0.0, 0.0, 2000.0, 2000.0)).unwrap();
        assert!(!set.paths.is_empty());
    }
}
