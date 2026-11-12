//! One open board: document editing, autosave, export and deep-link focus.
//!
//! [`EditorSession`] is the headless half of the whiteboard editor. It parses
//! the `.excalidraw` file through [`stratum_canvas::BoardDocument`], routes
//! every mutation through [`stratum_canvas::CanvasState`], and serializes
//! byte-identically while the document is untouched.

use std::time::Instant;

use stratum_canvas::render::{render_scene, SceneOptions, VisibleSet};
use stratum_canvas::{
    element_rect, Affine, Align, Axis, BoardDocument, BoardLink, CanvasAction, CanvasState,
    DeepLink, Library, LibraryItem, Point, PreviewTheme, Rect, StylePatch,
};

use crate::provider::PanelData;

use super::scene::screen_scene;
use super::{ExportFormat, SaveState, AUTOSAVE_DELAY, EXPORT_SCALE, FRAME_HEIGHT, FRAME_WIDTH};

/// A rendered viewport frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewportFrame {
    /// PNG bytes for the gpui image element.
    pub png: Vec<u8>,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Number of visible elements in the frame.
    pub visible: usize,
}

/// One open board with its autosave and export state.
pub struct EditorSession {
    name: String,
    original: String,
    state: CanvasState,
    theme: PreviewTheme,
    save_state: SaveState,
    last_edit: Option<Instant>,
    saved_at: Option<Instant>,
    thumbnail: Option<String>,
}

impl EditorSession {
    /// Parse a board and start an editing session.
    pub fn open(name: &str, content: &str, theme: PreviewTheme) -> Result<Self, String> {
        let doc = BoardDocument::load(content.as_bytes()).map_err(|error| error.to_string())?;
        Ok(Self {
            name: name.to_string(),
            original: content.to_string(),
            state: CanvasState::new(doc),
            theme,
            save_state: SaveState::Clean,
            last_edit: None,
            saved_at: None,
            thumbnail: None,
        })
    }

    /// Start an empty session (used by tests and the create flow).
    pub fn empty(name: &str, theme: PreviewTheme) -> Result<Self, String> {
        let mut doc = BoardDocument::empty();
        let content = doc.save_string().map_err(|error| error.to_string())?;
        Self::open(name, &content, theme)
    }

    /// The board name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The editing state.
    pub fn state(&self) -> &CanvasState {
        &self.state
    }

    /// Mutable editing state. Callers that mutate the document must call
    /// [`Self::note_edit`] afterwards so autosave is scheduled.
    pub fn state_mut(&mut self) -> &mut CanvasState {
        &mut self.state
    }

    /// The current save state.
    pub fn save_state(&self) -> &SaveState {
        &self.save_state
    }

    /// The stored preview data URL, when a save has rendered one.
    pub fn thumbnail(&self) -> Option<&str> {
        self.thumbnail.as_deref()
    }

    /// When the last successful save completed.
    pub fn saved_at(&self) -> Option<Instant> {
        self.saved_at
    }

    /// Mark an edit at `now` and schedule autosave.
    pub fn note_edit(&mut self, now: Instant) {
        if !self.state.dirty() {
            return;
        }
        self.last_edit = Some(now);
        self.save_state = SaveState::Pending;
    }

    /// Whether the 800 ms autosave deadline has elapsed.
    pub fn save_due(&self, now: Instant) -> bool {
        matches!(self.save_state, SaveState::Pending)
            && self
                .last_edit
                .is_some_and(|last| now.saturating_duration_since(last) >= AUTOSAVE_DELAY)
    }

    /// Serialize the board, returning the original bytes when untouched.
    pub fn serialized(&mut self) -> Result<String, String> {
        if !self.state.dirty() {
            return Ok(self.original.clone());
        }
        self.state
            .document_mut()
            .save_string()
            .map_err(|error| error.to_string())
    }

    /// Flush the board to the provider, refreshing its preview first.
    ///
    /// Returns `Ok(false)` when the board is untouched, so an unmodified open
    /// never rewrites the file.
    pub fn flush(&mut self, provider: &dyn PanelData, now: Instant) -> Result<bool, String> {
        if !self.state.dirty() {
            self.save_state = SaveState::Clean;
            return Ok(false);
        }
        let thumb = self
            .state
            .refresh_thumbnail(self.theme)
            .map_err(|error| error.to_string())?;
        let content = self.serialized()?;
        match provider.save_whiteboard(&self.name, &content) {
            Ok(()) => {
                self.original = content;
                self.thumbnail = Some(thumb.data_url);
                self.saved_at = Some(now);
                self.last_edit = None;
                self.save_state = SaveState::Saved;
                Ok(true)
            }
            Err(error) => {
                self.save_state = SaveState::Failed(error.clone());
                Err(error)
            }
        }
    }

    /// Run autosave when the deadline elapsed.
    pub fn autosave(&mut self, provider: &dyn PanelData, now: Instant) -> Result<bool, String> {
        if !self.save_due(now) {
            return Ok(false);
        }
        self.flush(provider, now)
    }

    /// Export the board through the provider.
    pub fn export(
        &mut self,
        provider: &dyn PanelData,
        format: ExportFormat,
    ) -> Result<String, String> {
        let options = SceneOptions::default();
        let (bytes, extension) = match format {
            ExportFormat::Svg => (
                self.state
                    .export_svg(&options)
                    .map_err(|error| error.to_string())?
                    .into_bytes(),
                "svg",
            ),
            ExportFormat::Png => (self.png_bytes(&options)?, "png"),
        };
        provider.write_canvas_export(&self.name, extension, bytes)
    }

    /// Render the board as PNG at twice the scene's world-unit size.
    ///
    /// The canvas crate's `export_png` caps the output at one pixel per world
    /// unit, so the scene is scaled by [`EXPORT_SCALE`] before rasterizing.
    /// This matches the shipped React export, which renders PNGs at scale 2.
    pub fn png_bytes(&mut self, options: &SceneOptions) -> Result<Vec<u8>, String> {
        let scene = self
            .state
            .render_scene(options)
            .map_err(|error| error.to_string())?;
        let bounds = scene
            .bounds
            .ok_or_else(|| "whiteboard has no raster extent".to_string())?;
        let visible = VisibleSet {
            paths: (0..scene.paths.len()).collect(),
            texts: (0..scene.texts.len()).collect(),
            images: (0..scene.images.len()).collect(),
            frames: (0..scene.frames.len()).collect(),
        };
        let scale = EXPORT_SCALE;
        let affine = Affine {
            a: scale,
            b: 0.0,
            c: 0.0,
            d: scale,
            e: -bounds.x * scale,
            f: -bounds.y * scale,
        };
        let screen = screen_scene(
            scene,
            &visible,
            &affine,
            scale,
            bounds.w * scale,
            bounds.h * scale,
        );
        render_scene(&screen, None)
            .map(|rendered| rendered.png)
            .map_err(|error| error.to_string())
    }

    /// Build the `Board#Element` link for the selection or a specific element.
    pub fn copy_link(&self, element: Option<&str>) -> String {
        let link = BoardLink {
            board: self.name.clone(),
            element: element.map(str::to_string),
        };
        format!("/whiteboards?{}", link.to_query())
    }

    /// Resolve an element id or frame name against the open document, focus it
    /// and return the resolved link.
    pub fn focus_element(&mut self, element: &str) -> Result<DeepLink, String> {
        let link = BoardLink {
            board: self.name.clone(),
            element: Some(element.to_string()),
        };
        let resolved =
            DeepLink::resolve(&link, self.state.document()).map_err(|error| error.to_string())?;
        if let Some(id) = resolved
            .element_id
            .clone()
            .or_else(|| resolved.frame_id.clone())
        {
            self.state.select_ids(std::slice::from_ref(&id));
            if let Some(rect) = self.state.document().element(&id).map(element_rect) {
                let viewport = *self.state.viewport();
                let center = Rect::new(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0, 0.0, 0.0);
                let mut next = viewport;
                next.x = FRAME_WIDTH / 2.0 - center.x * next.zoom;
                next.y = FRAME_HEIGHT / 2.0 - center.y * next.zoom;
                self.state.set_viewport(next);
            }
        }
        Ok(resolved)
    }

    /// Zoom the viewport around the frame centre without touching the board.
    ///
    /// Zooming is a view-only change: it never schedules an autosave.
    pub fn zoom(&mut self, factor: f64, _now: Instant) {
        let centre = Point::new(FRAME_WIDTH / 2.0, FRAME_HEIGHT / 2.0);
        self.state.zoom_at(centre, factor);
    }

    /// Copy the current selection as Excalidraw clipboard JSON.
    pub fn copy_selection(&self) -> String {
        self.state.copy_selection()
    }

    /// Paste clipboard JSON, scheduling autosave on success.
    pub fn paste(&mut self, payload: &str, now: Instant) -> Result<CanvasAction, String> {
        let action = self
            .state
            .paste(payload)
            .map_err(|error| error.to_string())?;
        self.note_edit(now);
        Ok(action)
    }

    /// Apply a style patch to the selection.
    pub fn set_style(&mut self, patch: &StylePatch, now: Instant) -> CanvasAction {
        let action = self.state.set_selection_style(patch);
        self.note_edit(now);
        action
    }

    /// Align the selection.
    pub fn align(&mut self, align: Align, now: Instant) -> CanvasAction {
        let action = self.state.align_selection(align);
        self.note_edit(now);
        action
    }

    /// Distribute the selection.
    pub fn distribute(&mut self, axis: Axis, now: Instant) -> CanvasAction {
        let action = self.state.distribute_selection(axis);
        self.note_edit(now);
        action
    }

    /// Delete the selection.
    pub fn delete_selection(&mut self, now: Instant) -> CanvasAction {
        let action = self.state.delete_selection();
        self.note_edit(now);
        action
    }

    /// Duplicate the selection.
    pub fn duplicate_selection(&mut self, now: Instant) -> CanvasAction {
        let action = self.state.duplicate_selection();
        self.note_edit(now);
        action
    }

    /// Undo the last document mutation.
    pub fn undo(&mut self, now: Instant) -> CanvasAction {
        let action = self.state.undo();
        self.note_edit(now);
        action
    }

    /// Redo the last undone mutation.
    pub fn redo(&mut self, now: Instant) -> CanvasAction {
        let action = self.state.redo();
        self.note_edit(now);
        action
    }

    /// Insert a library item at a world position.
    pub fn insert_library_item(
        &mut self,
        item: &LibraryItem,
        x: f64,
        y: f64,
        now: Instant,
    ) -> Result<Vec<String>, String> {
        let ids = item
            .insert_into(self.state.document_mut(), x, y)
            .map_err(|error| error.to_string())?;
        self.note_edit(now);
        Ok(ids)
    }

    /// Save the current selection as a new library item.
    pub fn save_selection_to_library(
        &self,
        provider: &dyn PanelData,
        item_name: &str,
    ) -> Result<LibraryItem, String> {
        let elements = self
            .state
            .document()
            .visible_elements()
            .filter(|element| self.state.selection().iter().any(|id| id == element.id()))
            .map(|element| element.to_json())
            .collect::<Vec<_>>();
        if elements.is_empty() {
            return Err("no elements selected".to_string());
        }
        let value = serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "status": "published",
            "created": 0,
            "name": item_name,
            "elements": elements,
        });
        let item = LibraryItem::from_json(value).map_err(|error| error.to_string())?;
        let mut library = Library::empty();
        for row in provider.whiteboard_library()? {
            if let Some(existing) = row.to_item() {
                library.push(existing);
            }
        }
        library.push(item.clone());
        let bytes = library.save().map_err(|error| error.to_string())?;
        let json = String::from_utf8(bytes).map_err(|error| error.to_string())?;
        provider.save_whiteboard_library(&json)?;
        Ok(item)
    }

    /// Visible scene item indices for the current viewport.
    pub fn visible(&mut self, width: f64, height: f64) -> Result<VisibleSet, String> {
        let options = SceneOptions::default();
        self.state
            .render_scene(&options)
            .map_err(|error| error.to_string())?;
        self.state
            .visible(Rect::new(0.0, 0.0, width, height))
            .map_err(|error| error.to_string())
    }

    /// Rasterize the visible scene into a viewport-sized PNG.
    ///
    /// The scene is built once per document revision by the canvas
    /// [`stratum_canvas::render::SceneCache`]; pan/zoom only rebuilds the small
    /// screen-space frame from the cached items, so a 5k-element board stays
    /// responsive.
    pub fn viewport_frame(&mut self, width: f64, height: f64) -> Result<ViewportFrame, String> {
        let options = SceneOptions::default();
        let affine = Affine::from_viewport(self.state.viewport());
        let zoom = self.state.viewport().zoom;
        self.state
            .render_scene(&options)
            .map_err(|error| error.to_string())?;
        let visible = self
            .state
            .visible(Rect::new(0.0, 0.0, width, height))
            .map_err(|error| error.to_string())?;
        let scene = self
            .state
            .render_scene(&options)
            .map_err(|error| error.to_string())?;
        let screen = screen_scene(scene, &visible, &affine, zoom, width, height);
        let rendered = render_scene(&screen, None).map_err(|error| error.to_string())?;
        let visible_count =
            visible.paths.len() + visible.texts.len() + visible.images.len() + visible.frames.len();
        Ok(ViewportFrame {
            png: rendered.png,
            width: rendered.width,
            height: rendered.height,
            visible: visible_count,
        })
    }

    /// Export the board as SVG without touching the vault (used by tests).
    pub fn svg(&mut self) -> Result<String, String> {
        self.state
            .export_svg(&SceneOptions::default())
            .map_err(|error| error.to_string())
    }

    /// Serialize the document for a byte-identity check without mutating it.
    pub fn original(&self) -> &str {
        &self.original
    }

    /// Whether the document has unsaved edits.
    pub fn is_dirty(&self) -> bool {
        self.state.dirty()
    }
}
