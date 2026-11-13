//! Per-revision scene cache and the per-frame visibility query.
//!
//! Building a scene is expensive (rough sketch generation, text layout), so
//! [`SceneCache`] keeps the last scene keyed by the document revision and the
//! options used. [`SceneCache::visible`] is the 60 fps path: it tests each
//! cached item rectangle against the viewport's world rectangle and returns
//! indices, without allocating any path geometry.
//!
//! The revision signature lives in `scene.rs`; a cache hit requires both the
//! revision and the options to match.

use pkm_core::{PkmError, PkmResult};

use crate::document::BoardDocument;
use crate::geometry::{Rect, Viewport};
use crate::render::scene::{document_revision, path_bounds, visible_rect, Scene, SceneOptions};

/// Indices of scene items intersecting the visible world rectangle.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VisibleSet {
    /// Visible path indices.
    pub paths: Vec<usize>,
    /// Visible text indices.
    pub texts: Vec<usize>,
    /// Visible image indices.
    pub images: Vec<usize>,
    /// Visible frame indices.
    pub frames: Vec<usize>,
}

/// Scene cache keyed by document revision and scene options.
#[derive(Debug, Default)]
pub struct SceneCache {
    scene: Option<Scene>,
    revision: u64,
    options: Option<SceneOptions>,
    path_bounds: Vec<Rect>,
    text_bounds: Vec<Rect>,
    image_bounds: Vec<Rect>,
    frame_bounds: Vec<Rect>,
}

impl SceneCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build once per document revision; subsequent calls reuse the cached
    /// scene.
    ///
    /// # Errors
    ///
    /// Propagates scene build errors. When the cache is already populated for
    /// the same revision and options, no work is done.
    pub fn scene(&mut self, doc: &BoardDocument, options: &SceneOptions) -> PkmResult<&Scene> {
        let revision = document_revision(doc);
        let hit = self.scene.is_some()
            && self.revision == revision
            && self.options.as_ref() == Some(options);
        if !hit {
            let scene = Scene::build(doc, options)?;
            self.revision = revision;
            self.options = Some(*options);
            self.path_bounds = scene
                .paths
                .iter()
                .map(|path| path_bounds(&path.ops))
                .collect();
            self.text_bounds = scene
                .texts
                .iter()
                .map(|text| Rect::new(text.x, text.y, text.width, text.height))
                .collect();
            self.image_bounds = scene
                .images
                .iter()
                .map(|image| Rect::new(image.x, image.y, image.width, image.height))
                .collect();
            self.frame_bounds = scene
                .frames
                .iter()
                .map(|frame| {
                    let mut rect = frame.rect;
                    rect.y -= 24.0;
                    rect.h += 24.0;
                    rect
                })
                .collect();
            self.scene = Some(scene);
        }
        self.scene
            .as_ref()
            .ok_or_else(|| PkmError::Canvas("scene cache is empty".to_string()))
    }

    /// Indices of scene items intersecting the viewport's visible world
    /// rectangle.
    ///
    /// The `screen` rectangle supplies the viewport size; its origin is
    /// ignored. This is O(n) with a cheap rectangle test and allocates only the
    /// result vectors.
    ///
    /// # Errors
    ///
    /// Returns [`PkmError::Canvas`] when no scene has been built yet.
    pub fn visible(&mut self, viewport: &Viewport, screen: Rect) -> PkmResult<VisibleSet> {
        if self.scene.is_none() {
            return Err(PkmError::Canvas(
                "visible() called before scene()".to_string(),
            ));
        }
        let world = visible_rect(viewport, screen);
        let mut set = VisibleSet {
            paths: Vec::new(),
            texts: Vec::new(),
            images: Vec::new(),
            frames: Vec::new(),
        };
        collect(&self.path_bounds, &world, &mut set.paths);
        collect(&self.text_bounds, &world, &mut set.texts);
        collect(&self.image_bounds, &world, &mut set.images);
        collect(&self.frame_bounds, &world, &mut set.frames);
        Ok(set)
    }
}

/// Push indices whose bounds intersect `world`.
fn collect(bounds: &[Rect], world: &Rect, out: &mut Vec<usize>) {
    for (index, rect) in bounds.iter().enumerate() {
        if rect.intersects(world) {
            out.push(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::sample_board;
    use crate::theme::PreviewTheme;

    #[test]
    fn caches_by_revision_and_reuses() {
        let mut cache = SceneCache::new();
        let doc = sample_board();
        let options = SceneOptions::default();
        let revision = {
            let scene = cache.scene(&doc, &options).unwrap();
            scene.revision
        };
        let counts = {
            let scene = cache.scene(&doc, &options).unwrap();
            assert_eq!(scene.revision, revision);
            scene.counts()
        };
        assert!(counts.paths > 0);
        assert!(counts.texts > 0);
        assert!(counts.frames > 0);
    }

    #[test]
    fn visible_filters_by_viewport() {
        let mut cache = SceneCache::new();
        let doc = sample_board();
        let options = SceneOptions::default();
        cache.scene(&doc, &options).unwrap();
        let viewport = Viewport::default();
        let all = cache
            .visible(&viewport, Rect::new(0.0, 0.0, 2000.0, 2000.0))
            .unwrap();
        let empty = cache
            .visible(&viewport, Rect::new(0.0, 0.0, 1.0, 1.0))
            .unwrap();
        assert!(!all.paths.is_empty());
        assert!(empty.paths.len() < all.paths.len());
    }

    #[test]
    fn dark_theme_changes_background_when_adapting() {
        let doc = sample_board();
        let scene = Scene::build(
            &doc,
            &SceneOptions {
                theme: PreviewTheme::Dark,
                adapt_theme: true,
                ..SceneOptions::default()
            },
        )
        .unwrap();
        assert_eq!(scene.background, "#232329");
    }

    #[test]
    fn exports_keep_the_document_background_without_adaptation() {
        let doc = sample_board();
        let scene = Scene::build(
            &doc,
            &SceneOptions {
                theme: PreviewTheme::Dark,
                ..SceneOptions::default()
            },
        )
        .unwrap();
        assert_eq!(scene.background, doc.view_background_color());
    }
}
