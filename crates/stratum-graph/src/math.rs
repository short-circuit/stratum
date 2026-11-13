//! Small, dependency-free linear algebra used by the camera, scene and renderer.
//!
//! The graph crate deliberately keeps its math self-contained so the physics and
//! camera parity layers stay deterministic and reviewable. Matrices are stored in
//! column-major order (the same layout used by WGSL and three.js), and the
//! projection matrix maps depth into the `0..1` range required by WebGPU.

/// Three-dimensional vector with the arithmetic helpers the graph needs.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    /// X component.
    pub x: f32,
    /// Y component.
    pub y: f32,
    /// Z component.
    pub z: f32,
}

#[allow(clippy::should_implement_trait)]
impl Vec3 {
    /// Create a vector from its components.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// The zero vector.
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    /// The `(0, 1, 0)` up vector used by the camera.
    pub const UP: Self = Self::new(0.0, 1.0, 0.0);

    /// Squared length.
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Euclidean length.
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Dot product.
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product.
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// Return a unit-length copy, or [`Vec3::ZERO`] for a zero vector.
    pub fn normalized(self) -> Self {
        let len = self.length();
        if len > f32::EPSILON {
            Self::new(self.x / len, self.y / len, self.z / len)
        } else {
            Self::ZERO
        }
    }

    /// Component-wise addition.
    pub fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }

    /// Component-wise subtraction.
    pub fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    /// Uniform scale.
    pub fn scale(self, factor: f32) -> Self {
        Self::new(self.x * factor, self.y * factor, self.z * factor)
    }

    /// Linear interpolation.
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self.add(other.sub(self).scale(t))
    }
}

/// A 4x4 column-major matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    /// Column-major matrix elements (`m[column * 4 + row]`).
    pub m: [f32; 16],
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mat4 {
    /// The identity matrix.
    pub const IDENTITY: Self = Self {
        m: [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ],
    };

    /// Build a right-handed view matrix looking from `eye` at `target`.
    ///
    /// When the view direction is parallel to `up`, the world Z axis is used as a
    /// fallback so the matrix stays finite.
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let forward = target.sub(eye).normalized();
        let mut side = forward.cross(up).normalized();
        if side.length_squared() < f32::EPSILON {
            side = forward.cross(Vec3::new(0.0, 0.0, 1.0)).normalized();
        }
        let true_up = side.cross(forward);
        Self {
            m: [
                side.x,
                true_up.x,
                -forward.x,
                0.0,
                side.y,
                true_up.y,
                -forward.y,
                0.0,
                side.z,
                true_up.z,
                -forward.z,
                0.0,
                -side.dot(eye),
                -true_up.dot(eye),
                forward.dot(eye),
                1.0,
            ],
        }
    }

    /// Build a perspective projection with WebGPU's `0..1` depth range.
    pub fn perspective(fov_y_radians: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y_radians * 0.5).tan();
        let aspect = if aspect.is_finite() && aspect > f32::EPSILON {
            aspect
        } else {
            1.0
        };
        // WebGPU maps the near plane to depth 0 and the far plane to depth 1.
        // With a right-handed view matrix, view-space z is negative in front of
        // the camera, so clip.w = -z_view and the depth row is
        // `far / (near - far) * z - far * near / (near - far)`.
        let denominator = near - far;
        Self {
            m: [
                f / aspect,
                0.0,
                0.0,
                0.0,
                0.0,
                f,
                0.0,
                0.0,
                0.0,
                0.0,
                far / denominator,
                -1.0,
                0.0,
                0.0,
                (far * near) / denominator,
                0.0,
            ],
        }
    }

    /// Build an orthographic projection with WebGPU's `0..1` depth range.
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let width = (right - left).max(f32::EPSILON);
        let height = (top - bottom).max(f32::EPSILON);
        let range = (far - near).max(f32::EPSILON);
        Self {
            m: [
                2.0 / width,
                0.0,
                0.0,
                0.0,
                0.0,
                2.0 / height,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0 / range,
                0.0,
                -(right + left) / width,
                -(top + bottom) / height,
                -near / range,
                1.0,
            ],
        }
    }

    /// Matrix multiplication (`self * rhs`).
    pub fn multiply(self, rhs: Self) -> Self {
        let mut out = [0.0_f32; 16];
        for column in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.m[k * 4 + row] * rhs.m[column * 4 + k];
                }
                out[column * 4 + row] = sum;
            }
        }
        Self { m: out }
    }

    /// Transform a point, returning `None` when the result is behind the camera
    /// or contains non-finite values.
    pub fn transform_point(self, point: Vec3) -> Option<Vec3> {
        let x = self.m[0] * point.x + self.m[4] * point.y + self.m[8] * point.z + self.m[12];
        let y = self.m[1] * point.x + self.m[5] * point.y + self.m[9] * point.z + self.m[13];
        let z = self.m[2] * point.x + self.m[6] * point.y + self.m[10] * point.z + self.m[14];
        let w = self.m[3] * point.x + self.m[7] * point.y + self.m[11] * point.z + self.m[15];
        if !x.is_finite() || !y.is_finite() || !z.is_finite() || !w.is_finite() || w.abs() < 1e-7 {
            return None;
        }
        Some(Vec3::new(x / w, y / w, z / w))
    }

    /// Transform a direction (ignores translation).
    pub fn transform_direction(self, direction: Vec3) -> Vec3 {
        Vec3::new(
            self.m[0] * direction.x + self.m[4] * direction.y + self.m[8] * direction.z,
            self.m[1] * direction.x + self.m[5] * direction.y + self.m[9] * direction.z,
            self.m[2] * direction.x + self.m[6] * direction.y + self.m[10] * direction.z,
        )
    }

    /// Project a world point into normalized device coordinates.
    pub fn project_ndc(self, point: Vec3) -> Option<Vec3> {
        self.transform_point(point)
    }

    /// Project a world point into pixel coordinates for a `width` x `height`
    /// viewport. Returns `(x, y, ndc_depth)`; the depth is in `0..1`.
    pub fn project_to_pixels(
        self,
        point: Vec3,
        width: u32,
        height: u32,
    ) -> Option<(f32, f32, f32)> {
        let ndc = self.project_ndc(point)?;
        if ndc.z < 0.0 || ndc.z > 1.0 {
            return None;
        }
        let x = (ndc.x * 0.5 + 0.5) * width as f32;
        let y = (0.5 - ndc.y * 0.5) * height as f32;
        Some((x, y, ndc.z))
    }

    /// Convert a normalized device depth into a linear view-space distance for a
    /// perspective matrix. Useful for depth-aware picking.
    pub fn linearize_depth(self, depth: f32, near: f32, far: f32) -> f32 {
        if self.m[11].abs() < f32::EPSILON {
            return depth;
        }
        let z_ndc = depth * 2.0 - 1.0;
        let view_z = (near * far) / (far - z_ndc * (far - near));
        if view_z.is_finite() {
            view_z
        } else {
            far
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-4, "expected {b}, got {a}");
    }

    #[test]
    fn vector_arithmetic_and_normalization() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        approx(v.length(), 5.0);
        approx(v.normalized().length(), 1.0);
        assert_eq!(Vec3::ZERO.normalized(), Vec3::ZERO);
        approx(v.dot(Vec3::new(1.0, 0.0, 0.0)), 3.0);
        assert_eq!(
            Vec3::new(1.0, 0.0, 0.0).cross(Vec3::UP),
            Vec3::new(0.0, 0.0, 1.0)
        );
    }

    #[test]
    fn perspective_projects_forward_point_inside_view() {
        let view = Mat4::look_at(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::UP);
        let proj = Mat4::perspective(45f32.to_radians(), 16.0 / 9.0, 0.1, 1000.0);
        let clip = proj.multiply(view);
        let (x, y, z) = clip
            .project_to_pixels(Vec3::ZERO, 1280, 720)
            .expect("origin visible");
        approx(x, 640.0);
        approx(y, 360.0);
        assert!(z > 0.0 && z < 1.0);
        assert!(clip
            .project_to_pixels(Vec3::new(0.0, 0.0, 20.0), 1280, 720)
            .is_none());
    }

    #[test]
    fn orthographic_maps_bounds() {
        let proj = Mat4::orthographic(-1.0, 1.0, -1.0, 1.0, 0.0, 10.0);
        let p = proj
            .transform_point(Vec3::new(1.0, 1.0, 5.0))
            .expect("finite");
        approx(p.x, 1.0);
        approx(p.y, 1.0);
        approx(p.z, 0.5);
    }

    #[test]
    fn look_at_handles_parallel_up() {
        let view = Mat4::look_at(Vec3::new(0.0, 10.0, 0.0), Vec3::ZERO, Vec3::UP);
        assert!(view.m.iter().all(|v| v.is_finite()));
    }
}
