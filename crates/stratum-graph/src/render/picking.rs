//! GPU picking: render node indices into an offscreen `Rgba8Unorm` target and
//! decode the pixel under the cursor.
//!
//! The pick shader writes the node index into the red, green and blue channels
//! and a hit flag into alpha. Decoding is therefore exact: an alpha byte of zero
//! means no node, and the RGB bytes are the little-endian index.

use crate::render::device::{create_pick_target, padded_row_bytes, read_texture, GpuContext};
use crate::render::nodes::{draw_pick, NodePipelines};
use pkm_core::{PkmError, PkmResult};
use wgpu::{
    BindGroup, CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, TextureFormat,
};

/// Decode a picked node index from a padded readback buffer.
///
/// Returns `None` when the pixel is outside the image or the alpha byte is zero.
pub fn decode_pick(bytes: &[u8], width: u32, height: u32, x: u32, y: u32) -> Option<usize> {
    if x >= width || y >= height {
        return None;
    }
    let padded = padded_row_bytes(width);
    let offset = (y as usize) * (padded as usize) + (x as usize) * 4;
    let pixel = bytes.get(offset..offset + 4)?;
    if pixel[3] == 0 {
        return None;
    }
    Some(u32::from(pixel[0]) | (u32::from(pixel[1]) << 8) | (u32::from(pixel[2]) << 16))
        .map(|index| index as usize)
}

/// Render the pick pass and decode the pixel at `(x, y)`.
///
/// `x` and `y` are in pixels with the origin at the top-left corner.
#[allow(clippy::too_many_arguments)]
pub fn pick_gpu(
    context: &GpuContext,
    pipelines: &NodePipelines,
    instances: &wgpu::Buffer,
    count: u32,
    uniform_bind_group: &BindGroup,
    width: u32,
    height: u32,
    x: f32,
    y: f32,
) -> PkmResult<Option<usize>> {
    let target = create_pick_target(
        &context.device,
        width,
        height,
        TextureFormat::Rgba8Unorm,
        TextureFormat::Depth32Float,
    );
    let mut encoder = context
        .device
        .create_command_encoder(&CommandEncoderDescriptor {
            label: Some("graph-pick-encoder"),
        });
    {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("graph-pick-pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &target.color_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &target.depth_view,
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
        pass.set_bind_group(0, uniform_bind_group, &[]);
        draw_pick(&mut pass, pipelines, instances, count);
    }
    context.queue.submit(Some(encoder.finish()));
    let bytes = read_texture(context, &target.color, width, height)?;
    let px = x.max(0.0).floor() as u32;
    let py = y.max(0.0).floor() as u32;
    Ok(decode_pick(&bytes, width, height, px, py))
}

/// CPU pick fallback used by the software renderer and tests.
pub fn pick_cpu(
    scene: &crate::scene::Scene,
    camera: &crate::camera::Camera,
    x: f32,
    y: f32,
    width: u32,
    height: u32,
) -> Option<usize> {
    scene.pick_cpu(camera, x, y, width, height)
}

/// Validate that a decoded index fits the node count.
pub fn validate_pick(index: Option<usize>, node_count: usize) -> PkmResult<Option<usize>> {
    match index {
        Some(index) if index >= node_count => Err(PkmError::Render(format!(
            "picked node index {index} is out of range for {node_count} nodes"
        ))),
        other => Ok(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_reads_little_endian_index_and_alpha() {
        let width = 2;
        let height = 2;
        let padded = padded_row_bytes(width) as usize;
        let mut bytes = vec![0_u8; padded * height as usize];
        let offset = padded + 4;
        bytes[offset] = 7;
        bytes[offset + 1] = 1;
        bytes[offset + 2] = 0;
        bytes[offset + 3] = 255;
        assert_eq!(decode_pick(&bytes, width, height, 1, 1), Some(263));
        assert_eq!(decode_pick(&bytes, width, height, 0, 0), None);
        assert_eq!(decode_pick(&bytes, width, height, 9, 0), None);
    }

    #[test]
    fn validate_rejects_out_of_range() {
        assert_eq!(validate_pick(Some(2), 3).expect("valid"), Some(2));
        assert!(validate_pick(Some(3), 3).is_err());
        assert_eq!(validate_pick(None, 3).expect("empty"), None);
    }
}
