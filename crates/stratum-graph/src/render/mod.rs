//! Headless graph renderer: GPU pipelines plus a deterministic CPU fallback.
//!
//! [`Renderer::gpu`] builds the wgpu pipelines described by the submodules and
//! renders into an offscreen multisampled target, resolving into an
//! `Rgba8UnormSrgb` texture. [`Renderer::software`] uses the CPU rasterizer when
//! no adapter exists. Both paths expose the same [`Frame`] type so callers can
//! save PNGs or compare byte output across runs.

pub mod device;
pub mod frame;
pub mod labels;
pub mod links;
pub mod nodes;
pub mod picking;
pub mod sdf;
pub mod software;

pub use frame::{Frame, RenderConfig};

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
use crate::scene::Scene;
use pkm_core::PkmResult;
use wgpu::{
    CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp,
};

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

    /// Update the target dimensions.
    ///
    /// The colour and depth targets are created per frame from
    /// [`RenderConfig`], so resizing only updates the configuration; pipelines,
    /// shaders and the glyph atlas stay intact.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
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

    /// Update the target dimensions for both backends.
    pub fn resize(&mut self, width: u32, height: u32) {
        match self {
            Self::Gpu(renderer) => renderer.resize(width, height),
            Self::Software(renderer) => renderer.resize(width, height),
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
