//! Camera parity layer for the shipped graph panel.
//!
//! The React panel uses three.js `TrackballControls` plus a custom 500 ms cubic
//! focus tween and discrete WASD camera steps. This module reproduces those
//! behaviours without three.js: orbit/pan/dolly around a look-at target, the
//! shipped WASD basis math (speed `0.8`, horizontal normalization), and a
//! cancellable focus tween.

use crate::math::{Mat4, Vec3};
use std::time::{Duration, Instant};

/// The shipped focus tween duration.
pub const FOCUS_DURATION: Duration = Duration::from_millis(500);
/// The shipped discrete camera step speed.
pub const WASD_SPEED: f64 = 0.8;

/// Discrete camera movement keys (arrows map to the same directions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKey {
    /// `W` / arrow up: move toward the view direction.
    Forward,
    /// `S` / arrow down: move away from the view direction.
    Back,
    /// `A` / arrow left: strafe left.
    Left,
    /// `D` / arrow right: strafe right.
    Right,
}

/// A first-person/trackball camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// Camera position in world space.
    pub position: Vec3,
    /// Look-at target.
    pub target: Vec3,
    /// Vertical field of view in radians.
    pub fov_y: f32,
    /// Near plane.
    pub near: f32,
    /// Far plane.
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(Vec3::new(0.0, 0.0, 200.0), Vec3::ZERO)
    }
}

impl Camera {
    /// Create a camera with the shipped 45-degree field of view.
    pub fn new(position: Vec3, target: Vec3) -> Self {
        Self {
            position,
            target,
            fov_y: 45f32.to_radians(),
            near: 0.1,
            far: 5000.0,
        }
    }

    /// View matrix.
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at(self.position, self.target, Vec3::UP)
    }

    /// Projection matrix for the given aspect ratio.
    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective(self.fov_y, aspect, self.near, self.far)
    }

    /// Combined view-projection matrix for a viewport.
    pub fn view_projection(&self, width: u32, height: u32) -> Mat4 {
        let aspect = width as f32 / height.max(1) as f32;
        self.projection_matrix(aspect).multiply(self.view_matrix())
    }

    /// Project a world point to pixels, returning `(x, y, depth)`.
    pub fn project(&self, point: Vec3, width: u32, height: u32) -> Option<(f32, f32, f32)> {
        self.view_projection(width, height)
            .project_to_pixels(point, width, height)
    }

    /// Camera basis: `(right, up, back)`, where `back` points away from the
    /// target (three.js camera local +Z).
    pub fn basis(&self) -> (Vec3, Vec3, Vec3) {
        let forward = self.target.sub(self.position).normalized();
        let mut right = forward.cross(Vec3::UP).normalized();
        if right.length_squared() < f32::EPSILON {
            right = forward.cross(Vec3::new(0.0, 0.0, 1.0)).normalized();
        }
        let up = right.cross(forward).normalized();
        (right, up, forward.scale(-1.0))
    }

    /// Orbit around the target by azimuth/elevation deltas in radians.
    pub fn orbit(&mut self, delta_azimuth: f32, delta_elevation: f32) {
        let offset = self.position.sub(self.target);
        let radius = offset.length().max(1e-3);
        let mut azimuth = offset.x.atan2(offset.z);
        let mut elevation = (offset.y / radius).clamp(-1.0, 1.0).acos();
        azimuth += delta_azimuth;
        elevation = (elevation + delta_elevation).clamp(0.05, std::f32::consts::PI - 0.05);
        self.position = Vec3::new(
            self.target.x + radius * elevation.sin() * azimuth.sin(),
            self.target.y + radius * elevation.cos(),
            self.target.z + radius * elevation.sin() * azimuth.cos(),
        );
    }

    /// Pan the camera and target together.
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let (right, up, _) = self.basis();
        let delta = right.scale(delta_x).add(up.scale(delta_y));
        self.position = self.position.add(delta);
        self.target = self.target.add(delta);
    }

    /// Dolly toward or away from the target.
    pub fn zoom(&mut self, delta: f32) {
        let offset = self.position.sub(self.target);
        let distance = (offset.length() + delta).clamp(1.0, 100_000.0);
        self.position = self.target.add(offset.normalized().scale(distance));
    }

    /// Apply the shipped discrete WASD step.
    ///
    /// The shipped handler projects the camera's local X/Z axes onto the world
    /// XZ plane, normalizes the horizontal step, scales it by [`WASD_SPEED`], and
    /// moves both the camera and the controls target.
    pub fn step(&mut self, key: MoveKey) {
        let (right, _, back) = self.basis();
        let mut dx = 0.0_f64;
        let mut dz = 0.0_f64;
        match key {
            MoveKey::Forward => {
                dx -= f64::from(back.x);
                dz -= f64::from(back.z);
            }
            MoveKey::Back => {
                dx += f64::from(back.x);
                dz += f64::from(back.z);
            }
            MoveKey::Left => {
                dx -= f64::from(right.x);
                dz -= f64::from(right.z);
            }
            MoveKey::Right => {
                dx += f64::from(right.x);
                dz += f64::from(right.z);
            }
        }
        let length = (dx * dx + dz * dz).sqrt();
        if length > 0.0 {
            dx = dx / length * WASD_SPEED;
            dz = dz / length * WASD_SPEED;
        }
        self.position = Vec3::new(
            self.position.x + dx as f32,
            self.position.y,
            self.position.z + dz as f32,
        );
        self.target = Vec3::new(
            self.target.x + dx as f32,
            self.target.y,
            self.target.z + dz as f32,
        );
    }

    /// Start the shipped 500 ms cubic focus tween toward `node_position`.
    pub fn focus_tween(&self, node_position: Vec3, now: Instant) -> FocusTween {
        let dest_position = self.position.add(node_position.sub(self.target));
        FocusTween {
            started_at: now,
            start_position: self.position,
            start_target: self.target,
            dest_position,
            dest_target: node_position,
            duration: FOCUS_DURATION,
            canceled: false,
        }
    }

    /// Build a world-space ray for a screen pixel (used by CPU picking).
    pub fn ray_from_screen(&self, x: f32, y: f32, width: u32, height: u32) -> (Vec3, Vec3) {
        let aspect = width as f32 / height.max(1) as f32;
        let ndc_x = (x / width.max(1) as f32) * 2.0 - 1.0;
        let ndc_y = 1.0 - (y / height.max(1) as f32) * 2.0;
        let tan = (self.fov_y * 0.5).tan();
        let (right, up, back) = self.basis();
        let forward = back.scale(-1.0);
        let direction = forward
            .add(right.scale(ndc_x * tan * aspect))
            .add(up.scale(ndc_y * tan))
            .normalized();
        (self.position, direction)
    }
}

/// A cancellable 500 ms cubic focus tween.
#[derive(Debug, Clone, Copy)]
pub struct FocusTween {
    started_at: Instant,
    start_position: Vec3,
    start_target: Vec3,
    dest_position: Vec3,
    dest_target: Vec3,
    duration: Duration,
    canceled: bool,
}

impl FocusTween {
    /// Whether the tween is still active.
    pub fn is_active(&self) -> bool {
        !self.canceled
    }

    /// Cancel the tween (a controls interaction does this in the shipped panel).
    pub fn cancel(&mut self) {
        self.canceled = true;
    }

    /// Apply the tween at `now`. Returns `true` while the tween is still running.
    ///
    /// Tests can advance to the end by passing `started_at + duration` without
    /// sleeping.
    pub fn apply_at(&mut self, now: Instant, camera: &mut Camera) -> bool {
        if self.canceled {
            return false;
        }
        let elapsed = now.saturating_duration_since(self.started_at);
        let t = (elapsed.as_secs_f64() / self.duration.as_secs_f64()).min(1.0) as f32;
        let eased = 1.0 - (1.0 - t).powi(3);
        camera.position = self.start_position.lerp(self.dest_position, eased);
        camera.target = self.start_target.lerp(self.dest_target, eased);
        t < 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_tween_reaches_the_node_at_500ms() {
        let camera = Camera::new(Vec3::new(0.0, 0.0, 200.0), Vec3::ZERO);
        let now = Instant::now();
        let mut tween = camera.focus_tween(Vec3::new(10.0, 20.0, -30.0), now);
        let mut camera = camera;
        assert!(tween.apply_at(now, &mut camera));
        assert!(tween.apply_at(now + Duration::from_millis(250), &mut camera));
        assert!(!tween.apply_at(now + FOCUS_DURATION, &mut camera));
        assert_eq!(camera.target, Vec3::new(10.0, 20.0, -30.0));
        assert_eq!(camera.position, Vec3::new(10.0, 20.0, 170.0));
    }

    #[test]
    fn canceled_tween_does_not_move_the_camera() {
        let camera = Camera::new(Vec3::new(0.0, 0.0, 200.0), Vec3::ZERO);
        let now = Instant::now();
        let mut tween = camera.focus_tween(Vec3::new(100.0, 0.0, 0.0), now);
        tween.cancel();
        let mut camera = camera;
        assert!(!tween.apply_at(now + FOCUS_DURATION, &mut camera));
        assert_eq!(camera.position, Vec3::new(0.0, 0.0, 200.0));
    }

    #[test]
    fn wasd_step_matches_the_shipped_horizontal_normalization() {
        let mut camera = Camera::new(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO);
        camera.step(MoveKey::Forward);
        assert_eq!(camera.position, Vec3::new(0.0, 0.0, 9.2));
        assert_eq!(camera.target, Vec3::new(0.0, 0.0, -0.8));
        camera.step(MoveKey::Right);
        assert!((camera.position.x - 0.8).abs() < 1e-6);
        assert!((camera.target.x - 0.8).abs() < 1e-6);
    }

    #[test]
    fn orbit_preserves_distance_and_pan_moves_both() {
        let mut camera = Camera::new(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO);
        let before = camera.position.sub(camera.target).length();
        camera.orbit(0.5, 0.2);
        let after = camera.position.sub(camera.target).length();
        assert!((before - after).abs() < 1e-4);
        let position = camera.position;
        let target = camera.target;
        camera.pan(1.0, 2.0);
        assert!((camera.position.x - position.x).abs() > 1e-6);
        assert!((camera.target.y - target.y).abs() > 1e-6);
    }

    #[test]
    fn projection_and_rays_agree() {
        let camera = Camera::default();
        let (x, y, _) = camera
            .project(Vec3::ZERO, 1280, 720)
            .expect("origin visible");
        assert!((x - 640.0).abs() < 1e-3);
        assert!((y - 360.0).abs() < 1e-3);
        let (_, direction) = camera.ray_from_screen(640.0, 360.0, 1280, 720);
        let forward = camera.target.sub(camera.position).normalized();
        assert!(direction.dot(forward) > 0.99);
    }
}
