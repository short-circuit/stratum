//! Signed-distance-field glyph atlas for graph labels.
//!
//! Labels use the embedded Inter Regular face. Each ASCII glyph is rasterized
//! with `ab_glyph`, converted to a signed distance field with a Felzenszwalb
//! one-dimensional Euclidean distance transform, and shelf-packed into a
//! 2048x2048 `R8Unorm` texture. The label shader samples the atlas and applies a
//! narrow smoothstep to keep edges crisp at any zoom level.

use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
use pkm_core::{PkmError, PkmResult};
use std::collections::HashMap;

/// Embedded Inter Regular face (copied from `stratum-ui/assets/fonts`).
pub const FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
/// Atlas edge length.
pub const ATLAS_SIZE: u32 = 2048;
/// Rasterization size in pixels.
pub const FONT_PX: f32 = 48.0;
/// SDF spread in pixels.
const SDF_SPREAD: f32 = 6.0;
/// Atlas padding around each glyph.
const PADDING: u32 = 2;

/// A packed glyph entry.
#[derive(Debug, Clone)]
pub struct Glyph {
    /// Normalized u0.
    pub u0: f32,
    /// Normalized v0.
    pub v0: f32,
    /// Normalized u1.
    pub u1: f32,
    /// Normalized v1.
    pub v1: f32,
    /// Bitmap width in pixels.
    pub width: u32,
    /// Bitmap height in pixels.
    pub height: u32,
    /// Horizontal advance in pixels.
    pub advance: f32,
    /// Left bearing in pixels.
    pub bearing_x: f32,
    /// Top bearing in pixels (negative above the baseline).
    pub bearing_y: f32,
}

/// A laid-out glyph quad in pixels relative to the baseline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphQuad {
    /// Left offset in pixels.
    pub x: f32,
    /// Top offset in pixels (positive up).
    pub y: f32,
    /// Quad width in pixels.
    pub width: f32,
    /// Quad height in pixels.
    pub height: f32,
    /// Atlas u0.
    pub u0: f32,
    /// Atlas v0.
    pub v0: f32,
    /// Atlas u1.
    pub u1: f32,
    /// Atlas v1.
    pub v1: f32,
}

/// A packed SDF atlas.
#[derive(Debug, Clone)]
pub struct GlyphAtlas {
    glyphs: HashMap<char, Glyph>,
    data: Vec<u8>,
    fallback: char,
}

impl GlyphAtlas {
    /// Build the atlas from a TrueType face.
    pub fn build(font_bytes: &[u8]) -> PkmResult<Self> {
        let font = FontRef::try_from_slice(font_bytes)
            .map_err(|error| PkmError::Render(format!("label font parse failed: {error}")))?;
        let scale = PxScale::from(FONT_PX);
        let scaled = font.as_scaled(scale);
        let mut glyphs = HashMap::new();
        let mut data = vec![0_u8; (ATLAS_SIZE * ATLAS_SIZE) as usize];
        let mut cursor_x = PADDING;
        let mut cursor_y = PADDING;
        let mut row_height = 0_u32;
        for code in 32_u32..=126 {
            let Some(character) = char::from_u32(code) else {
                continue;
            };
            let glyph_id = font.glyph_id(character);
            let advance = scaled.h_advance(glyph_id);
            let glyph = glyph_id.with_scale_and_position(scale, point(0.0, 0.0));
            let Some(outlined) = font.outline_glyph(glyph) else {
                glyphs.insert(
                    character,
                    Glyph {
                        u0: 0.0,
                        v0: 0.0,
                        u1: 0.0,
                        v1: 0.0,
                        width: 0,
                        height: 0,
                        advance,
                        bearing_x: 0.0,
                        bearing_y: 0.0,
                    },
                );
                continue;
            };
            let bounds = outlined.px_bounds();
            let width = bounds.width().ceil().max(0.0) as u32;
            let height = bounds.height().ceil().max(0.0) as u32;
            if width == 0 || height == 0 {
                glyphs.insert(
                    character,
                    Glyph {
                        u0: 0.0,
                        v0: 0.0,
                        u1: 0.0,
                        v1: 0.0,
                        width: 0,
                        height: 0,
                        advance,
                        bearing_x: bounds.min.x,
                        bearing_y: bounds.min.y,
                    },
                );
                continue;
            }
            if cursor_x + width + PADDING > ATLAS_SIZE {
                cursor_x = PADDING;
                cursor_y += row_height + PADDING;
                row_height = 0;
            }
            if cursor_y + height + PADDING > ATLAS_SIZE {
                // Out of atlas space: keep the glyph but leave it invisible.
                glyphs.insert(
                    character,
                    Glyph {
                        u0: 0.0,
                        v0: 0.0,
                        u1: 0.0,
                        v1: 0.0,
                        width: 0,
                        height: 0,
                        advance,
                        bearing_x: bounds.min.x,
                        bearing_y: bounds.min.y,
                    },
                );
                continue;
            }
            let mut coverage = vec![0.0_f32; (width * height) as usize];
            outlined.draw(|x, y, value| {
                let index = (y * width + x) as usize;
                if index < coverage.len() {
                    coverage[index] = value.max(coverage[index]);
                }
            });
            let sdf = coverage_to_sdf(&coverage, width, height);
            for row in 0..height {
                let destination = ((cursor_y + row) * ATLAS_SIZE + cursor_x) as usize;
                let source = (row * width) as usize;
                data[destination..destination + width as usize]
                    .copy_from_slice(&sdf[source..source + width as usize]);
            }
            glyphs.insert(
                character,
                Glyph {
                    u0: cursor_x as f32 / ATLAS_SIZE as f32,
                    v0: cursor_y as f32 / ATLAS_SIZE as f32,
                    u1: (cursor_x + width) as f32 / ATLAS_SIZE as f32,
                    v1: (cursor_y + height) as f32 / ATLAS_SIZE as f32,
                    width,
                    height,
                    advance,
                    bearing_x: bounds.min.x,
                    bearing_y: bounds.min.y,
                },
            );
            cursor_x += width + PADDING;
            row_height = row_height.max(height);
        }
        Ok(Self {
            glyphs,
            data,
            fallback: '?',
        })
    }

    /// Atlas edge length.
    pub fn size(&self) -> u32 {
        ATLAS_SIZE
    }

    /// Raw R8 SDF data.
    pub fn texture_data(&self) -> &[u8] {
        &self.data
    }

    /// Rasterization size used to build the atlas.
    pub fn font_px(&self) -> f32 {
        FONT_PX
    }

    /// Number of packed glyph entries.
    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    /// Lay out `text` into baseline-relative glyph quads in pixels.
    pub fn layout(&self, text: &str) -> Vec<GlyphQuad> {
        let mut quads = Vec::new();
        let mut pen = 0.0_f32;
        for character in text.chars() {
            let glyph = self
                .glyphs
                .get(&character)
                .or_else(|| self.glyphs.get(&self.fallback));
            let Some(glyph) = glyph else {
                continue;
            };
            if glyph.width > 0 && glyph.height > 0 {
                quads.push(GlyphQuad {
                    x: pen + glyph.bearing_x,
                    y: -glyph.bearing_y,
                    width: glyph.width as f32,
                    height: glyph.height as f32,
                    u0: glyph.u0,
                    v0: glyph.v0,
                    u1: glyph.u1,
                    v1: glyph.v1,
                });
            }
            pen += glyph.advance;
        }
        quads
    }
}

/// Convert a coverage bitmap to an 8-bit SDF.
fn coverage_to_sdf(coverage: &[f32], width: u32, height: u32) -> Vec<u8> {
    // Pad with transparent pixels so the outside distance transform always has a
    // finite source at the bitmap border.
    let pad = 2_usize;
    let padded_width = width as usize + pad * 2;
    let padded_height = height as usize + pad * 2;
    let mut inside = vec![0.0_f64; padded_width * padded_height];
    let mut outside = vec![0.0_f64; padded_width * padded_height];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let index = (y + pad) * padded_width + (x + pad);
            let value = coverage[y * width as usize + x];
            if value >= 0.5 {
                inside[index] = 0.0;
                outside[index] = f64::INFINITY;
            } else {
                inside[index] = f64::INFINITY;
                outside[index] = 0.0;
            }
        }
    }
    let inside_distance = edt_2d(&inside, padded_width, padded_height);
    let outside_distance = edt_2d(&outside, padded_width, padded_height);
    let mut output = vec![0_u8; coverage.len()];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let source = (y + pad) * padded_width + (x + pad);
            let signed = outside_distance[source].sqrt() - inside_distance[source].sqrt();
            let value = 128.0 + signed * (127.0 / f64::from(SDF_SPREAD));
            output[y * width as usize + x] = value.clamp(0.0, 255.0).round() as u8;
        }
    }
    output
}

/// 2D squared Euclidean distance transform (Felzenszwalb & Huttenlocher).
fn edt_2d(field: &[f64], width: usize, height: usize) -> Vec<f64> {
    let mut buffer = vec![0.0_f64; width * height];
    for x in 0..width {
        let column: Vec<f64> = (0..height).map(|y| field[y * width + x]).collect();
        let transformed = edt_1d(&column);
        for y in 0..height {
            buffer[y * width + x] = transformed[y];
        }
    }
    let mut output = vec![0.0_f64; width * height];
    for y in 0..height {
        let row: Vec<f64> = (0..width).map(|x| buffer[y * width + x]).collect();
        let transformed = edt_1d(&row);
        for x in 0..width {
            output[y * width + x] = transformed[x];
        }
    }
    output
}

fn edt_1d(field: &[f64]) -> Vec<f64> {
    let length = field.len();
    if length == 0 {
        return Vec::new();
    }
    if !field.iter().any(|value| value.is_finite()) {
        return vec![f64::INFINITY; length];
    }
    let mut output = vec![0.0_f64; length];
    let mut vertices = vec![0_usize; length];
    let mut boundaries = vec![0.0_f64; length + 1];
    let mut k = 0_usize;
    vertices[0] = 0;
    boundaries[0] = f64::NEG_INFINITY;
    boundaries[1] = f64::INFINITY;
    for q in 1..length {
        let mut intersection = ((field[q] + (q * q) as f64)
            - (field[vertices[k]] + (vertices[k] * vertices[k]) as f64))
            / (2.0 * (q as f64 - vertices[k] as f64));
        while intersection <= boundaries[k] {
            if k == 0 {
                break;
            }
            k -= 1;
            intersection = ((field[q] + (q * q) as f64)
                - (field[vertices[k]] + (vertices[k] * vertices[k]) as f64))
                / (2.0 * (q as f64 - vertices[k] as f64));
        }
        k += 1;
        vertices[k] = q;
        boundaries[k] = intersection;
        boundaries[k + 1] = f64::INFINITY;
    }
    k = 0;
    for (q, out) in output.iter_mut().enumerate() {
        while boundaries[k + 1] < q as f64 {
            k += 1;
        }
        let delta = q as f64 - vertices[k] as f64;
        *out = delta * delta + field[vertices[k]];
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_builds_from_the_embedded_font() {
        let atlas = GlyphAtlas::build(FONT_BYTES).expect("atlas");
        assert_eq!(atlas.size(), ATLAS_SIZE);
        assert_eq!(
            atlas.texture_data().len(),
            (ATLAS_SIZE * ATLAS_SIZE) as usize
        );
        assert!(atlas.glyph_count() > 80);
        let quads = atlas.layout("Note 1");
        assert_eq!(quads.len(), 5);
        assert!(quads.iter().all(|quad| quad.width > 0.0));
        assert!(atlas.texture_data().iter().any(|value| *value != 128));
    }

    #[test]
    fn sdf_has_positive_inside_and_negative_outside() {
        let coverage = vec![0.0, 1.0, 1.0, 0.0];
        let sdf = coverage_to_sdf(&coverage, 2, 2);
        assert!(sdf[1] > 128);
        assert!(sdf[0] < 128);
    }

    #[test]
    fn layout_falls_back_for_unknown_characters() {
        let atlas = GlyphAtlas::build(FONT_BYTES).expect("atlas");
        let quads = atlas.layout("\u{1F600}");
        assert!(!quads.is_empty());
    }
}
