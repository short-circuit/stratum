//! Canvas tool state machine: tools, pointer dispatch, selection and the
//! shared canvas state.
//!
//! [`CanvasState`] owns a [`BoardDocument`] and turns pointer events into
//! [`CanvasAction`]s. Drag handling lives in `tools_drag.rs`, editing
//! operations in `tools_edit.rs` and text editing in `tools_text.rs` so every
//! source file stays under the 500-line limit.
//!
//! Tool behaviour mirrors the Excalidraw editor where practical; deviations
//! are documented on the individual methods:
//!
//! * shape tools are single-shot (they return to [`Tool::Select`] after
//!   creation) because the lock-toggle is not modelled;
//! * delete and eraser mark elements `isDeleted` (Excalidraw soft-delete)
//!   instead of dropping their entries;
//! * arrows are not auto-bound to shapes on creation; fixtures and library
//!   inserts carry explicit bindings.

use crate::document::BoardDocument;
use crate::element::Element;
use crate::element_kind::{Arrowhead, ElementKind};
use crate::geometry::{hit_test_all, Point, Rect, SnapConfig, Viewport};
use crate::history::History;
use crate::render::SceneCache;

/// Active canvas tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// Selection and marquee.
    Select,
    /// Viewport panning.
    Hand,
    /// Rectangle creation.
    Rectangle,
    /// Diamond creation.
    Diamond,
    /// Ellipse creation.
    Ellipse,
    /// Arrow creation.
    Arrow,
    /// Line creation.
    Line,
    /// Pencil stroke.
    Freedraw,
    /// Text creation/editing.
    Text,
    /// Eraser.
    Eraser,
    /// Frame creation.
    Frame,
}

impl Tool {
    /// Element kind created by a shape tool.
    fn shape_kind(self) -> Option<ElementKind> {
        match self {
            Self::Rectangle => Some(ElementKind::Rectangle),
            Self::Diamond => Some(ElementKind::Diamond),
            Self::Ellipse => Some(ElementKind::Ellipse),
            _ => None,
        }
    }
}

/// Keyboard modifiers for a pointer event.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    /// Shift (square shapes, additive selection, toggle selection).
    pub shift: bool,
    /// Alt (duplicate-drag, disable snapping).
    pub alt: bool,
    /// Ctrl (reserved for future multi-select behaviour).
    pub ctrl: bool,
}

/// A pointer event in both world and screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerEvent {
    /// World position.
    pub point: Point,
    /// Screen position.
    pub screen: Point,
    /// Modifier state.
    pub modifiers: Modifiers,
}

/// Result of a canvas interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasAction {
    /// Nothing observable happened.
    None,
    /// The selection changed.
    SelectionChanged,
    /// The document changed.
    DocumentChanged,
    /// The active tool changed.
    ToolChanged,
    /// Text editing started for the given element id.
    TextEditing(String),
}

/// In-progress drag state.
#[derive(Debug, Clone)]
pub(crate) enum Drag {
    None,
    Pan {
        last: Point,
    },
    Marquee {
        start: Point,
        additive: bool,
    },
    Move {
        start: Point,
        origins: Vec<(String, f64, f64)>,
        original_rect: Rect,
    },
    Resize {
        id: String,
        handle: usize,
        start: Point,
        original: Rect,
    },
    CreateShape {
        id: String,
        start: Point,
        kind: ElementKind,
    },
    Freedraw {
        id: String,
        points: Vec<Point>,
    },
    FrameCreate {
        id: String,
        start: Point,
    },
    Erase {
        erased: Vec<String>,
    },
}

/// In-progress polyline (line/arrow) drawing.
#[derive(Debug, Clone)]
pub(crate) struct Polyline {
    pub(crate) id: String,
    pub(crate) points: Vec<Point>,
}

/// Text editing session state.
#[derive(Debug, Clone)]
pub(crate) struct TextEdit {
    pub(crate) id: String,
    pub(crate) created: bool,
    pub(crate) index: usize,
    pub(crate) original: Element,
    pub(crate) container: Option<String>,
}

/// Interactive canvas state.
pub struct CanvasState {
    pub(crate) doc: BoardDocument,
    pub(crate) tool: Tool,
    pub(crate) selection: Vec<String>,
    pub(crate) viewport: Viewport,
    pub(crate) snap: SnapConfig,
    pub(crate) history: History,
    pub(crate) drag: Drag,
    pub(crate) polyline: Option<Polyline>,
    pub(crate) text_edit: Option<TextEdit>,
    pub(crate) text_buffer: String,
    pub(crate) space_held: bool,
    pub(crate) pending_before: Option<Vec<Element>>,
    pub(crate) scene_cache: SceneCache,
}

impl CanvasState {
    /// Create a canvas over a document.
    pub fn new(document: BoardDocument) -> Self {
        Self {
            doc: document,
            tool: Tool::Select,
            selection: Vec::new(),
            viewport: Viewport::default(),
            snap: SnapConfig::default(),
            history: History::new(),
            drag: Drag::None,
            polyline: None,
            text_edit: None,
            text_buffer: String::new(),
            space_held: false,
            pending_before: None,
            scene_cache: SceneCache::new(),
        }
    }

    /// The underlying document.
    pub fn document(&self) -> &BoardDocument {
        &self.doc
    }

    /// Mutable document access. Marks the document dirty, snapshots the
    /// element list for the undo journal (finalized on the next mutation or
    /// undo/redo) and clears the redo stack.
    pub fn document_mut(&mut self) -> &mut BoardDocument {
        self.finalize_pending();
        self.pending_before = Some(self.doc.elements_snapshot());
        self.doc.mark_dirty();
        self.history.clear_redo();
        &mut self.doc
    }

    /// The active tool.
    pub fn tool(&self) -> Tool {
        self.tool
    }

    /// Switch tools. Finishing a polyline or text edit first keeps the
    /// document consistent.
    pub fn set_tool(&mut self, tool: Tool) {
        if self.polyline.is_some() {
            let _ = self.finish_linear();
        }
        if self.text_edit.is_some() {
            let _ = self.commit_text();
        }
        self.tool = tool;
    }

    /// Current selection ids.
    pub fn selection(&self) -> &[String] {
        &self.selection
    }

    /// The viewport transform.
    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    /// Replace the viewport transform.
    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.viewport = viewport;
    }

    /// Snapping configuration.
    pub fn snap(&self) -> &SnapConfig {
        &self.snap
    }

    /// Replace the snapping configuration.
    pub fn set_snap(&mut self, snap: SnapConfig) {
        self.snap = snap;
    }

    /// Space-drag panning flag (the shell sets this on key down/up).
    pub fn set_space_held(&mut self, held: bool) {
        self.space_held = held;
    }

    /// Pointer press.
    pub fn pointer_down(&mut self, ev: PointerEvent) -> CanvasAction {
        self.finalize_pending();
        if self.space_held || self.tool == Tool::Hand {
            self.drag = Drag::Pan { last: ev.screen };
            return CanvasAction::None;
        }
        if let Some(kind) = self.tool.shape_kind() {
            return self.begin_shape(kind, ev.point);
        }
        match self.tool {
            Tool::Select => self.select_down(ev),
            Tool::Arrow => self.begin_linear(ElementKind::Arrow, ev.point),
            Tool::Line => self.begin_linear(ElementKind::Line, ev.point),
            Tool::Freedraw => self.begin_freedraw(ev.point),
            Tool::Text => self.begin_text_at(ev.point),
            Tool::Eraser => self.begin_erase(ev.point),
            Tool::Frame => self.begin_frame(ev.point),
            Tool::Hand => CanvasAction::None,
            _ => CanvasAction::None,
        }
    }

    /// Double click: finish a polyline, or enter text editing.
    pub fn double_click(&mut self, ev: PointerEvent) -> CanvasAction {
        if self.polyline.is_some() {
            return self.finish_linear();
        }
        let Some(id) = crate::geometry::hit_test(self.doc.elements(), ev.point, self.viewport.zoom)
        else {
            return CanvasAction::None;
        };
        let Some(element) = self.doc.element(&id).cloned() else {
            return CanvasAction::None;
        };
        if element.kind() == ElementKind::Text {
            return self.edit_existing_text(&element);
        }
        if element.kind().is_bindable_container() {
            let bound_text = element
                .bound_elements()
                .into_iter()
                .find(|binding| binding.element_type == "text")
                .and_then(|binding| self.doc.element(&binding.id).cloned());
            if let Some(text) = bound_text {
                return self.edit_existing_text(&text);
            }
            return self.create_text(ev.point, Some(id));
        }
        CanvasAction::None
    }

    /// Escape: finish a polyline, cancel a text edit, or clear the selection.
    pub fn cancel(&mut self) -> CanvasAction {
        if self.polyline.is_some() {
            return self.finish_linear();
        }
        if self.text_edit.is_some() {
            return self.cancel_text();
        }
        if !matches!(self.drag, Drag::None) {
            self.drag = Drag::None;
            return CanvasAction::None;
        }
        self.clear_selection()
    }

    /// Select every visible element.
    pub fn select_all(&mut self) -> CanvasAction {
        self.selection = self
            .doc
            .visible_elements()
            .map(|element| element.id().to_string())
            .collect();
        CanvasAction::SelectionChanged
    }

    /// Replace the selection with the given element ids.
    ///
    /// Ids that are not present in the document are ignored, so a stale
    /// deep-link target cannot leave a phantom selection behind. Returns
    /// [`CanvasAction::SelectionChanged`] when the selection differs from the
    /// current one.
    pub fn select_ids(&mut self, ids: &[String]) -> CanvasAction {
        let next: Vec<String> = ids
            .iter()
            .filter(|id| self.doc.element(id).is_some())
            .cloned()
            .collect();
        if next == self.selection {
            return CanvasAction::None;
        }
        self.selection = next;
        CanvasAction::SelectionChanged
    }

    /// Clear the selection.
    pub fn clear_selection(&mut self) -> CanvasAction {
        if self.selection.is_empty() {
            return CanvasAction::None;
        }
        self.selection.clear();
        CanvasAction::SelectionChanged
    }

    /// Create a shape element and start a drag.
    fn begin_shape(&mut self, kind: ElementKind, point: Point) -> CanvasAction {
        let id = uuid::Uuid::new_v4().to_string();
        let element = Element::new(kind.clone(), &id, point.x, point.y, 0.0, 0.0);
        if self.doc.add_element(element).is_err() {
            return CanvasAction::None;
        }
        self.drag = Drag::CreateShape {
            id,
            start: point,
            kind,
        };
        CanvasAction::None
    }

    /// Begin or extend a polyline.
    fn begin_linear(&mut self, kind: ElementKind, point: Point) -> CanvasAction {
        if let Some(polyline) = &mut self.polyline {
            polyline.points.push(point);
            let id = polyline.id.clone();
            let points = polyline.points.clone();
            self.sync_absolute_points(&id, &points);
            return CanvasAction::DocumentChanged;
        }
        let id = uuid::Uuid::new_v4().to_string();
        let mut element = Element::new(kind.clone(), &id, point.x, point.y, 0.0, 0.0);
        element.set_points(&[Point::new(0.0, 0.0)]);
        if kind == ElementKind::Arrow {
            element.set_end_arrowhead(Some(Arrowhead::Arrow));
        }
        if self.doc.add_element(element).is_err() {
            return CanvasAction::None;
        }
        self.polyline = Some(Polyline {
            id,
            points: vec![point],
        });
        CanvasAction::DocumentChanged
    }

    /// Begin a freedraw stroke.
    fn begin_freedraw(&mut self, point: Point) -> CanvasAction {
        let id = uuid::Uuid::new_v4().to_string();
        let element = Element::new(ElementKind::Freedraw, &id, point.x, point.y, 0.0, 0.0);
        if self.doc.add_element(element).is_err() {
            return CanvasAction::None;
        }
        self.drag = Drag::Freedraw {
            id,
            points: vec![point],
        };
        CanvasAction::DocumentChanged
    }

    /// Begin an eraser stroke.
    fn begin_erase(&mut self, point: Point) -> CanvasAction {
        let hits = hit_test_all(self.doc.elements(), point, self.viewport.zoom);
        let mut erased = Vec::new();
        for id in hits {
            if let Some(element) = self.doc.element_mut(&id) {
                if !element.is_deleted() {
                    element.set_deleted(true);
                    erased.push(id);
                }
            }
        }
        self.drag = Drag::Erase {
            erased: erased.clone(),
        };
        if erased.is_empty() {
            CanvasAction::None
        } else {
            CanvasAction::DocumentChanged
        }
    }

    /// Begin a frame.
    fn begin_frame(&mut self, point: Point) -> CanvasAction {
        let id = uuid::Uuid::new_v4().to_string();
        let element = Element::new(ElementKind::Frame, &id, point.x, point.y, 0.0, 0.0);
        if self.doc.add_element(element).is_err() {
            return CanvasAction::None;
        }
        self.drag = Drag::FrameCreate { id, start: point };
        CanvasAction::None
    }

    /// Begin text creation at a world point.
    fn begin_text_at(&mut self, point: Point) -> CanvasAction {
        let container = hit_test_all(self.doc.elements(), point, self.viewport.zoom)
            .into_iter()
            .find(|id| {
                self.doc
                    .element(id)
                    .map(|element| {
                        element.kind().is_bindable_container()
                            && element.kind() != ElementKind::Text
                    })
                    .unwrap_or(false)
            });
        self.create_text(point, container)
    }
}
