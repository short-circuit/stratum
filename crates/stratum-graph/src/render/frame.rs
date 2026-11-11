//! Render configuration and frame readback types shared by both backends.
//!
//! Extracted from [`super`] so the renderer module stays inside the repository
//! file-length limit. [`RenderConfig`] carries the target dimensions and
//! pipeline formats; [`Frame`] is the tightly packed (or padded) RGBA result
//! that hosts can encode as PNG or hand to a windowing system.

use image::ImageEncoder as _;
use pkm_core::{PkmError, PkmResult};
use std::path::Path;
use wgpu::TextureFormat;

use crate::render::device::padded_row_bytes;
use crate::scene::SceneOptions;

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
