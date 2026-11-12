//! Arrowhead geometry for Excalidraw arrow elements.
//!
//! This ports the arrowhead branch of the shipped Excalidraw renderer (the
//! `ii`/`G1` functions in `chunk-K2UTITRG.js`). The head length, half-angle
//! and fill rules match that code:
//!
//! | Head | Length | Half-angle | Paint |
//! |------|--------|------------|-------|
//! | `arrow` | 25 | 20° | stroke only |
//! | `triangle` | 15 | 25° | stroke + solid fill |
//! | `diamond` | 12 | 25° | stroke + solid fill |
//! | `circle` | `dist + strokeWidth - 2` radius | — | stroke + solid fill |
//! | `bar` | 15 | 90° | stroke only |
//!
//! Head length is clamped to half the last segment (a quarter for diamonds),
//! exactly like Excalidraw. The head sketch is drawn with roughness
//! `min(1, element roughness)` and no dash pattern (dotted arrows use the
//! dotted head dash instead), mirroring the shipped options.
//!
//! Geometry is computed from the element's stored points rather than the
//! roughened sketch endpoints. Excalidraw anchors heads on the roughened
//! path, but the model points are the deterministic input and differ by at
//! most a couple of world units.
//!
//! Heads inherit the element's opacity and theme adaptation happens later in
//! the scene builder, so the colours returned here are the element's original
//! wire colours.

use roughr::core::{FillStyle as RoughFillStyle, Options};
use roughr::generator::Generator;
use roughr::Point2D;

use crate::element::Element;
use crate::element_kind::{Arrowhead, ElementKind, StrokeStyle};
use crate::geometry::Point;
use crate::render::color::{is_transparent, parse_color, to_srgba};
use crate::render::rough::{drawable_paths, sketch_options, Sketch};
use crate::render::scene::ScenePath;

/// Arrow head length in world units.
const ARROW_LENGTH: f64 = 25.0;
/// Triangle head length in world units.
const TRIANGLE_LENGTH: f64 = 15.0;
/// Bar head length in world units.
const BAR_LENGTH: f64 = 15.0;
/// Diamond head length in world units.
const DIAMOND_LENGTH: f64 = 12.0;
/// Arrow head half-angle in degrees.
const ARROW_ANGLE: f64 = 20.0;
/// Triangle/diamond head half-angle in degrees.
const TRIANGLE_ANGLE: f64 = 25.0;
/// Bar head half-angle in degrees.
const BAR_ANGLE: f64 = 90.0;

/// Head length clamp fractions (Excalidraw: `I * 0.5`, diamonds `I * 0.25`).
const LENGTH_FRACTION: f64 = 0.5;
/// Diamond head length clamp fraction.
const DIAMOND_LENGTH_FRACTION: f64 = 0.25;

/// One arrowhead's local geometry.
struct HeadShape {
    /// Tip point (the arrow endpoint).
    tip: Point,
    /// First barb (back rotated by `-angle`).
    barb_a: Point,
    /// Second barb (back rotated by `+angle`).
    barb_b: Point,
    /// Diamond rear vertex at `tip - unit * 2 * length`.
    rear: Point,
    /// Circle radius.
    radius: f64,
}

/// Build the rough paths for every arrowhead attached to `element`.
///
/// Returns an empty vector for non-arrows, arrows with fewer than two points
/// and arrows without `startArrowhead`/`endArrowhead`.
pub fn arrowhead_paths(element: &Element) -> Vec<ScenePath> {
    if element.kind() != ElementKind::Arrow {
        return Vec::new();
    }
    let points = element.points();
    if points.len() < 2 {
        return Vec::new();
    }
    let Ok(sketch) = sketch_options(element, false) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(head) = element.start_arrowhead() {
        if let Some(shape) = head_shape(&points, true, head, sketch.stroke_width) {
            out.extend(head_paths(element, &sketch, head, &shape));
        }
    }
    if let Some(head) = element.end_arrowhead() {
        if let Some(shape) = head_shape(&points, false, head, sketch.stroke_width) {
            out.extend(head_paths(element, &sketch, head, &shape));
        }
    }
    out
}

/// Compute the local geometry for one arrowhead.
fn head_shape(
    points: &[Point],
    start: bool,
    head: Arrowhead,
    stroke_width: f64,
) -> Option<HeadShape> {
    let (tip, adjacent) = if start {
        (points[0], points[1])
    } else {
        (points[points.len() - 1], points[points.len() - 2])
    };
    let unit = unit_vector(adjacent, tip)?;
    let distance = ((tip.x - adjacent.x).powi(2) + (tip.y - adjacent.y).powi(2)).sqrt();
    let fraction = if matches!(head, Arrowhead::Diamond) {
        DIAMOND_LENGTH_FRACTION
    } else {
        LENGTH_FRACTION
    };
    let length = head_length(head).min(distance * fraction).max(0.0);
    let back = Point::new(tip.x - unit.x * length, tip.y - unit.y * length);
    let angle = head_angle(head).to_radians();
    let barb_a = rotate(back, tip, -angle);
    let barb_b = rotate(back, tip, angle);
    let rear = Point::new(tip.x - unit.x * length * 2.0, tip.y - unit.y * length * 2.0);
    let radius = (length + stroke_width - 2.0).max(0.0);
    Some(HeadShape {
        tip,
        barb_a,
        barb_b,
        rear,
        radius,
    })
}

/// Generate the rough drawable for one head and map it to scene paths.
fn head_paths(
    element: &Element,
    sketch: &Sketch,
    head: Arrowhead,
    shape: &HeadShape,
) -> Vec<ScenePath> {
    let dotted = element.style().stroke_style == StrokeStyle::Dotted;
    let raw_width = element.style().stroke_width;
    let dash = if dotted && matches!(head, Arrowhead::Arrow | Arrowhead::Bar) {
        Some(vec![1.5, 5.0 + raw_width])
    } else {
        None
    };
    let fill = if matches!(head, Arrowhead::Arrow | Arrowhead::Bar) {
        None
    } else {
        head_fill(element)
    };
    let options = head_options(sketch, fill, dash.clone());
    let generator = Generator::default();
    let drawable = match head {
        Arrowhead::Arrow | Arrowhead::Bar => generator.linear_path(
            &[
                to_point2d(shape.barb_a),
                to_point2d(shape.tip),
                to_point2d(shape.barb_b),
            ],
            false,
            &Some(options),
        ),
        Arrowhead::Triangle => generator.polygon(
            &[
                to_point2d(shape.tip),
                to_point2d(shape.barb_a),
                to_point2d(shape.barb_b),
                to_point2d(shape.tip),
            ],
            &Some(options),
        ),
        Arrowhead::Diamond => generator.polygon(
            &[
                to_point2d(shape.tip),
                to_point2d(shape.barb_a),
                to_point2d(shape.rear),
                to_point2d(shape.barb_b),
                to_point2d(shape.tip),
            ],
            &Some(options),
        ),
        Arrowhead::Circle => generator.ellipse(
            shape.tip.x as f32,
            shape.tip.y as f32,
            shape.radius as f32 * 2.0,
            shape.radius as f32 * 2.0,
            &Some(options),
        ),
    };
    let head_sketch = Sketch {
        options: sketch.options.clone(),
        stroke_color: sketch.stroke_color.clone(),
        fill_color: if fill.is_some() {
            sketch.stroke_color.clone()
        } else {
            None
        },
        stroke_width: sketch.stroke_width,
        dash,
        opacity: sketch.opacity,
    };
    drawable_paths(element, &head_sketch, &drawable)
        .into_iter()
        .filter(|path| !path.ops.is_empty())
        .collect()
}

/// Clone the element options with the arrowhead overrides.
fn head_options(sketch: &Sketch, fill: Option<roughr::Srgba>, dash: Option<Vec<f64>>) -> Options {
    let mut options = sketch.options.clone();
    options.roughness = Some(options.roughness.unwrap_or(1.0).min(1.0));
    options.stroke_line_dash = dash;
    options.fill = fill;
    options.fill_style = fill.map(|_| RoughFillStyle::Solid);
    options
}

/// Solid head fill for triangle/diamond/circle, using the stroke colour.
fn head_fill(element: &Element) -> Option<roughr::Srgba> {
    let style = element.style();
    if is_transparent(&style.stroke_color) {
        return None;
    }
    parse_color(&style.stroke_color).map(|rgba| to_srgba(rgba, style.opacity))
}

/// Head length in world units.
fn head_length(head: Arrowhead) -> f64 {
    match head {
        Arrowhead::Arrow => ARROW_LENGTH,
        Arrowhead::Diamond => DIAMOND_LENGTH,
        Arrowhead::Triangle | Arrowhead::Circle => TRIANGLE_LENGTH,
        Arrowhead::Bar => BAR_LENGTH,
    }
}

/// Head half-angle in degrees.
fn head_angle(head: Arrowhead) -> f64 {
    match head {
        Arrowhead::Arrow => ARROW_ANGLE,
        Arrowhead::Bar => BAR_ANGLE,
        Arrowhead::Triangle | Arrowhead::Diamond | Arrowhead::Circle => TRIANGLE_ANGLE,
    }
}

/// Unit vector from `from` to `to`, or `None` when they coincide.
fn unit_vector(from: Point, to: Point) -> Option<Point> {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let length = (dx * dx + dy * dy).sqrt();
    if length <= f64::EPSILON || !length.is_finite() {
        return None;
    }
    Some(Point::new(dx / length, dy / length))
}

/// Rotate `point` around `center` by `radians`.
fn rotate(point: Point, center: Point, radians: f64) -> Point {
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    let (sin, cos) = radians.sin_cos();
    Point::new(
        center.x + dx * cos - dy * sin,
        center.y + dx * sin + dy * cos,
    )
}

/// Convert a local point into roughr's `f32` point type.
fn to_point2d(point: Point) -> Point2D<f32> {
    Point2D::new(point.x as f32, point.y as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::scene::path_bounds;

    /// Arrow element from `(0,0)` to `(100,0)` with the given head.
    fn arrow(head: Arrowhead, start: bool) -> Element {
        let mut element = Element::new(ElementKind::Arrow, "a", 10.0, 20.0, 100.0, 0.0);
        element.set_points(&[Point::new(0.0, 0.0), Point::new(100.0, 0.0)]);
        if start {
            element.set_start_arrowhead(Some(head));
        } else {
            element.set_end_arrowhead(Some(head));
        }
        element
    }

    #[test]
    fn non_arrows_and_headless_arrows_have_no_paths() {
        let rect = Element::new(ElementKind::Rectangle, "r", 0.0, 0.0, 10.0, 10.0);
        assert!(arrowhead_paths(&rect).is_empty());
        let mut line = Element::new(ElementKind::Arrow, "a", 0.0, 0.0, 10.0, 0.0);
        line.set_points(&[Point::new(0.0, 0.0), Point::new(10.0, 0.0)]);
        assert!(arrowhead_paths(&line).is_empty());
    }

    #[test]
    fn arrow_head_is_a_v_at_the_tip() {
        let paths = arrowhead_paths(&arrow(Arrowhead::Arrow, false));
        assert!(!paths.is_empty());
        assert!(paths.iter().all(|path| path.stroke.is_some()));
        let bounds = paths
            .iter()
            .map(|path| path_bounds(&path.ops))
            .reduce(|a, b| a.union(&b))
            .unwrap();
        // Tip is at world (110, 20); barbs point back and outwards.
        assert!(bounds.right() >= 109.0);
        assert!(bounds.h - 0.0 >= 8.0);
    }

    #[test]
    fn start_head_points_the_other_way() {
        let paths = arrowhead_paths(&arrow(Arrowhead::Arrow, true));
        assert!(!paths.is_empty());
        let bounds = paths
            .iter()
            .map(|path| path_bounds(&path.ops))
            .reduce(|a, b| a.union(&b))
            .unwrap();
        assert!(bounds.x <= 11.0);
    }

    #[test]
    fn triangle_and_diamond_fill_with_the_stroke_colour() {
        for head in [Arrowhead::Triangle, Arrowhead::Diamond] {
            let paths = arrowhead_paths(&arrow(head, false));
            assert!(paths.iter().any(|path| path.fill.is_some()));
        }
    }

    #[test]
    fn circle_head_is_an_ellipse_at_the_tip() {
        let paths = arrowhead_paths(&arrow(Arrowhead::Circle, false));
        assert!(!paths.is_empty());
        let bounds = paths
            .iter()
            .map(|path| path_bounds(&path.ops))
            .reduce(|a, b| a.union(&b))
            .unwrap();
        assert!(bounds.x <= 110.0 && bounds.right() >= 110.0);
        assert!(bounds.y <= 20.0 && bounds.bottom() >= 20.0);
    }

    #[test]
    fn short_segments_clamp_the_head_length() {
        let mut element = Element::new(ElementKind::Arrow, "a", 0.0, 0.0, 8.0, 0.0);
        element.set_points(&[Point::new(0.0, 0.0), Point::new(8.0, 0.0)]);
        element.set_end_arrowhead(Some(Arrowhead::Triangle));
        let paths = arrowhead_paths(&element);
        let bounds = paths
            .iter()
            .map(|path| path_bounds(&path.ops))
            .reduce(|a, b| a.union(&b))
            .unwrap();
        assert!(bounds.x >= -1.0);
    }

    #[test]
    fn rotation_moves_the_head_with_the_element() {
        let mut element = arrow(Arrowhead::Arrow, false);
        element.set_angle(std::f64::consts::FRAC_PI_2);
        let paths = arrowhead_paths(&element);
        let bounds = paths
            .iter()
            .map(|path| path_bounds(&path.ops))
            .reduce(|a, b| a.union(&b))
            .unwrap();
        assert!(bounds.bottom() > 60.0);
        assert!(bounds.right() <= 110.0);
    }
}
