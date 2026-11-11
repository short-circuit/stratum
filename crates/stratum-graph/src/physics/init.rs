//! Deterministic random source and d3-compatible node initialization.
//!
//! The shipped simulation uses d3-force-3d's linear congruential generator
//! (seed `1`) and its golden-angle spherical placement for nodes without a
//! stored position. Both are reproduced here so the sequential tick path is a
//! faithful port and the parallel path stays reproducible.

use crate::math::Vec3;

/// d3-force-3d's linear congruential generator parameters.
const LCG_A: u32 = 1_664_525;
const LCG_C: u32 = 1_013_904_223;

/// Deterministic 32-bit LCG matching `d3-force-3d/src/lcg.js`.
#[derive(Debug, Clone)]
pub struct Lcg {
    state: u32,
}

impl Lcg {
    /// Create a generator seeded with `seed` (the shipped default is `1`).
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    /// Advance the generator and return a value in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_mul(LCG_A).wrapping_add(LCG_C);
        f64::from(self.state) / 4_294_967_296.0
    }

    /// d3's `jiggle`: a tiny deterministic perturbation used to break exact
    /// coincidences.
    pub fn jiggle(&mut self) -> f64 {
        (self.next_f64() - 0.5) * 1e-6
    }
}

impl Default for Lcg {
    fn default() -> Self {
        Self::new(1)
    }
}

/// The golden-ratio roll angle used by d3's spherical initialization.
pub const INITIAL_ANGLE_ROLL: f64 = std::f64::consts::PI * (3.0 - 2.236_067_977_499_79);
/// The Markov-irrational yaw angle used by d3's spherical initialization.
pub const INITIAL_ANGLE_YAW: f64 = std::f64::consts::PI * 20.0 / (9.0 + 14.866_068_747_318_506);

/// d3-force-3d's spherical seed position for node `index`.
///
/// Only used when a node has no stored/derived position; the graph panel always
/// supplies the shipped hash seed, so this is a compatibility fallback.
pub fn spherical_position(index: usize) -> Vec3 {
    let radius = 10.0 * (0.5 + index as f64).cbrt();
    let roll = index as f64 * INITIAL_ANGLE_ROLL;
    let yaw = index as f64 * INITIAL_ANGLE_YAW;
    Vec3::new(
        (radius * roll.sin() * yaw.cos()) as f32,
        (radius * roll.cos()) as f32,
        (radius * roll.sin() * yaw.sin()) as f32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lcg_matches_reference_sequence() {
        let mut rng = Lcg::new(1);
        let first = rng.next_f64();
        // (1664525 * 1 + 1013904223) / 2^32
        assert!((first - 0.236_455_525_271_594_52).abs() < 1e-12);
        let second = rng.next_f64();
        assert!((second - 0.369_270_673_720_166_1).abs() < 1e-12);
    }

    #[test]
    fn jiggle_is_tiny_and_deterministic() {
        let mut a = Lcg::new(1);
        let mut b = Lcg::new(1);
        let j = a.jiggle();
        assert_eq!(j, b.jiggle());
        assert!(j.abs() <= 5e-7);
    }

    #[test]
    fn spherical_positions_are_finite_and_unique() {
        let a = spherical_position(0);
        let b = spherical_position(1);
        assert!(a.x.is_finite() && a.y.is_finite() && a.z.is_finite());
        assert_ne!(a, b);
    }
}
