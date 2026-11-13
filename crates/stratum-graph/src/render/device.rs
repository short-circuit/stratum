//! Headless wgpu device setup, render targets and buffer readback.
//!
//! The renderer never creates a surface: it renders into an offscreen
//! multisampled colour target, resolves into an `Rgba8UnormSrgb` texture, and
//! copies the resolved image into a padded readback buffer. The same helpers
//! create the single-sample pick target used by [`super::picking`].

use crate::camera::Camera;
use crate::math::Mat4;
use bytemuck::{Pod, Zeroable};
use pkm_core::{PkmError, PkmResult};
use std::num::NonZeroU64;
use std::sync::mpsc;
use wgpu::util::DeviceExt as _;
use wgpu::{
    BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    Buffer, BufferBindingType, BufferDescriptor, BufferUsages, CommandEncoderDescriptor,
    DeviceDescriptor, Extent3d, InstanceDescriptor, Origin3d, PollType, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo,
    Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor, TextureViewDimension,
};

/// A headless GPU context.
pub struct GpuContext {
    /// wgpu device.
    pub device: wgpu::Device,
    /// wgpu queue.
    pub queue: wgpu::Queue,
    /// Adapter name for the golden report.
    pub adapter_name: String,
}

impl std::fmt::Debug for GpuContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GpuContext")
            .field("adapter_name", &self.adapter_name)
            .finish_non_exhaustive()
    }
}

/// Uniform data shared by all graph pipelines.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Uniforms {
    /// Combined view-projection matrix.
    pub view_proj: [f32; 16],
    /// Camera position (`w` = 1).
    pub camera_pos: [f32; 4],
    /// Camera right basis (`w` = 0).
    pub camera_right: [f32; 4],
    /// Camera up basis (`w` = 0).
    pub camera_up: [f32; 4],
    /// Viewport `(width, height, 0, 0)`.
    pub viewport: [f32; 4],
}

impl Uniforms {
    /// Build uniforms for a camera and viewport.
    pub fn new(camera: &Camera, width: u32, height: u32) -> Self {
        let view_proj = camera.view_projection(width, height);
        let (right, up, _) = camera.basis();
        Self {
            view_proj: view_proj.m,
            camera_pos: [camera.position.x, camera.position.y, camera.position.z, 1.0],
            camera_right: [right.x, right.y, right.z, 0.0],
            camera_up: [up.x, up.y, up.z, 0.0],
            viewport: [width as f32, height as f32, 0.0, 0.0],
        }
    }
}

/// Initialise a headless GPU context, returning an error when no adapter exists.
pub fn create_gpu() -> PkmResult<GpuContext> {
    let instance = wgpu::Instance::new(InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    }))
    .map_err(|error| PkmError::Render(format!("no graph render adapter: {error}")))?;
    let adapter_name = adapter.get_info().name;
    let (device, queue) = pollster::block_on(adapter.request_device(&DeviceDescriptor {
        label: Some("stratum-graph"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }))
    .map_err(|error| PkmError::Render(format!("graph device request failed: {error}")))?;
    Ok(GpuContext {
        device,
        queue,
        adapter_name,
    })
}

/// Create the uniform bind-group layout used by every pipeline.
pub fn create_uniform_layout(device: &wgpu::Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("graph-uniform-layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX_FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: NonZeroU64::new(std::mem::size_of::<Uniforms>() as u64),
            },
            count: None,
        }],
    })
}

/// Create the uniform buffer and bind group for a frame.
pub fn create_uniform_bind_group(
    device: &wgpu::Device,
    layout: &BindGroupLayout,
    uniforms: &Uniforms,
) -> (Buffer, BindGroup) {
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("graph-uniforms"),
        contents: bytemuck::bytes_of(uniforms),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    let size = NonZeroU64::new(std::mem::size_of::<Uniforms>() as u64).unwrap_or(NonZeroU64::MIN);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("graph-uniforms-bind"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: Some(size),
            }),
        }],
    });
    (buffer, bind_group)
}

/// Create a pipeline layout with one bind group.
pub fn create_pipeline_layout(
    device: &wgpu::Device,
    layout: &BindGroupLayout,
) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("graph-pipeline-layout"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    })
}

/// Create a shader module from WGSL source.
pub fn create_shader(device: &wgpu::Device, label: &str, source: &str) -> wgpu::ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label: Some(label),
        source: ShaderSource::Wgsl(source.into()),
    })
}

/// Render targets for one frame.
pub struct ColorTargets {
    /// Multisampled colour attachment.
    pub color: Texture,
    /// Multisampled colour view.
    pub color_view: TextureView,
    /// Resolved single-sample colour texture.
    pub resolve: Texture,
    /// Resolved colour view.
    pub resolve_view: TextureView,
    /// Depth attachment.
    pub depth: Texture,
    /// Depth view.
    pub depth_view: TextureView,
}

/// Create the multisampled colour, resolve and depth targets.
pub fn create_color_targets(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    sample_count: u32,
    color_format: TextureFormat,
    depth_format: TextureFormat,
) -> ColorTargets {
    let size = Extent3d {
        width: width.max(1),
        height: height.max(1),
        depth_or_array_layers: 1,
    };
    let color = device.create_texture(&TextureDescriptor {
        label: Some("graph-color-msaa"),
        size,
        mip_level_count: 1,
        sample_count,
        dimension: TextureDimension::D2,
        format: color_format,
        usage: TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let resolve = device.create_texture(&TextureDescriptor {
        label: Some("graph-color-resolve"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: color_format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth = device.create_texture(&TextureDescriptor {
        label: Some("graph-depth"),
        size,
        mip_level_count: 1,
        sample_count,
        dimension: TextureDimension::D2,
        format: depth_format,
        usage: TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    ColorTargets {
        color_view: color.create_view(&view_descriptor(
            "graph-color-msaa-view",
            color_format,
            false,
        )),
        color,
        resolve_view: resolve.create_view(&view_descriptor(
            "graph-color-resolve-view",
            color_format,
            true,
        )),
        resolve,
        depth_view: depth.create_view(&view_descriptor("graph-depth-view", depth_format, false)),
        depth,
    }
}

/// A single-sample pick target.
pub struct PickTarget {
    /// Colour target holding the encoded index.
    pub color: Texture,
    /// Colour view.
    pub color_view: TextureView,
    /// Depth target.
    pub depth: Texture,
    /// Depth view.
    pub depth_view: TextureView,
}

/// Create a single-sample pick target.
pub fn create_pick_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    color_format: TextureFormat,
    depth_format: TextureFormat,
) -> PickTarget {
    let size = Extent3d {
        width: width.max(1),
        height: height.max(1),
        depth_or_array_layers: 1,
    };
    let color = device.create_texture(&TextureDescriptor {
        label: Some("graph-pick-color"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: color_format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth = device.create_texture(&TextureDescriptor {
        label: Some("graph-pick-depth"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: depth_format,
        usage: TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    PickTarget {
        color_view: color.create_view(&view_descriptor(
            "graph-pick-color-view",
            color_format,
            true,
        )),
        color,
        depth_view: depth.create_view(&view_descriptor(
            "graph-pick-depth-view",
            depth_format,
            false,
        )),
        depth,
    }
}

fn view_descriptor(
    label: &str,
    format: TextureFormat,
    copy_src: bool,
) -> TextureViewDescriptor<'_> {
    TextureViewDescriptor {
        label: Some(label),
        format: Some(format),
        dimension: Some(TextureViewDimension::D2),
        usage: Some(if copy_src {
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC
        } else {
            TextureUsages::RENDER_ATTACHMENT
        }),
        aspect: TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: Some(1),
        base_array_layer: 0,
        array_layer_count: Some(1),
    }
}

/// Copy a texture into a padded readback buffer and return the raw bytes.
///
/// Rows are padded to the wgpu 256-byte alignment requirement.
pub fn read_texture(
    context: &GpuContext,
    texture: &Texture,
    width: u32,
    height: u32,
) -> PkmResult<Vec<u8>> {
    let padded_bytes_per_row = padded_row_bytes(width);
    let buffer_size = u64::from(padded_bytes_per_row) * u64::from(height);
    let buffer = context.device.create_buffer(&BufferDescriptor {
        label: Some("graph-readback"),
        size: buffer_size,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = context
        .device
        .create_command_encoder(&CommandEncoderDescriptor {
            label: Some("graph-readback-encoder"),
        });
    encoder.copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    context.queue.submit(Some(encoder.finish()));
    let (sender, receiver) = mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result.is_ok());
        });
    context
        .device
        .poll(PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .map_err(|error| PkmError::Render(format!("graph device poll failed: {error}")))?;
    let mapped = receiver
        .recv()
        .map_err(|error| PkmError::Render(format!("graph readback channel failed: {error}")))?;
    if !mapped {
        return Err(PkmError::Render("graph buffer mapping failed".to_string()));
    }
    let range = buffer
        .slice(..)
        .get_mapped_range()
        .map_err(|error| PkmError::Render(format!("graph mapped range failed: {error}")))?;
    let bytes = range.to_vec();
    drop(range);
    buffer.unmap();
    Ok(bytes)
}

/// Round a row of `width` RGBA pixels up to wgpu's 256-byte alignment.
pub fn padded_row_bytes(width: u32) -> u32 {
    let unpadded = width.saturating_mul(4);
    unpadded.div_ceil(256).saturating_mul(256)
}

/// Convert a scene background into a linear wgpu clear colour.
pub fn clear_color(background: [u8; 4]) -> wgpu::Color {
    let linear = crate::data::srgb_to_linear([background[0], background[1], background[2]]);
    wgpu::Color {
        r: f64::from(linear[0]),
        g: f64::from(linear[1]),
        b: f64::from(linear[2]),
        a: f64::from(background[3]) / 255.0,
    }
}

/// Convenience alias for a view-projection matrix from a camera.
pub fn camera_matrix(camera: &Camera, width: u32, height: u32) -> Mat4 {
    camera.view_projection(width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padded_rows_are_aligned() {
        assert_eq!(padded_row_bytes(1), 256);
        assert_eq!(padded_row_bytes(64), 256);
        assert_eq!(padded_row_bytes(65), 512);
        assert_eq!(padded_row_bytes(1280), 5120);
        assert_eq!(padded_row_bytes(1280) % 256, 0);
    }

    #[test]
    fn uniforms_match_the_shader_layout_size() {
        assert_eq!(std::mem::size_of::<Uniforms>(), 128);
        let camera = Camera::default();
        let uniforms = Uniforms::new(&camera, 1280, 720);
        assert_eq!(uniforms.viewport[0], 1280.0);
        assert!(uniforms.view_proj.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn clear_colour_is_linear() {
        let color = clear_color([255, 255, 255, 255]);
        assert!((color.r - 1.0).abs() < 1e-6);
        assert!((color.a - 1.0).abs() < 1e-6);
    }
}
