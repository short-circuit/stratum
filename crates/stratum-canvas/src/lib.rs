//! Excalidraw-compatible whiteboard document model, tools and `.excalidraw`
//! round-trip for the Stratum canvas.
//!
//! The crate is the headless core behind the native whiteboard surface. It
//! mirrors the shipped Excalidraw file format without depending on
//! `stratum-app`, `tauri` or a windowing system:
//!
//! * [`element`] keeps every element's full JSON object so unknown fields
//!   survive a load/save cycle; [`element_kind`] maps the wire enums.
//! * [`document`], [`document_meta`] and [`files`] load and save boards with a
//!   byte-stable untouched path and a lossless rewrite path.
//! * [`library`] reads/writes both shipped `.excalidrawlib` shapes.
//! * [`geometry`] provides hit testing, marquee selection, snapping and resize
//!   handles; [`tools`] and [`history`] implement the canvas state machine and
//!   the bounded undo/redo journal.
//! * [`link`] parses `Board#Element` deep links and resolves them against a
//!   loaded document.
//! * [`text`] and [`theme`] supply deterministic text metrics and the shipped
//!   thumbnail theme adaptation.
//! * [`render`] builds the rough scene, the per-frame visible set, and the
//!   SVG/PNG/WebP preview and export output.
//! * [`fixture`] builds deterministic boards for tests and captures.

pub mod document;
pub mod document_meta;
pub mod element;
pub mod element_kind;
pub mod files;
pub mod fixture;
pub mod geometry;
pub mod history;
pub mod library;
pub mod link;
pub mod render;
pub mod text;
pub mod theme;
pub mod tools;

// Internal submodules split out of the documented files to keep every source
// file under the 500-line limit. Their items are re-exported from the
// documented module paths above.
mod element_content;
mod element_style;
mod geometry_snap;
mod tools_clipboard;
mod tools_drag;
mod tools_edit;
mod tools_style;
mod tools_text;

#[cfg(test)]
mod tools_tests;

pub use document::BoardDocument;
pub use document_meta::{DocumentMeta, DEFAULT_SOURCE, DEFAULT_VERSION};
pub use element::{
    seed_for, ArrowBinding, Binding, Element, Roundness, Style, DEFAULT_BACKGROUND_COLOR,
    DEFAULT_STROKE_COLOR,
};
pub use element_kind::{
    Arrowhead, ElementKind, FillStyle, FontFamily, StrokeSharpness, StrokeStyle, TextAlign,
    VerticalAlign,
};
pub use files::{data_url_mime, decode_data_url, empty_files, file_entry, set_file};
pub use geometry::{
    element_bounds, element_rect, hit_test, hit_test_all, marquee_select, resize_handle_rects,
    resize_rect, snap_rect, Affine, Point, Rect, SnapConfig, Viewport, RESIZE_HANDLES,
};
pub use history::{Delta, History, HISTORY_LIMIT};
pub use library::{Library, LibraryItem, LibraryShape};
pub use link::{decode_component, encode_component, BoardLink, DeepLink};
pub use text::{layout, line_height, measure, wrap, TextLayout};
pub use theme::{adapt_color_for_theme, invert_hex, luminance, theme_background, PreviewTheme};
pub use tools::{CanvasAction, CanvasState, Modifiers, PointerEvent, Tool};
pub use tools_clipboard::CLIPBOARD_TYPE;
pub use tools_style::{Align, Axis, StylePatch};
