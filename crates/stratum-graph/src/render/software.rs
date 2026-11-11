//! Deterministic CPU rasterizer used when no wgpu adapter is available.
//!
//! The fallback reproduces the visual structure of the GPU path (background,
//! links, shaded node discs and SDF label glyphs) without requiring a GPU. It
//! uses the same camera projection as the GPU path and writes straight into an
//! RGBA byte buffer, so the smoke test can compare renders byte-for-byte.

use crate::camera::Camera;
use crate::curve::link_curve_points;
use crate::render::sdf::{GlyphAtlas, ATLAS_SIZE, FONT_BYTES};
use crate::render::{Frame, RenderConfig};
use crate::scene::Scene;
use pkm_core::PkmResult;

/// A software renderer holding the optional label atlas.
pub struct SoftwareRenderer {
    config: RenderConfig,
    atlas: Option<GlyphAtlas>,
}

impl SoftwareRenderer {
    /// Create a software renderer for the given configuration.
    pub fn new(config: RenderConfig) -> Self {
        Self {
            config,
            atlas: None,
        }
    }

    /// Target width in pixels.
    pub fn width(&self) -> u32 {
        self.config.width.max(1)
    }

    /// Target height in pixels.
    pub fn height(&self) -> u32 {
        self.config.height.max(1)
    }

    /// Render the scene into an RGBA frame.
    pub fn render(&mut self, scene: &Scene, camera: &Camera) -> PkmResult<Frame> {
        let width = self.config.width.max(1);
        let height = self.config.height.max(1);
        let mut pixels = vec![0_u8; (width * height * 4) as usize];
        fill_background(&mut pixels, scene.background);

        let view_proj = camera.view_projection(width, height);
        let scale = (height as f32 * 0.5) / (camera.fov_y * 0.5).tan();

        let projected: Vec<Option<Projected>> = scene
            .nodes
            .iter()
            .map(|node| project_point(&view_proj, node.position, camera, width, height, scale))
            .collect();

        for (link_index, link) in scene.links.iter().enumerate() {
            let (Some(source_node), Some(target_node)) =
                (scene.nodes.get(link.source), scene.nodes.get(link.target))
            else {
                continue;
            };
            let distance = match (
                projected.get(link.source).copied().flatten(),
                projected.get(link.target).copied().flatten(),
            ) {
                (Some(source), Some(target)) => (source.distance + target.distance) * 0.5,
                _ => continue,
            };
            let width_px = (link.width * scale / distance.max(1.0)).max(1.0);
            let points = link_curve_points(
                source_node.position,
                target_node.position,
                link.curvature,
                link_index,
            );
            let projected_points: Vec<Projected> = points
                .iter()
                .filter_map(|point| project_point(&view_proj, *point, camera, width, height, scale))
                .collect();
            if projected_points.len() != points.len() {
                continue;
            }
            for segment in projected_points.windows(2) {
                draw_line(
                    &mut pixels,
                    width,
                    height,
                    segment[0].x,
                    segment[0].y,
                    segment[1].x,
                    segment[1].y,
                    width_px,
                    link.color,
                    link.opacity,
                );
            }
        }

        let mut order: Vec<(usize, f32)> = projected
            .iter()
            .enumerate()
            .filter_map(|(index, projected)| projected.map(|projected| (index, projected.distance)))
            .collect();
        order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (index, _) in order {
            let Some(projected) = projected[index] else {
                continue;
            };
            let node = &scene.nodes[index];
            let radius = (node.radius * scale / projected.distance.max(1.0)).max(1.0);
            draw_node(
                &mut pixels,
                width,
                height,
                projected.x,
                projected.y,
                radius,
                node.color,
                node.opacity,
            );
        }

        if !scene.labels.is_empty() {
            if self.atlas.is_none() {
                self.atlas = Some(GlyphAtlas::build(FONT_BYTES)?);
            }
            if let Some(atlas) = &self.atlas {
                draw_labels(
                    &mut pixels,
                    width,
                    height,
                    scene,
                    &view_proj,
                    camera,
                    scale,
                    atlas,
                );
            }
        }

        Frame::from_rgba(width, height, &pixels)
    }
}

#[derive(Debug, Clone, Copy)]
struct Projected {
    x: f32,
    y: f32,
    distance: f32,
}

fn project_point(
    view_proj: &crate::math::Mat4,
    position: crate::math::Vec3,
    camera: &Camera,
    width: u32,
    height: u32,
    _scale: f32,
) -> Option<Projected> {
    let (x, y, _) = view_proj.project_to_pixels(position, width, height)?;
    let distance = position.sub(camera.position).length();
    if distance <= 0.0 || !distance.is_finite() {
        return None;
    }
    Some(Projected { x, y, distance })
}

fn fill_background(pixels: &mut [u8], background: [u8; 4]) {
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.copy_from_slice(&background);
    }
}

fn blend(pixels: &mut [u8], width: u32, height: u32, x: i32, y: i32, color: [u8; 3], alpha: f32) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }
    let alpha = alpha.clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    let offset = ((y as u32 * width + x as u32) * 4) as usize;
    let Some(pixel) = pixels.get_mut(offset..offset + 4) else {
        return;
    };
    for channel in 0..3 {
        let source = f32::from(color[channel]);
        let destination = f32::from(pixel[channel]);
        pixel[channel] = (source * alpha + destination * (1.0 - alpha)).round() as u8;
    }
    pixel[3] = 255;
}

#[allow(clippy::too_many_arguments)]
fn draw_line(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    width_px: f32,
    color: [u8; 3],
    opacity: f32,
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as u32;
    let radius = (width_px * 0.5).max(0.5);
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let x = x0 + dx * t;
        let y = y0 + dy * t;
        let left = (x - radius).floor() as i32;
        let right = (x + radius).ceil() as i32;
        let top = (y - radius).floor() as i32;
        let bottom = (y + radius).ceil() as i32;
        for py in top..=bottom {
            for px in left..=right {
                let ox = px as f32 + 0.5 - x;
                let oy = py as f32 + 0.5 - y;
                if ox * ox + oy * oy <= radius * radius {
                    blend(pixels, width, height, px, py, color, opacity);
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_node(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x: f32,
    y: f32,
    radius: f32,
    color: [u8; 3],
    opacity: f32,
) {
    let left = (x - radius).floor() as i32;
    let right = (x + radius).ceil() as i32;
    let top = (y - radius).floor() as i32;
    let bottom = (y + radius).ceil() as i32;
    let light = [0.35_f32, 0.8, 0.55];
    let light_length = (light[0] * light[0] + light[1] * light[1] + light[2] * light[2]).sqrt();
    for py in top..=bottom {
        for px in left..=right {
            let ox = (px as f32 + 0.5 - x) / radius;
            let oy = -(py as f32 + 0.5 - y) / radius;
            let squared = ox * ox + oy * oy;
            if squared > 1.0 {
                continue;
            }
            let oz = (1.0 - squared).max(0.0).sqrt();
            let diffuse = (ox * light[0] + oy * light[1] + oz * light[2]) / light_length;
            let shade = 0.32 + 0.68 * diffuse.max(0.0);
            let shaded = [
                (f32::from(color[0]) * shade).round().clamp(0.0, 255.0) as u8,
                (f32::from(color[1]) * shade).round().clamp(0.0, 255.0) as u8,
                (f32::from(color[2]) * shade).round().clamp(0.0, 255.0) as u8,
            ];
            blend(pixels, width, height, px, py, shaded, opacity);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_labels(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    scene: &Scene,
    view_proj: &crate::math::Mat4,
    camera: &Camera,
    scale: f32,
    atlas: &GlyphAtlas,
) {
    let mut order: Vec<(usize, f32)> = Vec::new();
    for (index, label) in scene.labels.iter().enumerate() {
        if let Some(projected) =
            project_point(view_proj, label.position, camera, width, height, scale)
        {
            order.push((index, projected.distance));
        }
    }
    order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    for (index, distance) in order {
        let label = &scene.labels[index];
        let Some(projected) =
            project_point(view_proj, label.position, camera, width, height, scale)
        else {
            continue;
        };
        let quads = atlas.layout(&label.text);
        if quads.is_empty() {
            continue;
        }
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for quad in &quads {
            min_x = min_x.min(quad.x);
            max_x = max_x.max(quad.x + quad.width);
            min_y = min_y.min(quad.y - quad.height);
            max_y = max_y.max(quad.y);
        }
        let center_x = (min_x + max_x) * 0.5;
        let center_y = (min_y + max_y) * 0.5;
        let unit = label.height * scale / (distance.max(1.0) * atlas.font_px());
        for quad in quads {
            let left = projected.x + (quad.x - center_x) * unit;
            let right = projected.x + (quad.x + quad.width - center_x) * unit;
            let top = projected.y - (quad.y - center_y) * unit;
            let bottom = projected.y - (quad.y - quad.height - center_y) * unit;
            draw_glyph(
                pixels,
                width,
                height,
                atlas,
                &quad,
                left,
                right,
                top,
                bottom,
                label.color,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_glyph(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    atlas: &GlyphAtlas,
    quad: &crate::render::sdf::GlyphQuad,
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    color: [u8; 3],
) {
    let x0 = left.floor() as i32;
    let x1 = right.ceil() as i32;
    let y0 = top.floor() as i32;
    let y1 = bottom.ceil() as i32;
    let atlas_data = atlas.texture_data();
    let span_x = (right - left).max(1e-3);
    let span_y = (bottom - top).max(1e-3);
    for py in y0..=y1 {
        for px in x0..=x1 {
            if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                continue;
            }
            let u = (px as f32 + 0.5 - left) / span_x;
            let v = (py as f32 + 0.5 - top) / span_y;
            if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                continue;
            }
            let atlas_u = quad.u0 + (quad.u1 - quad.u0) * u;
            let atlas_v = quad.v0 + (quad.v1 - quad.v0) * v;
            let tx = (atlas_u * ATLAS_SIZE as f32)
                .floor()
                .clamp(0.0, ATLAS_SIZE as f32 - 1.0) as u32;
            let ty = (atlas_v * ATLAS_SIZE as f32)
                .floor()
                .clamp(0.0, ATLAS_SIZE as f32 - 1.0) as u32;
            let sample = f32::from(atlas_data[(ty * ATLAS_SIZE + tx) as usize]) / 255.0;
            let alpha = smoothstep(0.42, 0.58, sample);
            if alpha > 0.01 {
                blend(pixels, width, height, px, py, color, alpha);
            }
        }
    }
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::fixture;
    use crate::math::Vec3;
    use crate::scene::{Scene, SceneOptions};

    #[test]
    fn software_render_produces_visible_pixels() {
        let data = fixture(10, 1);
        let positions: Vec<Vec3> = (0..10)
            .map(|index| Vec3::new(index as f32 * 8.0 - 40.0, 0.0, 0.0))
            .collect();
        let scene = Scene::build(&data, &positions, &SceneOptions::default()).expect("scene");
        let mut renderer = SoftwareRenderer::new(RenderConfig {
            width: 320,
            height: 180,
            ..RenderConfig::default()
        });
        let frame = renderer.render(&scene, &Camera::default()).expect("frame");
        let background = scene.background;
        let visible = frame
            .rgba()
            .chunks_exact(4)
            .filter(|pixel| pixel[0] != background[0] || pixel[1] != background[1])
            .count();
        assert!(visible > 0);
    }

    #[test]
    fn software_render_is_deterministic() {
        let data = fixture(8, 1);
        let positions = data.seed_positions();
        let scene = Scene::build(&data, &positions, &SceneOptions::default()).expect("scene");
        let mut renderer = SoftwareRenderer::new(RenderConfig {
            width: 160,
            height: 120,
            ..RenderConfig::default()
        });
        let first = renderer.render(&scene, &Camera::default()).expect("frame");
        let second = renderer.render(&scene, &Camera::default()).expect("frame");
        assert_eq!(first.rgba(), second.rgba());
    }
}
