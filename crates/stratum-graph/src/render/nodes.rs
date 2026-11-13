//! Node sphere mesh and instanced node/pick pipelines.
//!
//! Nodes are rendered as low-poly UV spheres (the shipped panel uses
//! `nodeResolution: 8`), with one instance per scene node carrying the world
//! position, linear colour, radius and node index. The pick pipeline shares the
//! mesh and instance layout but encodes the index into an RGBA byte target.

use crate::data::srgb_to_linear;
use crate::render::device::{create_pipeline_layout, create_shader, Uniforms};
use crate::render::RenderConfig;
use crate::scene::Scene;
use bytemuck::{Pod, Zeroable};
use std::f32::consts::{PI, TAU};
use wgpu::{
    BindGroupLayout, Buffer, BufferAddress, BufferDescriptor, BufferUsages, ColorTargetState,
    ColorWrites, CompareFunction, DepthStencilState, Face, FrontFace, MultisampleState,
    PipelineCompilationOptions, PrimitiveState, PrimitiveTopology, RenderPipeline,
    RenderPipelineDescriptor, TextureFormat, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState, VertexStepMode,
};

/// Per-node instance data.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct NodeInstance {
    /// World position.
    pub position: [f32; 3],
    /// Linear RGBA colour.
    pub color: [f32; 4],
    /// Sphere radius.
    pub radius: f32,
    /// Node index for picking.
    pub index: u32,
}

/// Node pipelines and the shared sphere mesh.
pub struct NodePipelines {
    /// Visible node pipeline.
    pub render: RenderPipeline,
    /// Pick pipeline.
    pub pick: RenderPipeline,
    /// Sphere vertex buffer (`position + normal`).
    pub mesh_vertices: Buffer,
    /// Sphere index buffer.
    pub mesh_indices: Buffer,
    /// Sphere index count.
    pub index_count: u32,
}

/// Build the node and pick pipelines.
pub fn create_node_pipelines(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &BindGroupLayout,
    config: &RenderConfig,
) -> NodePipelines {
    let pipeline_layout = create_pipeline_layout(device, layout);
    let node_shader = create_shader(
        device,
        "graph-node-shader",
        include_str!("shaders/node.wgsl"),
    );
    let pick_shader = create_shader(
        device,
        "graph-pick-shader",
        include_str!("shaders/pick.wgsl"),
    );
    let (vertices, indices) = sphere_mesh(8, 6);
    let mesh_vertices = device.create_buffer(&BufferDescriptor {
        label: Some("graph-node-mesh"),
        size: (vertices.len() * std::mem::size_of::<[f32; 6]>()) as u64,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&mesh_vertices, 0, bytemuck::cast_slice(&vertices));
    let mesh_indices = device.create_buffer(&BufferDescriptor {
        label: Some("graph-node-indices"),
        size: (indices.len() * std::mem::size_of::<u32>()) as u64,
        usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&mesh_indices, 0, bytemuck::cast_slice(&indices));
    let mesh_layout = VertexBufferLayout {
        array_stride: (std::mem::size_of::<[f32; 6]>()) as BufferAddress,
        step_mode: VertexStepMode::Vertex,
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
        ],
    };
    let instance_layout = VertexBufferLayout {
        array_stride: std::mem::size_of::<NodeInstance>() as BufferAddress,
        step_mode: VertexStepMode::Instance,
        attributes: &[
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 2,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 12,
                shader_location: 3,
            },
            VertexAttribute {
                format: VertexFormat::Float32,
                offset: 28,
                shader_location: 4,
            },
            VertexAttribute {
                format: VertexFormat::Uint32,
                offset: 32,
                shader_location: 5,
            },
        ],
    };
    let render = create_node_pipeline(
        device,
        &pipeline_layout,
        &node_shader,
        &mesh_layout,
        &instance_layout,
        config.color_format,
        config.sample_count,
    );
    let pick = create_node_pipeline(
        device,
        &pipeline_layout,
        &pick_shader,
        &mesh_layout,
        &instance_layout,
        TextureFormat::Rgba8Unorm,
        1,
    );
    NodePipelines {
        render,
        pick,
        mesh_vertices,
        mesh_indices,
        index_count: indices.len() as u32,
    }
}

fn create_node_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    mesh_layout: &VertexBufferLayout<'_>,
    instance_layout: &VertexBufferLayout<'_>,
    format: TextureFormat,
    sample_count: u32,
) -> RenderPipeline {
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("graph-node-pipeline"),
        layout: Some(layout),
        vertex: VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            buffers: &[Some(mesh_layout.clone()), Some(instance_layout.clone())],
        },
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: Some(Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            targets: &[Some(ColorTargetState {
                format,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// Build a UV sphere mesh with `segments` longitude slices and `rings` latitude
/// bands. Returns interleaved `[x, y, z, nx, ny, nz]` vertices and indices.
pub fn sphere_mesh(segments: u32, rings: u32) -> (Vec<[f32; 6]>, Vec<u32>) {
    let segments = segments.max(3);
    let rings = rings.max(2);
    let mut vertices = Vec::with_capacity(((segments + 1) * (rings + 1)) as usize);
    for ring in 0..=rings {
        let phi = PI * ring as f32 / rings as f32;
        let sin_phi = phi.sin();
        let cos_phi = phi.cos();
        for segment in 0..=segments {
            let theta = TAU * segment as f32 / segments as f32;
            let x = sin_phi * theta.cos();
            let y = cos_phi;
            let z = sin_phi * theta.sin();
            vertices.push([x, y, z, x, y, z]);
        }
    }
    let mut indices = Vec::with_capacity((segments * rings * 6) as usize);
    let stride = segments + 1;
    for ring in 0..rings {
        for segment in 0..segments {
            let current = ring * stride + segment;
            let next = current + stride;
            indices.extend_from_slice(&[current, next, current + 1, current + 1, next, next + 1]);
        }
    }
    (vertices, indices)
}

/// Build node instances from a scene, converting colours to linear space.
pub fn node_instances(scene: &Scene) -> Vec<NodeInstance> {
    scene
        .nodes
        .iter()
        .map(|node| {
            let linear = srgb_to_linear(node.color);
            NodeInstance {
                position: [node.position.x, node.position.y, node.position.z],
                color: [linear[0], linear[1], linear[2], node.opacity],
                radius: node.radius,
                index: node.index as u32,
            }
        })
        .collect()
}

/// Create an instance buffer, tolerating an empty instance list.
pub fn create_instance_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    instances: &[NodeInstance],
) -> Buffer {
    let size = (instances.len().max(1) * std::mem::size_of::<NodeInstance>()) as u64;
    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some("graph-node-instances"),
        size,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    if !instances.is_empty() {
        queue.write_buffer(&buffer, 0, bytemuck::cast_slice(instances));
    }
    buffer
}

/// Draw the node instances into an active render pass.
pub fn draw_nodes<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    pipelines: &'a NodePipelines,
    instances: &'a Buffer,
    count: u32,
) {
    if count == 0 {
        return;
    }
    pass.set_pipeline(&pipelines.render);
    pass.set_vertex_buffer(0, pipelines.mesh_vertices.slice(..));
    pass.set_vertex_buffer(1, instances.slice(..));
    pass.set_index_buffer(pipelines.mesh_indices.slice(..), wgpu::IndexFormat::Uint32);
    pass.draw_indexed(0..pipelines.index_count, 0, 0..count);
}

/// Draw the pick pass.
pub fn draw_pick<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    pipelines: &'a NodePipelines,
    instances: &'a Buffer,
    count: u32,
) {
    if count == 0 {
        return;
    }
    pass.set_pipeline(&pipelines.pick);
    pass.set_vertex_buffer(0, pipelines.mesh_vertices.slice(..));
    pass.set_vertex_buffer(1, instances.slice(..));
    pass.set_index_buffer(pipelines.mesh_indices.slice(..), wgpu::IndexFormat::Uint32);
    pass.draw_indexed(0..pipelines.index_count, 0, 0..count);
}

/// Build a uniform buffer payload (kept here so the renderer can reuse it).
pub fn uniforms_for(camera: &crate::camera::Camera, width: u32, height: u32) -> Uniforms {
    Uniforms::new(camera, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_mesh_is_bounded_and_indexed() {
        let (vertices, indices) = sphere_mesh(8, 6);
        assert_eq!(vertices.len(), 9 * 7);
        assert_eq!(indices.len(), 8 * 6 * 6);
        assert!(vertices.iter().all(|vertex| (vertex[0].powi(2)
            + vertex[1].powi(2)
            + vertex[2].powi(2)
            - 1.0)
            .abs()
            < 1e-4));
        assert!(indices
            .iter()
            .all(|index| (*index as usize) < vertices.len()));
    }
}
