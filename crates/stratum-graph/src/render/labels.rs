//! Label rendering from the signed-distance-field glyph atlas.
//!
//! Labels are billboarded glyph quads anchored at the node position. Text is
//! centred on the anchor and scaled from the atlas raster size to the requested
//! world-space label height, matching the shipped `SpriteText` sizing.

use crate::data::srgb_to_linear;
use crate::render::device::create_shader;
use crate::render::sdf::GlyphAtlas;
use crate::render::RenderConfig;
use crate::scene::Scene;
use bytemuck::{Pod, Zeroable};
use pkm_core::PkmResult;
use wgpu::{
    BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BlendComponent, BlendFactor, BlendState, Buffer, BufferAddress, BufferDescriptor, BufferUsages,
    ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, Extent3d, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
    RenderPipeline, RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor, ShaderStages,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState, VertexStepMode,
};

/// A single label vertex (one corner of a glyph quad).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LabelVertex {
    /// World-space anchor.
    pub anchor: [f32; 3],
    /// Camera-basis offset in world units.
    pub offset: [f32; 2],
    /// Atlas UV.
    pub uv: [f32; 2],
    /// Linear RGBA colour.
    pub color: [f32; 4],
}

/// Label pipeline and the atlas bind group it samples.
pub struct LabelPipelines {
    /// Visible label pipeline.
    pub pipeline: RenderPipeline,
    /// Bind group holding the atlas texture and sampler.
    pub atlas_bind_group: BindGroup,
}

/// Build the label pipeline and upload the SDF atlas.
pub fn create_label_pipelines(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    uniform_layout: &BindGroupLayout,
    atlas: &GlyphAtlas,
    config: &RenderConfig,
) -> PkmResult<LabelPipelines> {
    let atlas_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("graph-label-atlas-layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let atlas_size = atlas.size();
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("graph-label-atlas"),
        size: Extent3d {
            width: atlas_size,
            height: atlas_size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::R8Unorm,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        atlas.texture_data(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(atlas_size),
            rows_per_image: Some(atlas_size),
        },
        Extent3d {
            width: atlas_size,
            height: atlas_size,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&TextureViewDescriptor {
        label: Some("graph-label-atlas-view"),
        format: Some(TextureFormat::R8Unorm),
        dimension: Some(TextureViewDimension::D2),
        usage: Some(TextureUsages::TEXTURE_BINDING),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: Some(1),
        base_array_layer: 0,
        array_layer_count: Some(1),
    });
    let sampler = device.create_sampler(&SamplerDescriptor {
        label: Some("graph-label-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: 1.0,
        compare: None,
        anisotropy_clamp: 1,
        border_color: None,
    });
    let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("graph-label-atlas-bind"),
        layout: &atlas_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("graph-label-pipeline-layout"),
        bind_group_layouts: &[Some(uniform_layout), Some(&atlas_layout)],
        immediate_size: 0,
    });
    let shader = create_shader(
        device,
        "graph-label-shader",
        include_str!("shaders/label.wgsl"),
    );
    let vertex_layout = VertexBufferLayout {
        array_stride: std::mem::size_of::<LabelVertex>() as BufferAddress,
        step_mode: VertexStepMode::Vertex,
        attributes: &[
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 12,
                shader_location: 1,
            },
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 20,
                shader_location: 2,
            },
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 28,
                shader_location: 3,
            },
        ],
    };
    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("graph-label-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            buffers: &[Some(vertex_layout)],
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
    Ok(LabelPipelines {
        pipeline,
        atlas_bind_group,
    })
}

/// Build label vertices for every scene label.
pub fn label_vertices(scene: &Scene, atlas: &GlyphAtlas) -> Vec<LabelVertex> {
    let scale = atlas.font_px();
    let mut vertices = Vec::new();
    for label in &scene.labels {
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
        let unit = label.height / scale;
        let linear = srgb_to_linear(label.color);
        let color = [linear[0], linear[1], linear[2], 1.0];
        for quad in quads {
            let left = (quad.x - center_x) * unit;
            let right = (quad.x + quad.width - center_x) * unit;
            let top = (quad.y - center_y) * unit;
            let bottom = (quad.y - quad.height - center_y) * unit;
            let anchor = [label.position.x, label.position.y, label.position.z];
            let corners = [
                ([left, top], [quad.u0, quad.v0]),
                ([right, top], [quad.u1, quad.v0]),
                ([right, bottom], [quad.u1, quad.v1]),
                ([left, top], [quad.u0, quad.v0]),
                ([right, bottom], [quad.u1, quad.v1]),
                ([left, bottom], [quad.u0, quad.v1]),
            ];
            for (offset, uv) in corners {
                vertices.push(LabelVertex {
                    anchor,
                    offset,
                    uv,
                    color,
                });
            }
        }
    }
    vertices
}

/// Create a label vertex buffer, tolerating an empty list.
pub fn create_label_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    vertices: &[LabelVertex],
) -> Buffer {
    let size = (vertices.len().max(1) * std::mem::size_of::<LabelVertex>()) as u64;
    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some("graph-label-vertices"),
        size,
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    if !vertices.is_empty() {
        queue.write_buffer(&buffer, 0, bytemuck::cast_slice(vertices));
    }
    buffer
}

/// Draw label vertices into an active render pass.
pub fn draw_labels<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    pipelines: &'a LabelPipelines,
    vertices: &'a Buffer,
    count: u32,
) {
    if count == 0 {
        return;
    }
    pass.set_pipeline(&pipelines.pipeline);
    pass.set_bind_group(1, &pipelines.atlas_bind_group, &[]);
    pass.set_vertex_buffer(0, vertices.slice(..));
    pass.draw(0..count, 0..1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::sdf::FONT_BYTES;

    #[test]
    fn label_vertices_are_six_per_glyph() {
        let atlas = GlyphAtlas::build(FONT_BYTES).expect("atlas");
        let quads = atlas.layout("Note 1");
        assert_eq!(quads.len(), 5);
        let scene = crate::scene::Scene {
            nodes: Vec::new(),
            links: Vec::new(),
            labels: vec![crate::scene::SceneLabel {
                node: 0,
                text: "Note 1".to_string(),
                position: crate::math::Vec3::ZERO,
                height: 6.0,
                color: [255, 255, 255],
            }],
            background: crate::scene::DEFAULT_BACKGROUND,
            text_color: crate::scene::DEFAULT_TEXT_COLOR,
        };
        let vertices = label_vertices(&scene, &atlas);
        assert_eq!(vertices.len(), 5 * 6);
        assert!(vertices
            .iter()
            .all(|vertex| vertex.anchor == [0.0, 0.0, 0.0]));
    }

    #[test]
    fn empty_labels_produce_no_vertices() {
        let atlas = GlyphAtlas::build(FONT_BYTES).expect("atlas");
        let scene = crate::scene::Scene {
            nodes: Vec::new(),
            links: Vec::new(),
            labels: Vec::new(),
            background: crate::scene::DEFAULT_BACKGROUND,
            text_color: crate::scene::DEFAULT_TEXT_COLOR,
        };
        assert!(label_vertices(&scene, &atlas).is_empty());
    }
}
