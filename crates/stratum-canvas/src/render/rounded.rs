//! Local-coordinate geometry helpers for the rough generator.
//!
//! This module keeps the shape primitives that [`crate::render::rough`] feeds
//! to `roughr`: polyline extraction, loop detection, diamond corners and the
//! Excalidraw rounded rectangle/diamond path strings. All coordinates are
//! element-local; the caller applies the world affine afterwards.

use roughr::Point2D;

use crate::element::Element;

/// Corner radius fraction for proportional/legacy rounding.
const CORNER_RADIUS_FRACTION: f64 = 0.25;
/// Default adaptive corner radius in world units.
const ADAPTIVE_CORNER_RADIUS: f64 = 32.0;
/// Excalidraw's loop-closing tolerance for filled polylines.
const LOOP_TOLERANCE: f64 = 8.0;

/// Local polyline points, falling back to a diagonal when absent.
pub(crate) fn polyline(element: &Element) -> Vec<Point2D<f64>> {
    let points = element.points();
    if points.len() >= 2 {
        return points
            .iter()
            .map(|point| Point2D::new(point.x, point.y))
            .collect();
    }
    vec![
        Point2D::new(0.0, 0.0),
        Point2D::new(element.width(), element.height()),
    ]
}

/// Whether a polyline closes into a loop (Excalidraw's `isPathALoop`).
pub(crate) fn is_loop(points: &[Point2D<f64>]) -> bool {
    if points.len() < 3 {
        return false;
    }
    let first = points[0];
    let last = points[points.len() - 1];
    let dx = last.x - first.x;
    let dy = last.y - first.y;
    (dx * dx + dy * dy).sqrt() <= LOOP_TOLERANCE
}

/// World affine for an element: translate to its origin, then rotate around
/// its centre exactly like Excalidraw.
pub(crate) fn element_affine(element: &Element) -> crate::geometry::Affine {
    let x = element.x();
    let y = element.y();
    let angle = element.angle();
    if angle == 0.0 {
        return crate::geometry::Affine {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: x,
            f: y,
        };
    }
    let cx = x + element.width() / 2.0;
    let cy = y + element.height() / 2.0;
    let cos = angle.cos();
    let sin = angle.sin();
    crate::geometry::Affine {
        a: cos,
        b: sin,
        c: -sin,
        d: cos,
        e: cx + cos * (x - cx) - sin * (y - cy),
        f: cy + sin * (x - cx) + cos * (y - cy),
    }
}

/// Diamond corner points in local coordinates.
pub(crate) fn diamond_points(element: &Element) -> Vec<Point2D<f64>> {
    let w = element.width();
    let h = element.height();
    vec![
        Point2D::new(w / 2.0, 0.0),
        Point2D::new(w, h / 2.0),
        Point2D::new(w / 2.0, h),
        Point2D::new(0.0, h / 2.0),
    ]
}

/// Excalidraw corner radius for a rounded rectangle/diamond.
fn corner_radius(size: f64, element: &Element) -> f64 {
    let Some(roundness) = &element.style().roundness else {
        return 0.0;
    };
    match roundness.kind {
        1 | 2 => size * CORNER_RADIUS_FRACTION,
        3 => {
            let radius = roundness.value.unwrap_or(ADAPTIVE_CORNER_RADIUS);
            if size <= radius / CORNER_RADIUS_FRACTION {
                size * CORNER_RADIUS_FRACTION
            } else {
                radius
            }
        }
        _ => 0.0,
    }
}

/// Excalidraw rounded rectangle path in local coordinates.
pub(crate) fn rounded_rect_path(element: &Element) -> String {
    let w = element.width();
    let h = element.height();
    let r = corner_radius(w.min(h), element);
    format!(
        "M {r} 0 L {} 0 C {w} 0 {w} 0 {w} {r} L {w} {} C {w} {h} {w} {h} {} {h} L {r} {h} C 0 {h} 0 {h} 0 {} L 0 {r} C 0 0 0 0 {r} 0",
        w - r,
        h - r,
        w - r,
        h - r
    )
}

/// Excalidraw rounded diamond path in local coordinates.
pub(crate) fn rounded_diamond_path(element: &Element) -> String {
    let w = element.width();
    let h = element.height();
    let bx = corner_radius(w / 2.0, element);
    let by = corner_radius(h / 2.0, element);
    let (ax, ay) = (w / 2.0, 0.0);
    let (dx, dy) = (w, h / 2.0);
    let (lx, uy) = (w / 2.0, h);
    let (px, my) = (0.0, h / 2.0);
    format!(
        "M {} {} L {} {} C {dx} {dy} {dx} {dy} {} {} L {} {} C {lx} {uy} {lx} {uy} {} {} L {} {} C {px} {my} {px} {my} {} {} L {} {} C {ax} {ay} {ax} {ay} {} {}",
        ax + bx,
        ay + by,
        dx - bx,
        dy - by,
        dx - bx,
        dy + by,
        lx + bx,
        uy - by,
        lx - bx,
        uy - by,
        px + bx,
        my + by,
        px + bx,
        my - by,
        ax - bx,
        ay + by,
        ax + bx,
        ay + by
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element_kind::ElementKind;
    use crate::geometry::Point;

    #[test]
    fn loop_detection_matches_tolerance() {
        let closed = vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(10.0, 0.0),
            Point2D::new(10.0, 10.0),
            Point2D::new(1.0, 1.0),
        ];
        assert!(is_loop(&closed));
        let open = vec![
            Point2D::new(0.0, 0.0),
            Point2D::new(10.0, 0.0),
            Point2D::new(10.0, 10.0),
        ];
        assert!(!is_loop(&open));
    }

    #[test]
    fn rounded_rect_path_starts_at_radius() {
        let mut rect = Element::new(ElementKind::Rectangle, "r", 0.0, 0.0, 100.0, 60.0);
        rect.set_style(&crate::element::Style {
            roundness: Some(crate::element::Roundness {
                kind: 1,
                value: None,
            }),
            ..crate::element::Style::default()
        });
        let path = rounded_rect_path(&rect);
        assert!(path.starts_with("M 15 0"));
    }

    #[test]
    fn element_affine_rotates_around_centre() {
        let mut rect = Element::new(ElementKind::Rectangle, "r", 10.0, 20.0, 40.0, 20.0);
        rect.set_angle(std::f64::consts::FRAC_PI_2);
        let affine = element_affine(&rect);
        let origin = affine.apply(Point::new(0.0, 0.0));
        // 90° rotation about (30, 30) maps local (0, 0) to (40, 10).
        assert!((origin.x - 40.0).abs() < 1e-9);
        assert!((origin.y - 10.0).abs() < 1e-9);
    }
}
