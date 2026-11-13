//! Geometry primitives, hit testing and marquee selection for the whiteboard
//! canvas.
//!
//! Coordinates are world units unless the name says otherwise. Screen
//! coordinates are produced by [`Viewport::world_to_screen`]. All helpers are
//! pure so they can be unit-tested without a window.
//!
//! Hit testing walks the element list topmost-first (reverse file order) and
//! treats linear/freedraw elements as polylines, frames as selectable from
//! their title bar or interior, and everything else by its bounding box.
//!
//! Snapping and resize handles live in `geometry_snap.rs` to keep files small;
//! they are re-exported here so the documented `geometry::snap_rect` and
//! `geometry::resize_rect` paths stay valid.

use crate::element::Element;
use crate::element_kind::ElementKind;

pub use crate::geometry_snap::{
    resize_handle_rects, resize_rect, snap_rect, SnapConfig, RESIZE_HANDLES,
};

use std::ops::{Add, Sub};

/// A world or screen point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate.
    pub y: f64,
}

impl Point {
    /// Build a point.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Euclidean length.
    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

impl Add for Point {
    type Output = Point;

    /// Vector addition.
    fn add(self, other: Point) -> Point {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Point {
    type Output = Point;

    /// Vector subtraction.
    fn sub(self, other: Point) -> Point {
        Point {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

/// An axis-aligned rectangle in world units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width (may be negative before normalisation).
    pub w: f64,
    /// Height (may be negative before normalisation).
    pub h: f64,
}

impl Rect {
    /// Build a rectangle.
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    /// Right edge.
    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    /// Bottom edge.
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }

    /// Centre point.
    pub fn center(&self) -> Point {
        Point {
            x: self.x + self.w / 2.0,
            y: self.y + self.h / 2.0,
        }
    }

    /// Whether the point lies inside the rectangle.
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x <= self.right() && p.y >= self.y && p.y <= self.bottom()
    }

    /// Whether two rectangles overlap.
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x <= other.right()
            && other.x <= self.right()
            && self.y <= other.bottom()
            && other.y <= self.bottom()
    }

    /// Grow the rectangle by `d` on every side.
    pub fn inflate(&self, d: f64) -> Rect {
        Rect {
            x: self.x - d,
            y: self.y - d,
            w: self.w + 2.0 * d,
            h: self.h + 2.0 * d,
        }
    }

    /// Smallest rectangle covering both inputs.
    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Rect {
            x,
            y,
            w: right - x,
            h: bottom - y,
        }
    }

    /// Return a rectangle with positive width/height.
    pub fn normalized(&self) -> Rect {
        let x = if self.w < 0.0 {
            self.x + self.w
        } else {
            self.x
        };
        let y = if self.h < 0.0 {
            self.y + self.h
        } else {
            self.y
        };
        Rect {
            x,
            y,
            w: self.w.abs(),
            h: self.h.abs(),
        }
    }
}

/// Screen/world transform for the canvas viewport (`screen = world * zoom +
/// offset`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// Screen x offset.
    pub x: f64,
    /// Screen y offset.
    pub y: f64,
    /// Zoom factor.
    pub zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}

impl Viewport {
    /// Transform a world point into screen space.
    pub fn world_to_screen(&self, p: Point) -> Point {
        Point {
            x: p.x * self.zoom + self.x,
            y: p.y * self.zoom + self.y,
        }
    }

    /// Transform a screen point into world space.
    pub fn screen_to_world(&self, p: Point) -> Point {
        Point {
            x: (p.x - self.x) / self.zoom,
            y: (p.y - self.y) / self.zoom,
        }
    }

    /// The world rectangle covered by a screen of the given size.
    pub fn visible_world_rect(&self, width: f64, height: f64) -> Rect {
        let top_left = self.screen_to_world(Point::new(0.0, 0.0));
        Rect {
            x: top_left.x,
            y: top_left.y,
            w: width / self.zoom,
            h: height / self.zoom,
        }
    }
}

/// 2D affine transform, used by renderers to place elements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine {
    /// Scale/rotation component.
    pub a: f64,
    /// Shear component.
    pub b: f64,
    /// Shear component.
    pub c: f64,
    /// Scale/rotation component.
    pub d: f64,
    /// Translation x.
    pub e: f64,
    /// Translation y.
    pub f: f64,
}

impl Affine {
    /// Identity transform.
    pub fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Transform matching a viewport.
    pub fn from_viewport(v: &Viewport) -> Self {
        Self {
            a: v.zoom,
            b: 0.0,
            c: 0.0,
            d: v.zoom,
            e: v.x,
            f: v.y,
        }
    }

    /// Apply the transform to a point.
    pub fn apply(&self, p: Point) -> Point {
        Point {
            x: self.a * p.x + self.c * p.y + self.e,
            y: self.b * p.x + self.d * p.y + self.f,
        }
    }

    /// Invert the transform when the determinant is non-zero.
    pub fn invert(&self) -> Option<Self> {
        let det = self.a * self.d - self.b * self.c;
        if det.abs() < f64::EPSILON {
            return None;
        }
        let inv = 1.0 / det;
        Some(Self {
            a: self.d * inv,
            b: -self.b * inv,
            c: -self.c * inv,
            d: self.a * inv,
            e: (self.c * self.f - self.d * self.e) * inv,
            f: (self.b * self.e - self.a * self.f) * inv,
        })
    }
}

/// Height of a frame title bar in world units.
const FRAME_TITLE_HEIGHT: f64 = 24.0;
/// Maximum width of the frame title hit area.
const FRAME_TITLE_WIDTH: f64 = 240.0;

/// Bounding rectangle of an element as stored in its JSON.
pub fn element_rect(e: &Element) -> Rect {
    Rect::new(e.x(), e.y(), e.width(), e.height())
}

/// Bounding rectangle padded by half the stroke width.
pub fn element_bounds(e: &Element) -> Rect {
    element_rect(e).inflate(e.style().stroke_width / 2.0)
}

/// Frame title bar hit area (above the frame's top-left corner).
fn frame_title_rect(frame: &Rect) -> Rect {
    Rect::new(
        frame.x,
        frame.y - FRAME_TITLE_HEIGHT,
        frame.w.min(FRAME_TITLE_WIDTH),
        FRAME_TITLE_HEIGHT,
    )
}

/// Distance from `p` to the polyline described by `element`.
fn polyline_distance(element: &Element, p: Point) -> f64 {
    let points = element.points();
    if points.is_empty() {
        return f64::INFINITY;
    }
    let origin = Point::new(element.x(), element.y());
    let world: Vec<Point> = points.iter().map(|point| origin + *point).collect();
    if world.len() == 1 {
        return (world[0] - p).length();
    }
    let mut best = f64::INFINITY;
    for window in world.windows(2) {
        best = best.min(point_segment_distance(p, window[0], window[1]));
    }
    best
}

/// Distance from a point to a line segment.
fn point_segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let length_sq = ab.x * ab.x + ab.y * ab.y;
    if length_sq <= f64::EPSILON {
        return (p - a).length();
    }
    let ap = p - a;
    let t = ((ap.x * ab.x + ap.y * ab.y) / length_sq).clamp(0.0, 1.0);
    let closest = Point::new(a.x + ab.x * t, a.y + ab.y * t);
    (p - closest).length()
}

/// Whether the element is hit by the world point at the given zoom.
fn hits(element: &Element, p: Point, zoom: f64) -> bool {
    match element.kind() {
        ElementKind::Frame => {
            let rect = element_rect(element);
            frame_title_rect(&rect).contains(p) || rect.contains(p)
        }
        kind if kind.is_linear() => {
            let tolerance = element.style().stroke_width.max(6.0) / zoom.max(0.1);
            polyline_distance(element, p) <= tolerance
        }
        _ => element_rect(element).contains(p),
    }
}

/// Hit test topmost-first.
///
/// Returns the id of the first visible element under `p`, or `None`. Linear
/// and freedraw elements use stroke distance; frames are selectable from their
/// title bar or their interior.
pub fn hit_test(elements: &[Element], p: Point, zoom: f64) -> Option<String> {
    elements
        .iter()
        .rev()
        .filter(|element| !element.is_deleted())
        .find(|element| hits(element, p, zoom))
        .map(|element| element.id().to_string())
}

/// Every visible element under `p`, topmost-first.
pub fn hit_test_all(elements: &[Element], p: Point, zoom: f64) -> Vec<String> {
    elements
        .iter()
        .rev()
        .filter(|element| !element.is_deleted())
        .filter(|element| hits(element, p, zoom))
        .map(|element| element.id().to_string())
        .collect()
}

/// Ids of visible elements intersecting the marquee rectangle.
pub fn marquee_select(elements: &[Element], r: Rect) -> Vec<String> {
    let marquee = r.normalized();
    elements
        .iter()
        .filter(|element| !element.is_deleted())
        .filter(|element| element_rect(element).intersects(&marquee))
        .map(|element| element.id().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element_kind::ElementKind;

    #[test]
    fn viewport_round_trips() {
        let vp = Viewport {
            x: 40.0,
            y: -20.0,
            zoom: 2.0,
        };
        let p = Point::new(10.0, 15.0);
        let back = vp.screen_to_world(vp.world_to_screen(p));
        assert!((back.x - p.x).abs() < 1e-9);
        assert!((back.y - p.y).abs() < 1e-9);
    }

    #[test]
    fn linear_hit_uses_stroke_distance() {
        let mut arrow = Element::new(ElementKind::Arrow, "a", 0.0, 0.0, 100.0, 0.0);
        arrow.set_points(&[Point::new(0.0, 0.0), Point::new(100.0, 0.0)]);
        assert_eq!(
            hit_test(&[arrow], Point::new(50.0, 3.0), 1.0).as_deref(),
            Some("a")
        );
    }
}
