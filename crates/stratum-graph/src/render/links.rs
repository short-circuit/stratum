//! Link rendering: instanced world-space quads with the shipped link colours
//! and widths.
//!
//! The shipped panel draws every edge as a three.js line with a width accessor
//! (`0.5` normally, `2` for the highlighted neighbourhood, `0.1` otherwise).
//! The GPU path expands each link into a camera-facing quad in the vertex
//! shader, so widths are honoured without a geometry shader.

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
pub fn link_instances(scene: &Scene) -> Vec<LinkInstance> {
    scene
        .links
        .iter()
        .filter_map(|link| {
            let start = scene.nodes.get(link.source)?;
            let end = scene.nodes.get(link.target)?;
            let linear = srgb_to_linear(link.color);
            Some(LinkInstance {
                start: [start.position.x, start.position.y, start.position.z],
                end: [end.position.x, end.position.y, end.position.z],
                color: [linear[0], linear[1], linear[2], link.opacity],
                width: link.width,
            })
        })
        .collect()
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
        assert_eq!(instances.len(), scene.links.len());
        for (instance, link) in instances.iter().zip(&scene.links) {
            let start = scene.nodes[link.source].position;
            assert_eq!(instance.start, [start.x, start.y, start.z]);
        }
    }

    #[test]
    fn empty_scene_has_no_instances() {
        let data = fixture(0, 1);
        let scene = Scene::build(&data, &[], &SceneOptions::default()).expect("scene");
        assert!(link_instances(&scene).is_empty());
    }
}
