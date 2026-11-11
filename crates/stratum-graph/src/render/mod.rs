//! Headless graph renderer: GPU pipelines plus a deterministic CPU fallback.
//!
//! [`Renderer::gpu`] builds the wgpu pipelines described by the submodules and
//! renders into an offscreen multisampled target, resolving into an
//! `Rgba8UnormSrgb` texture. [`Renderer::software`] uses the CPU rasterizer when
//! no adapter exists. Both paths expose the same [`Frame`] type so callers can
//! save PNGs or compare byte output across runs.

pub mod device;
pub mod labels;
pub mod links;
pub mod nodes;
pub mod picking;
pub mod sdf;
pub mod software;

use crate::camera::Camera;
use crate::data::srgb_to_linear;
use crate::render::device::{
    clear_color, create_color_targets, create_gpu, create_uniform_bind_group,
    create_uniform_layout, padded_row_bytes, read_texture, GpuContext, Uniforms,
};
use crate::render::labels::{
    create_label_buffer, create_label_pipelines, draw_labels, label_vertices, LabelPipelines,
};
use crate::render::links::{
    build_link_cache, create_link_pipelines, draw_links, link_fingerprint, LinkCache, LinkPipelines,
};
use crate::render::nodes::{
    create_instance_buffer, create_node_pipelines, draw_nodes, node_instances, NodePipelines,
};
use crate::render::picking::{pick_gpu, validate_pick};
use crate::render::sdf::{GlyphAtlas, FONT_BYTES};
use crate::render::software::SoftwareRenderer;
use crate::scene::{Scene, SceneOptions};
use image::ImageEncoder as _;
use pkm_core::{PkmError, PkmResult};
use std::path::Path;
use wgpu::{
    CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, TextureFormat,
};
/// Renderer configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderConfig {
    /// Target width in pixels.
    pub width: u32,
    /// Target height in pixels.
    pub height: u32,
    /// Multisample count for the visible pass.
    pub sample_count: u32,
    /// Visible colour format.
    pub color_format: TextureFormat,
    /// Depth format.
    pub depth_format: TextureFormat,
    /// Label text height in world units.
    pub label_height: f32,
    /// Maximum number of labels emitted per scene.
    pub max_labels: usize,
    /// Link curvature setting carried into the scene options.
    pub link_curvature: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            sample_count: 4,
            color_format: TextureFormat::Rgba8UnormSrgb,
            depth_format: TextureFormat::Depth32Float,
            label_height: 6.0,
            max_labels: 2048,
            link_curvature: crate::scene::DEFAULT_LINK_CURVATURE,
        }
    }
}

impl RenderConfig {
    /// Scene options matching this render configuration.
    pub fn scene_options(&self) -> SceneOptions {
        SceneOptions {
            max_labels: self.max_labels,
            label_height: self.label_height,
            link_curvature: self.link_curvature,
            ..SceneOptions::default()
        }
    }
}

/// A rendered image with padded readback rows.
#[derive(Debug, Clone)]
pub struct Frame {
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
    bytes: Vec<u8>,
}

impl Frame {
    /// Build a frame from a padded readback buffer.
    ///
    /// Returns [`PkmError::Render`] when the buffer is smaller than expected.
    pub fn from_readback(
        width: u32,
        height: u32,
        padded_bytes_per_row: u32,
        bytes: Vec<u8>,
    ) -> PkmResult<Self> {
        let required = u64::from(padded_bytes_per_row) * u64::from(height);
        if (bytes.len() as u64) < required {
            return Err(PkmError::Render(format!(
                "graph readback buffer {} is smaller than the required {} bytes",
                bytes.len(),
                required
            )));
        }
        Ok(Self {
            width,
            height,
            padded_bytes_per_row,
            bytes,
        })
    }

    /// Build a frame from tightly packed RGBA pixels.
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> PkmResult<Self> {
        let expected = (width as usize) * (height as usize) * 4;
        if rgba.len() != expected {
            return Err(PkmError::Render(format!(
                "rgba buffer {} does not match {}x{} pixels",
                rgba.len(),
                width,
                height
            )));
        }
        let padded = padded_row_bytes(width);
        let mut bytes = vec![0_u8; (padded as usize) * (height as usize)];
        for row in 0..height as usize {
            let source = row * (width as usize) * 4;
            let destination = row * (padded as usize);
            bytes[destination..destination + (width as usize) * 4]
                .copy_from_slice(&rgba[source..source + (width as usize) * 4]);
        }
        Ok(Self {
            width,
            height,
            padded_bytes_per_row: padded,
            bytes,
        })
    }

    /// Frame width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Frame height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Tightly packed RGBA pixels.
    pub fn rgba(&self) -> Vec<u8> {
        let row_bytes = (self.width as usize) * 4;
        let padded = self.padded_bytes_per_row as usize;
        let mut output = Vec::with_capacity(row_bytes * self.height as usize);
        for row in 0..self.height as usize {
            let start = row * padded;
            output.extend_from_slice(&self.bytes[start..start + row_bytes]);
        }
        output
    }

    /// Read a single RGBA pixel, if the coordinates are in bounds.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let offset = (y as usize) * (self.padded_bytes_per_row as usize) + (x as usize) * 4;
        let pixel = self.bytes.get(offset..offset + 4)?;
        Some([pixel[0], pixel[1], pixel[2], pixel[3]])
    }

    /// Deterministic FNV-1a 64-bit hash of the tightly packed RGBA pixels.
    ///
    /// The golden runner prints this value so a changed frame is obvious even
    /// when the PNG is regenerated.
    pub fn content_hash(&self) -> String {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for byte in self.rgba() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        format!("{hash:016x}")
    }

    /// Encode the frame as a PNG.
    pub fn png(&self) -> PkmResult<Vec<u8>> {
        let rgba = self.rgba();
        let mut output = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut output);
        encoder
            .write_image(
                &rgba,
                self.width,
                self.height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|error| PkmError::Render(format!("graph png encode failed: {error}")))?;
        Ok(output)
    }

    /// Save the frame as a PNG at `path`.
    pub fn save_png(&self, path: &Path) -> PkmResult<()> {
        let rgba = self.rgba();
        let file = std::fs::File::create(path)
            .map_err(|error| PkmError::Render(format!("graph png create failed: {error}")))?;
        let mut writer = std::io::BufWriter::new(file);
        let encoder = image::codecs::png::PngEncoder::new(&mut writer);
        encoder
            .write_image(
                &rgba,
                self.width,
                self.height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|error| PkmError::Render(format!("graph png write failed: {error}")))?;
        Ok(())
    }
}

/// GPU-backed renderer.
pub struct GpuRenderer {
    context: GpuContext,
    config: RenderConfig,
    uniform_layout: wgpu::BindGroupLayout,
    nodes: NodePipelines,
    links: LinkPipelines,
    labels: LabelPipelines,
    atlas: GlyphAtlas,
    link_cache: Option<Box<LinkCache>>,
}

impl GpuRenderer {
    /// Build the GPU renderer, returning [`PkmError::Render`] when no adapter
    /// is available.
    pub fn new(config: RenderConfig) -> PkmResult<Self> {
        let context = create_gpu()?;
        let uniform_layout = create_uniform_layout(&context.device);
        let nodes =
            create_node_pipelines(&context.device, &context.queue, &uniform_layout, &config);
        let links = create_link_pipelines(&context.device, &uniform_layout, &config);
        let atlas = GlyphAtlas::build(FONT_BYTES)?;
        let labels = create_label_pipelines(
            &context.device,
            &context.queue,
            &uniform_layout,
            &atlas,
            &config,
        )?;
        Ok(Self {
            context,
            config,
            uniform_layout,
            nodes,
            links,
            labels,
            atlas,
            link_cache: None,
        })
    }

    /// Adapter name reported by wgpu.
    pub fn adapter_name(&self) -> &str {
        &self.context.adapter_name
    }

    /// Render a scene from a camera.
    pub fn render(&mut self, scene: &Scene, camera: &Camera) -> PkmResult<Frame> {
        let width = self.config.width.max(1);
        let height = self.config.height.max(1);
        let uniforms = Uniforms::new(camera, width, height);
        let (_uniform_buffer, bind_group) =
            create_uniform_bind_group(&self.context.device, &self.uniform_layout, &uniforms);
        let instances = node_instances(scene);
        let node_buffer =
            create_instance_buffer(&self.context.device, &self.context.queue, &instances);
        let fingerprint = link_fingerprint(scene);
        let refresh = self
            .link_cache
            .as_ref()
            .map(|cache| !cache.matches(fingerprint))
            .unwrap_or(true);
        if refresh {
            self.link_cache = Some(Box::new(build_link_cache(
                &self.context.device,
                &self.context.queue,
                scene,
            )));
        }
        let link_cache = self.link_cache.as_ref().expect("link cache");
        let labels = label_vertices(scene, &self.atlas);
        let label_buffer = create_label_buffer(&self.context.device, &self.context.queue, &labels);
        let targets = create_color_targets(
            &self.context.device,
            width,
            height,
            self.config.sample_count,
            self.config.color_format,
            self.config.depth_format,
        );
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("graph-render-encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("graph-render-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &targets.color_view,
                    depth_slice: None,
                    resolve_target: Some(&targets.resolve_view),
                    ops: Operations {
                        load: LoadOp::Clear(clear_color(scene.background)),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &targets.depth_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &bind_group, &[]);
            draw_links(
                &mut pass,
                &self.links,
                link_cache.buffer(),
                link_cache.count(),
            );
            draw_nodes(&mut pass, &self.nodes, &node_buffer, instances.len() as u32);
            draw_labels(&mut pass, &self.labels, &label_buffer, labels.len() as u32);
        }
        self.context.queue.submit(Some(encoder.finish()));
        let bytes = read_texture(&self.context, &targets.resolve, width, height)?;
        Frame::from_readback(width, height, padded_row_bytes(width), bytes)
    }

    /// Pick the node under a screen pixel.
    pub fn pick(&self, scene: &Scene, camera: &Camera, x: f32, y: f32) -> PkmResult<Option<usize>> {
        let width = self.config.width.max(1);
        let height = self.config.height.max(1);
        let uniforms = Uniforms::new(camera, width, height);
        let (_uniform_buffer, bind_group) =
            create_uniform_bind_group(&self.context.device, &self.uniform_layout, &uniforms);
        let instances = node_instances(scene);
        let node_buffer =
            create_instance_buffer(&self.context.device, &self.context.queue, &instances);
        let picked = pick_gpu(
            &self.context,
            &self.nodes,
            &node_buffer,
            instances.len() as u32,
            &bind_group,
            width,
            height,
            x,
            y,
        )?;
        validate_pick(picked, scene.nodes.len())
    }
}

/// A renderer backend.
pub enum Renderer {
    /// GPU-backed renderer.
    Gpu(GpuRenderer),
    /// Deterministic CPU fallback.
    Software(SoftwareRenderer),
}

impl Renderer {
    /// Build a GPU renderer.
    pub fn gpu(config: RenderConfig) -> PkmResult<Self> {
        Ok(Self::Gpu(GpuRenderer::new(config)?))
    }

    /// Build a software renderer.
    pub fn software(config: RenderConfig) -> PkmResult<Self> {
        Ok(Self::Software(SoftwareRenderer::new(config)))
    }

    /// Try the GPU backend and fall back to the software rasterizer.
    pub fn auto(config: RenderConfig) -> PkmResult<Self> {
        match Self::gpu(config) {
            Ok(renderer) => Ok(renderer),
            Err(error) => {
                log::warn!("stratum-graph GPU backend unavailable, using software: {error}");
                Self::software(config)
            }
        }
    }

    /// Backend name for reports.
    pub fn backend(&self) -> &'static str {
        match self {
            Self::Gpu(_) => "gpu",
            Self::Software(_) => "software",
        }
    }

    /// GPU adapter name, when the GPU backend is active.
    pub fn adapter_name(&self) -> Option<&str> {
        match self {
            Self::Gpu(renderer) => Some(renderer.adapter_name()),
            Self::Software(_) => None,
        }
    }

    /// Render a scene.
    pub fn render(&mut self, scene: &Scene, camera: &Camera) -> PkmResult<Frame> {
        match self {
            Self::Gpu(renderer) => renderer.render(scene, camera),
            Self::Software(renderer) => renderer.render(scene, camera),
        }
    }

    /// Pick the node under a screen pixel.
    pub fn pick(
        &mut self,
        scene: &Scene,
        camera: &Camera,
        x: f32,
        y: f32,
    ) -> PkmResult<Option<usize>> {
        match self {
            Self::Gpu(renderer) => renderer.pick(scene, camera, x, y),
            Self::Software(renderer) => {
                Ok(scene.pick_cpu(camera, x, y, renderer.width(), renderer.height()))
            }
        }
    }
}

/// Convert a sRGB colour to linear values for shader input.
pub fn linear_color(color: [u8; 3]) -> [f32; 3] {
    srgb_to_linear(color)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_round_trips_rgba() {
        let rgba = vec![1_u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let frame = Frame::from_rgba(2, 2, &rgba).expect("frame");
        assert_eq!(frame.width(), 2);
        assert_eq!(frame.height(), 2);
        assert_eq!(frame.rgba(), rgba);
        assert_eq!(frame.pixel(1, 1), Some([13, 14, 15, 16]));
        assert_eq!(frame.pixel(2, 0), None);
    }

    #[test]
    fn frame_rejects_short_readback() {
        assert!(Frame::from_readback(2, 2, 256, vec![0; 100]).is_err());
        assert!(Frame::from_readback(2, 2, 256, vec![0; 512]).is_ok());
    }

    #[test]
    fn png_has_the_png_signature() {
        let frame = Frame::from_rgba(2, 2, &[0_u8; 16]).expect("frame");
        let png = frame.png().expect("png");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn scene_options_follow_config() {
        let config = RenderConfig {
            label_height: 9.0,
            max_labels: 12,
            link_curvature: 0.3,
            ..RenderConfig::default()
        };
        let options = config.scene_options();
        assert_eq!(options.label_height, 9.0);
        assert_eq!(options.max_labels, 12);
        assert_eq!(options.link_curvature, 0.3);
    }
}
