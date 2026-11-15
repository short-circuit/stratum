//! Snapping and resize handles for the whiteboard canvas.
//!
//! Split out of `geometry.rs` to keep every source file under the 500-line
//! limit. The items are re-exported from [`crate::geometry`] so the documented
//! `geometry::snap_rect` / `geometry::resize_rect` paths stay valid.

use crate::geometry::{Point, Rect};

/// Number of resize handles (four corners and four edges).
pub const RESIZE_HANDLES: usize = 8;
/// Visual size of a resize handle in world units.
const RESIZE_HANDLE_SIZE: f64 = 10.0;

/// Snapping configuration for moving elements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapConfig {
    /// Optional grid size; when set, positions snap to the grid.
    pub grid: Option<f64>,
    /// Whether to snap to other elements' edges and centres.
    pub object: bool,
    /// Object-snap threshold in screen pixels.
    pub threshold: f64,
}

impl Default for SnapConfig {
    fn default() -> Self {
        Self {
            grid: None,
            object: true,
            threshold: 8.0,
        }
    }
}

/// Snap a moving rectangle to the grid or to other elements' edges/centres.
///
/// Returns the delta to add to the rectangle's position. When `cfg.grid` is
/// set the grid takes precedence and the delta rounds the top-left corner onto
/// the grid. Otherwise the nearest object edge/centre within
/// `cfg.threshold / zoom` world units wins. Callers disable snapping for the
/// Alt modifier.
pub fn snap_rect(moving: &Rect, others: &[Rect], cfg: &SnapConfig, zoom: f64) -> Point {
    if let Some(grid) = cfg.grid {
        if grid > 0.0 {
            return Point {
                x: (moving.x / grid).round() * grid - moving.x,
                y: (moving.y / grid).round() * grid - moving.y,
            };
        }
    }
    if !cfg.object || cfg.threshold <= 0.0 {
        return Point::new(0.0, 0.0);
    }
    let limit = cfg.threshold / zoom.max(0.1);
    let moving_x = [moving.x, moving.center().x, moving.right()];
    let moving_y = [moving.y, moving.center().y, moving.bottom()];
    let mut best_x: Option<f64> = None;
    let mut best_y: Option<f64> = None;
    for other in others {
        let target_x = [other.x, other.center().x, other.right()];
        let target_y = [other.y, other.center().y, other.bottom()];
        for from in moving_x {
            for to in target_x {
                let delta = to - from;
                if delta.abs() <= limit
                    && best_x.map(|best| delta.abs() < best.abs()).unwrap_or(true)
                {
                    best_x = Some(delta);
                }
            }
        }
        for from in moving_y {
            for to in target_y {
                let delta = to - from;
                if delta.abs() <= limit
                    && best_y.map(|best| delta.abs() < best.abs()).unwrap_or(true)
                {
                    best_y = Some(delta);
                }
            }
        }
    }
    Point {
        x: best_x.unwrap_or(0.0),
        y: best_y.unwrap_or(0.0),
    }
}

/// Resize handle rectangles in NW, N, NE, E, SE, S, SW, W order.
pub fn resize_handle_rects(r: &Rect) -> [Rect; RESIZE_HANDLES] {
    let rect = r.normalized();
    let half = RESIZE_HANDLE_SIZE / 2.0;
    let center = rect.center();
    let positions = [
        (rect.x, rect.y),
        (center.x, rect.y),
        (rect.right(), rect.y),
        (rect.right(), center.y),
        (rect.right(), rect.bottom()),
        (center.x, rect.bottom()),
        (rect.x, rect.bottom()),
        (rect.x, center.y),
    ];
    positions.map(|(x, y)| Rect::new(x - half, y - half, RESIZE_HANDLE_SIZE, RESIZE_HANDLE_SIZE))
}

/// Apply a resize handle drag to a rectangle.
///
/// `handle` indexes [`resize_handle_rects`] (0..7). Negative resulting sizes
/// are normalised by flipping the rectangle. Out-of-range handles return the
/// input unchanged.
pub fn resize_rect(r: &Rect, handle: usize, delta: Point) -> Rect {
    let mut x = r.x;
    let mut y = r.y;
    let mut w = r.w;
    let mut h = r.h;
    match handle {
        0 => {
            x += delta.x;
            y += delta.y;
            w -= delta.x;
            h -= delta.y;
        }
        1 => {
            y += delta.y;
            h -= delta.y;
        }
        2 => {
            y += delta.y;
            w += delta.x;
            h -= delta.y;
        }
        3 => {
            w += delta.x;
        }
        4 => {
            w += delta.x;
            h += delta.y;
        }
        5 => {
            h += delta.y;
        }
        6 => {
            x += delta.x;
            w -= delta.x;
            h += delta.y;
        }
        7 => {
            x += delta.x;
            w -= delta.x;
        }
        _ => return *r,
    }
    Rect::new(x, y, w, h).normalized()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_snap_rounds_to_grid() {
        let cfg = SnapConfig {
            grid: Some(20.0),
            ..SnapConfig::default()
        };
        let delta = snap_rect(&Rect::new(7.0, 33.0, 10.0, 10.0), &[], &cfg, 1.0);
        assert_eq!(delta, Point::new(-7.0, 7.0));
    }

    #[test]
    fn resize_normalises_negative_sizes() {
        let rect = resize_rect(
            &Rect::new(0.0, 0.0, 100.0, 100.0),
            7,
            Point::new(150.0, 0.0),
        );
        assert_eq!(rect, Rect::new(100.0, 0.0, 50.0, 100.0));
    }

    #[test]
    fn object_snap_aligns_edges() {
        let cfg = SnapConfig::default();
        let delta = snap_rect(
            &Rect::new(102.0, 0.0, 10.0, 10.0),
            &[Rect::new(100.0, 0.0, 50.0, 50.0)],
            &cfg,
            1.0,
        );
        assert_eq!(delta.x, -2.0);
    }
}
