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
/// The shipped `TrackballControls.panSpeed`.
pub const PAN_SPEED: f64 = 0.15;
/// The shipped `TrackballControls.zoomSpeed`.
pub const ZOOM_SPEED: f64 = 1.2;
/// The shipped `3d-force-graph` camera distance-to-nodes factor.
///
/// The shipped panel places the camera at
/// `(0, 0, cbrt(node_count) * CAMERA_DISTANCE2NODES_FACTOR)` looking at the
/// origin (`CAMERA_DISTANCE2NODES_FACTOR = 170` in `dist/3d-force-graph.mjs`).
pub const CAMERA_DISTANCE2NODES_FACTOR: f32 = 170.0;

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
    /// When set, use an orthographic projection instead of perspective.
    ///
    /// This is a native-shell deviation from the shipped React panel: the
    /// shipped desktop graph always renders 3D perspective and the mobile
    /// variant switches to a separate 2D canvas. The native shell keeps one
    /// renderer and toggles an orthographic camera for its 2D view.
    pub orthographic: bool,
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
            orthographic: false,
        }
    }

    /// Camera placement matching the shipped `3d-force-graph` panel.
    ///
    /// The shipped panel does not fit the graph to its bounding box: it places
    /// the camera on the +Z axis at
    /// `(0, 0, cbrt(node_count) * CAMERA_DISTANCE2NODES_FACTOR)` looking at the
    /// origin, so the rendered graph keeps the shipped initial framing. An
    /// empty graph uses the base distance [`CAMERA_DISTANCE2NODES_FACTOR`].
    ///
    /// The far plane is raised to at least `distance * 4.0` so the whole graph
    /// stays inside the frustum, and the camera keeps the shipped perspective
    /// projection (`orthographic = false`).
    pub fn shipped_for_nodes(node_count: usize) -> Self {
        let distance = if node_count == 0 {
            CAMERA_DISTANCE2NODES_FACTOR
        } else {
            (node_count as f32).cbrt() * CAMERA_DISTANCE2NODES_FACTOR
        };
        let mut camera = Self::new(Vec3::new(0.0, 0.0, distance), Vec3::ZERO);
        camera.far = camera.far.max(distance * 4.0);
        camera.orthographic = false;
        camera
    }

    /// Enable or disable the orthographic projection.
    ///
    /// The orthographic frustum is sized from the current camera distance and
    /// the perspective field of view, so switching modes keeps the framed
    /// content at a similar on-screen scale.
    pub fn set_orthographic(&mut self, enabled: bool) {
        self.orthographic = enabled;
    }

    /// Whether the camera currently uses an orthographic projection.
    pub fn is_orthographic(&self) -> bool {
        self.orthographic
    }

    /// View matrix.
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at(self.position, self.target, Vec3::UP)
    }

    /// Projection matrix for the given aspect ratio.
    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        if self.orthographic {
            let (half_width, half_height) = self.ortho_extents(aspect);
            // The view matrix keeps the right-handed convention where points in
            // front of the camera have negative view-space z, while
            // `Mat4::orthographic` maps a positive near..far range. Passing the
            // negated range keeps the WebGPU depth mapping consistent with the
            // perspective path.
            Mat4::orthographic(
                -half_width,
                half_width,
                -half_height,
                half_height,
                -self.far,
                -self.near,
            )
        } else {
            Mat4::perspective(self.fov_y, aspect, self.near, self.far)
        }
    }

    /// Orthographic half extents `(width, height)` at the current distance.
    fn ortho_extents(&self, aspect: f32) -> (f32, f32) {
        let distance = self.position.sub(self.target).length().max(1.0);
        let half_height = distance * (self.fov_y * 0.5).tan();
        let aspect = if aspect.is_finite() && aspect > f32::EPSILON {
            aspect
        } else {
            1.0
        };
        (half_height * aspect, half_height)
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

    /// Pan by screen pixels using the shipped `TrackballControls` pan math.
    ///
    /// `TrackballControls` converts a pixel delta into a world-space offset:
    /// `target_distance = distance * tan(fov_y / 2)` and
    /// `world_per_pixel = 2 * target_distance / viewport_height * PAN_SPEED`.
    /// The camera and its target then move together along `right * dx` and
    /// `up * -dy`, matching the shipped screen-space drag direction.
    ///
    /// A non-positive or non-finite `viewport_height` leaves the camera
    /// unchanged so callers can pass an unmeasured canvas without producing a
    /// division by zero.
    pub fn pan_pixels(&mut self, dx: f32, dy: f32, viewport_height: f32) {
        if !viewport_height.is_finite() || viewport_height <= 0.0 {
            return;
        }
        let distance = self.position.sub(self.target).length();
        let target_distance = distance * (self.fov_y * 0.5).tan();
        let world_per_pixel = 2.0 * target_distance / viewport_height * PAN_SPEED as f32;
        let (right, up, _) = self.basis();
        let delta = right
            .scale(dx * world_per_pixel)
            .add(up.scale(-dy * world_per_pixel));
        self.position = self.position.add(delta);
        self.target = self.target.add(delta);
    }

    /// Dolly toward or away from the target.
    pub fn zoom(&mut self, delta: f32) {
        let offset = self.position.sub(self.target);
        let distance = (offset.length() + delta).clamp(1.0, 100_000.0);
        self.position = self.target.add(offset.normalized().scale(distance));
    }

    /// Dolly by discrete wheel steps using the shipped `TrackballControls`
    /// dolly factor `0.95^ZOOM_SPEED`.
    ///
    /// Positive steps zoom in (the distance shrinks), negative steps zoom out.
    /// Only the camera position changes; the look-at target stays fixed like
    /// the shipped controls. The distance stays inside the same
    /// `1.0..=100_000.0` range used by [`Camera::zoom`].
    pub fn zoom_steps(&mut self, steps: f32) {
        let offset = self.position.sub(self.target);
        let distance = offset.length();
        if !distance.is_finite() || distance <= f32::EPSILON {
            return;
        }
        let factor = 0.95_f32.powf(ZOOM_SPEED as f32 * steps);
        let distance = (distance * factor).clamp(1.0, 100_000.0);
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

    /// Frame a set of world positions.
    ///
    /// Centres the target on the bounding-box centre and places the camera on
    /// the +Z axis at the distance where the bounding sphere fits the vertical
    /// field of view with a 1.2 safety factor.
    ///
    /// The shipped `3d-force-graph` panel uses [`Camera::shipped_for_nodes`]
    /// instead, which keeps the shipped initial camera placement. This helper
    /// remains available for callers that want to frame an arbitrary point set.
    ///
    /// An empty slice leaves the camera unchanged. The far plane is extended
    /// when the fitted distance would otherwise clip the sphere.
    pub fn fit_to_positions(&mut self, positions: &[Vec3]) {
        if positions.is_empty() {
            return;
        }
        let mut minimum = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut maximum = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
        for position in positions {
            if !position.x.is_finite() || !position.y.is_finite() || !position.z.is_finite() {
                continue;
            }
            minimum = Vec3::new(
                minimum.x.min(position.x),
                minimum.y.min(position.y),
                minimum.z.min(position.z),
            );
            maximum = Vec3::new(
                maximum.x.max(position.x),
                maximum.y.max(position.y),
                maximum.z.max(position.z),
            );
        }
        if !minimum.x.is_finite() || !maximum.x.is_finite() {
            return;
        }
        let centre = minimum.add(maximum).scale(0.5);
        let radius = positions
            .iter()
            .filter(|position| {
                position.x.is_finite() && position.y.is_finite() && position.z.is_finite()
            })
            .map(|position| position.sub(centre).length())
            .fold(0.0_f32, f32::max)
            .max(1.0);
        let distance = radius / (self.fov_y * 0.5).sin() * 1.2;
        self.position = centre.add(Vec3::new(0.0, 0.0, distance));
        self.target = centre;
        // Keep the whole bounding sphere between the near and far planes.
        self.far = self.far.max(distance + radius * 2.0);
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
        let (right, up, back) = self.basis();
        let forward = back.scale(-1.0);
        if self.orthographic {
            // Orthographic rays share the view direction and start on the
            // camera plane, offset by the frustum extents.
            let (half_width, half_height) = self.ortho_extents(aspect);
            let origin = self
                .position
                .add(right.scale(ndc_x * half_width))
                .add(up.scale(ndc_y * half_height));
            return (origin, forward);
        }
        let tan = (self.fov_y * 0.5).tan();
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
mod tests;
