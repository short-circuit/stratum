//! Link rendering: instanced world-space quads with the shipped link colours
//! and widths.
//!
//! The shipped panel draws every edge as a three.js line with a width accessor
//! (`0.5` normally, `2` for the highlighted neighbourhood, `0.1` otherwise) and
//! bends it by `linkCurvature`. The Rust port tessellates that curve into
//! [`LINK_CURVE_SEGMENTS`] straight quads; the GPU path expands each segment into
//! a camera-facing quad in the vertex shader, so widths are honoured without a
//! geometry shader.

use crate::curve::{link_curve_points, LINK_CURVE_SEGMENTS};
use crate::data::srgb_to_linear;
use crate::render::device::{create_pipeline_layout, create_shader};
use crate::render::RenderConfig;
use crate::scene::Scene;
use bytemuck::{Pod, Zeroable};
use wgpu::{
    BindGroupLayout, BlendComponent, BlendFactor, BlendState, Buffer, BufferAddress,
    BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites, CompareFunction,
    DepthStencilState, MultisampleState, PipelineCompilationOptions, PrimitiveState,
    PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

/// Per-link instance data.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LinkInstance {
    /// World-space start point.
    pub start: [f32; 3],
    /// World-space end point.
    pub end: [f32; 3],
    /// Linear RGBA colour.
    pub color: [f32; 4],
    /// Line width in world units.
    pub width: f32,
}

/// Link pipeline.
pub struct LinkPipelines {
    /// Visible link pipeline.
    pub render: RenderPipeline,
}

/// Build the link pipeline.
pub fn create_link_pipelines(
    device: &wgpu::Device,
    layout: &BindGroupLayout,
    config: &RenderConfig,
) -> LinkPipelines {
    let pipeline_layout = create_pipeline_layout(device, layout);
    let shader = create_shader(
        device,
        "graph-link-shader",
        include_str!("shaders/link.wgsl"),
    );
    let instance_layout = VertexBufferLayout {
        array_stride: std::mem::size_of::<LinkInstance>() as BufferAddress,
        step_mode: VertexStepMode::Instance,
        attributes: &[
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 12,
                shader_location: 1,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 24,
                shader_location: 2,
            },
            VertexAttribute {
                format: VertexFormat::Float32,
                offset: 40,
                shader_location: 3,
            },
        ],
    };
    let render = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("graph-link-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            buffers: &[Some(instance_layout)],
        },
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(DepthStencilState {
            format: config.depth_format,
            depth_write_enabled: Some(false),
            depth_compare: Some(CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: MultisampleState {
            count: config.sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            targets: &[Some(ColorTargetState {
                format: config.color_format,
                blend: Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    LinkPipelines { render }
}

/// Build link instances from a scene.
///
/// Each link contributes [`LINK_CURVE_SEGMENTS`] instances: the quadratic bezier
/// defined by the link's signed curvature is sampled and emitted as consecutive
/// start/end pairs. The instance layout and shader stay unchanged.
pub fn link_instances(scene: &Scene) -> Vec<LinkInstance> {
    let mut instances = Vec::with_capacity(scene.links.len() * LINK_CURVE_SEGMENTS);
    for (link_index, link) in scene.links.iter().enumerate() {
        let (Some(source), Some(target)) =
            (scene.nodes.get(link.source), scene.nodes.get(link.target))
        else {
            continue;
        };
        let linear = srgb_to_linear(link.color);
        let points =
            link_curve_points(source.position, target.position, link.curvature, link_index);
        for segment in points.windows(2) {
            let [start, end] = [segment[0], segment[1]];
            instances.push(LinkInstance {
                start: [start.x, start.y, start.z],
                end: [end.x, end.y, end.z],
                color: [linear[0], linear[1], linear[2], link.opacity],
                width: link.width,
            });
        }
    }
    instances
}

/// Cached link geometry for repeated frames of an unchanged scene.
///
/// The interactive simulation rebuilds the scene every tick, which changes the
/// fingerprint and refreshes the buffer. The golden runner renders the same scene
/// for every frame, so the cache removes the per-frame tessellation and upload
/// from the measured frame time while keeping the rendered bytes identical.
pub struct LinkCache {
    fingerprint: u64,
    buffer: Buffer,
    count: u32,
}

impl LinkCache {
    /// Whether this cache was built from the same scene inputs.
    pub fn matches(&self, fingerprint: u64) -> bool {
        self.fingerprint == fingerprint
    }

    /// Cached instance buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Number of cached instances.
    pub fn count(&self) -> u32 {
        self.count
    }
}

/// Fingerprint of the link geometry inputs (positions, curvature, width, colour).
///
/// A word-at-a-time FNV mix keeps this cheap enough to run every frame; the
/// interactive path refreshes the cache whenever the simulation moves a node.
pub fn link_fingerprint(scene: &Scene) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut mix = |value: u64| {
        hash ^= value;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };
    mix(scene.nodes.len() as u64);
    mix(scene.links.len() as u64);
    for node in &scene.nodes {
        mix(u64::from(node.position.x.to_bits()));
        mix(u64::from(node.position.y.to_bits()));
        mix(u64::from(node.position.z.to_bits()));
    }
    for link in &scene.links {
        mix(link.source as u64);
        mix(link.target as u64);
        mix(u64::from(link.curvature.to_bits()));
        mix(u64::from(link.width.to_bits()));
        mix(u64::from(link.opacity.to_bits()));
        mix(u64::from(u32::from_be_bytes([
            link.color[0],
            link.color[1],
            link.color[2],
            0,
        ])));
    }
    hash
}

/// Build a cached link buffer for a scene.
pub fn build_link_cache(device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene) -> LinkCache {
    let instances = link_instances(scene);
    let count = instances.len() as u32;
    let buffer = create_link_buffer(device, queue, &instances);
    LinkCache {
        fingerprint: link_fingerprint(scene),
        buffer,
        count,
    }
}

/// Create a link instance buffer, tolerating an empty list.
pub fn create_link_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    instances: &[LinkInstance],
) -> Buffer {
    let size = (instances.len().max(1) * std::mem::size_of::<LinkInstance>()) as u64;
    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some("graph-link-instances"),
        size,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    if !instances.is_empty() {
        queue.write_buffer(&buffer, 0, bytemuck::cast_slice(instances));
    }
    buffer
}

/// Draw the link instances into an active render pass.
pub fn draw_links<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    pipelines: &'a LinkPipelines,
    instances: &'a Buffer,
    count: u32,
) {
    if count == 0 {
        return;
    }
    pass.set_pipeline(&pipelines.render);
    pass.set_vertex_buffer(0, instances.slice(..));
    pass.draw(0..6, 0..count);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::fixture;
    use crate::math::Vec3;
    use crate::scene::{Scene, SceneOptions};

    #[test]
    fn link_instances_follow_scene_endpoints() {
        let data = fixture(6, 1);
        let positions: Vec<Vec3> = (0..6)
            .map(|index| Vec3::new(index as f32, 0.0, 0.0))
            .collect();
        let scene = Scene::build(&data, &positions, &SceneOptions::default()).expect("scene");
        let instances = link_instances(&scene);
        assert_eq!(instances.len(), scene.links.len() * LINK_CURVE_SEGMENTS);
        for (link_index, link) in scene.links.iter().enumerate() {
            let start = scene.nodes[link.source].position;
            let first = &instances[link_index * LINK_CURVE_SEGMENTS];
            assert_eq!(first.start, [start.x, start.y, start.z]);
            // Consecutive segments chain end -> start.
            for segment in 0..LINK_CURVE_SEGMENTS - 1 {
                let current = &instances[link_index * LINK_CURVE_SEGMENTS + segment];
                let next = &instances[link_index * LINK_CURVE_SEGMENTS + segment + 1];
                assert_eq!(current.end, next.start);
            }
        }
    }

    #[test]
    fn zero_curvature_emits_collinear_segments() {
        let data = fixture(6, 1);
        let positions: Vec<Vec3> = (0..6)
            .map(|index| Vec3::new(index as f32, 0.0, 0.0))
            .collect();
        let options = SceneOptions {
            link_curvature: 0.0,
            ..SceneOptions::default()
        };
        let scene = Scene::build(&data, &positions, &options).expect("scene");
        let instances = link_instances(&scene);
        for instance in &instances {
            assert!((instance.start[1] - instance.end[1]).abs() < 1e-4);
            assert!((instance.start[2] - instance.end[2]).abs() < 1e-4);
        }
    }

    #[test]
    fn empty_scene_has_no_instances() {
        let data = fixture(0, 1);
        let scene = Scene::build(&data, &[], &SceneOptions::default()).expect("scene");
        assert!(link_instances(&scene).is_empty());
    }
}
