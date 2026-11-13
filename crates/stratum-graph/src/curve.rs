//! Deterministic link curvature helpers.
//!
//! The shipped graph panel hands `linkCurvature` to three.js, which bends each
//! link into a quadratic bezier. The Rust port needs the same visual result
//! without an RNG: [`link_perpendicular`] derives a stable unit vector orthogonal
//! to the link from its index, and [`link_curve_points`] tessellates the curve
//! into straight segments for both the GPU and software renderers.

use crate::math::Vec3;

/// Number of straight segments used to tessellate a curved link.
pub const LINK_CURVE_SEGMENTS: usize = 8;

/// Deterministic unit vector orthogonal to a link direction.
///
/// The candidate is derived from the link index with a splitmix-style integer
/// hash and projected onto the plane perpendicular to the link. No RNG state is
/// shared between links, which keeps every fixture and every render
/// byte-identical across builds.
pub fn link_perpendicular(direction: Vec3, link_index: usize) -> Vec3 {
    let dir = direction.normalized();
    if dir.length_squared() < f32::EPSILON {
        return Vec3::UP;
    }
    let candidate = index_vector(link_index);
    let mut perpendicular = candidate.sub(dir.scale(candidate.dot(dir)));
    if perpendicular.length_squared() < 1e-6 {
        perpendicular = dir.cross(Vec3::UP);
    }
    if perpendicular.length_squared() < 1e-6 {
        perpendicular = dir.cross(Vec3::new(0.0, 0.0, 1.0));
    }
    perpendicular.normalized()
}

/// Quadratic bezier control point for a link.
///
/// `curvature` is the signed world-space offset from the straight midpoint, so a
/// zero setting returns the midpoint itself (a straight link).
pub fn link_curve_control(start: Vec3, end: Vec3, curvature: f32, link_index: usize) -> Vec3 {
    let midpoint = start.lerp(end, 0.5);
    if curvature == 0.0 {
        return midpoint;
    }
    midpoint.add(link_perpendicular(end.sub(start), link_index).scale(curvature))
}

/// Tessellate a link into [`LINK_CURVE_SEGMENTS`] straight segments.
///
/// The returned points are `segments + 1` samples of the quadratic bezier
/// `(start, control, end)`. A zero curvature yields collinear points, i.e. a
/// straight segment. The fixed-size return avoids a heap allocation per link on
/// the render hot path.
pub fn link_curve_points(
    start: Vec3,
    end: Vec3,
    curvature: f32,
    link_index: usize,
) -> [Vec3; LINK_CURVE_SEGMENTS + 1] {
    let control = link_curve_control(start, end, curvature, link_index);
    let mut points = [Vec3::ZERO; LINK_CURVE_SEGMENTS + 1];
    for (step, point) in points.iter_mut().enumerate() {
        let t = step as f32 / LINK_CURVE_SEGMENTS as f32;
        // De Casteljau split: lerp twice to evaluate the quadratic bezier.
        let first = start.lerp(control, t);
        let second = control.lerp(end, t);
        *point = first.lerp(second, t);
    }
    points
}

/// Index-seeded pseudo-random vector in `[-1, 1]^3`.
fn index_vector(index: usize) -> Vec3 {
    let mut state = (index as u64)
        .wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .wrapping_add(0xbf58_476d_1ce4_e5b9);
    let mut components = [0.0_f32; 3];
    for component in &mut components {
        state ^= state >> 30;
        state = state.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        state ^= state >> 27;
        state = state.wrapping_mul(0x94d0_49bb_1331_11eb);
        state ^= state >> 31;
        let unit = ((state >> 40) & 0x00ff_ffff) as f32 / 16_777_215.0;
        *component = unit * 2.0 - 1.0;
    }
    Vec3::new(components[0], components[1], components[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perpendicular_is_unit_and_orthogonal() {
        let direction = Vec3::new(12.0, -5.0, 3.0);
        for index in 0..16 {
            let perpendicular = link_perpendicular(direction, index);
            assert!((perpendicular.length() - 1.0).abs() < 1e-3);
            assert!(perpendicular.dot(direction.normalized()).abs() < 1e-3);
        }
    }

    #[test]
    fn zero_curvature_returns_the_midpoint() {
        let start = Vec3::new(-10.0, 2.0, 0.0);
        let end = Vec3::new(10.0, 2.0, 0.0);
        let midpoint = start.lerp(end, 0.5);
        assert_eq!(link_curve_control(start, end, 0.0, 3), midpoint);
        let points = link_curve_points(start, end, 0.0, 3);
        assert_eq!(points.len(), LINK_CURVE_SEGMENTS + 1);
        assert!(points.iter().all(|point| point.y == midpoint.y));
    }
}
