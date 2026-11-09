//! Math source extraction and raster caching.
//!
//! The shipped editor renders LaTeX with KaTeX; the native shell renders the
//! same source with the pure-Rust RaTeX pipeline (`ratex-parser` →
//! `ratex-layout` → `ratex-render`) and caches the resulting PNG. Rendering is
//! deliberately kept out of [`crate::render`]: the decoration model stays
//! UI-free and the view only reads an `Arc<MathImage>`.
//!
//! # Cache
//!
//! [`MathCache`] is a bounded LRU (64 entries by default). The key includes the
//! LaTeX source, font size, device pixel ratio and background colour, because
//! every one of those changes the raster bytes. Invalid LaTeX is reported as
//! [`pkm_core::PkmError::Render`] and never panics.

use std::collections::HashMap;
use std::sync::Arc;

use pkm_core::{PkmError, PkmResult};

/// A LaTeX expression found in block content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathSource {
    /// The LaTeX source without delimiters.
    pub latex: String,
    /// Whether the source was display math (`$$…$$` or a ```math fence).
    pub display: bool,
}

/// A rendered math image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathImage {
    /// PNG bytes.
    pub png: Vec<u8>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

/// Options that affect the rendered raster and therefore the cache key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MathOptions {
    /// Font size passed to RaTeX.
    pub font_size: f32,
    /// Device pixel ratio for the raster.
    pub device_pixel_ratio: f32,
    /// RGBA background colour. Alpha 0 renders a transparent background.
    pub background: [f32; 4],
}

impl Default for MathOptions {
    fn default() -> Self {
        Self {
            font_size: 40.0,
            device_pixel_ratio: 1.0,
            background: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

/// Extract a math expression from block content.
///
/// Supported forms are a fenced ```math block, `$$…$$` display math, `$…$`
/// inline math and the first inline `$…$` expression inside otherwise plain
/// content. Returns `None` when the content carries no math.
pub fn math_source(block_content: &str) -> Option<MathSource> {
    let trimmed = block_content.trim();

    if let Some(source) = fenced_math(trimmed) {
        return Some(source);
    }

    if trimmed.len() >= 4 && trimmed.starts_with("$$") && trimmed.ends_with("$$") {
        return Some(MathSource {
            latex: trimmed[2..trimmed.len() - 2].to_string(),
            display: true,
        });
    }

    if trimmed.len() >= 2 && trimmed.starts_with('$') && trimmed.ends_with('$') {
        return Some(MathSource {
            latex: trimmed[1..trimmed.len() - 1].to_string(),
            display: false,
        });
    }

    inline_math(trimmed)
}

/// A bounded LRU cache of rendered math images.
#[derive(Debug)]
pub struct MathCache {
    entries: HashMap<String, Arc<MathImage>>,
    order: Vec<String>,
    capacity: usize,
    hits: u64,
    misses: u64,
}

impl Default for MathCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MathCache {
    /// Create a cache with the default bound of 64 images.
    pub fn new() -> Self {
        Self::with_capacity(64)
    }

    /// Create a cache with an explicit bound. Primarily useful for tests.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: Vec::new(),
            capacity: capacity.max(1),
            hits: 0,
            misses: 0,
        }
    }

    /// Number of cache hits.
    pub fn hits(&self) -> u64 {
        self.hits
    }

    /// Number of cache misses.
    pub fn misses(&self) -> u64 {
        self.misses
    }

    /// Number of cached images.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Render `source`, returning a cached image when the same source and
    /// options were rendered before.
    ///
    /// # Errors
    ///
    /// Invalid LaTeX, layout or rasterisation failures are reported as
    /// [`PkmError::Render`].
    pub fn render(
        &mut self,
        source: &MathSource,
        options: &MathOptions,
    ) -> PkmResult<Arc<MathImage>> {
        let key = cache_key(source, options);
        if let Some(image) = self.entries.get(&key).cloned() {
            self.hits += 1;
            self.touch(&key);
            return Ok(image);
        }
        self.misses += 1;
        let image = Arc::new(render_image(source, options)?);
        self.insert(key, Arc::clone(&image));
        Ok(image)
    }

    fn touch(&mut self, key: &str) {
        if let Some(position) = self.order.iter().position(|entry| entry == key) {
            let entry = self.order.remove(position);
            self.order.push(entry);
        }
    }

    fn insert(&mut self, key: String, image: Arc<MathImage>) {
        while self.entries.len() >= self.capacity {
            let Some(oldest) = self.order.first().cloned() else {
                break;
            };
            self.order.remove(0);
            self.entries.remove(&oldest);
        }
        self.order.push(key.clone());
        self.entries.insert(key, image);
    }
}

fn cache_key(source: &MathSource, options: &MathOptions) -> String {
    // Float bit patterns keep the key exact without relying on `Eq`/`Hash` for
    // f32. Colour is part of the key because the background is rasterised.
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        source.display,
        source.latex,
        options.font_size.to_bits(),
        options.device_pixel_ratio.to_bits(),
        options.background[0].to_bits(),
        options.background[1].to_bits(),
        options.background[2].to_bits(),
    ) + &format!("|{}", options.background[3].to_bits())
}

fn render_image(source: &MathSource, options: &MathOptions) -> PkmResult<MathImage> {
    let ast = ratex_parser::parser::parse(&source.latex)
        .map_err(|error| PkmError::Render(format!("invalid LaTeX: {error}")))?;
    let layout = ratex_layout::layout(&ast, &ratex_layout::LayoutOptions::default());
    let display_list = ratex_layout::to_display_list(&layout);
    let background = ratex_types::color::Color::new(
        options.background[0],
        options.background[1],
        options.background[2],
        options.background[3],
    );
    let png = ratex_render::render_to_png(
        &display_list,
        &ratex_render::RenderOptions {
            font_size: options.font_size,
            padding: 8.0,
            background_color: background,
            font_dir: String::new(),
            device_pixel_ratio: options.device_pixel_ratio,
        },
    )
    .map_err(PkmError::Render)?;

    let (width, height) = png_dimensions(&png)
        .ok_or_else(|| PkmError::Render("RaTeX returned a malformed PNG".into()))?;
    Ok(MathImage { png, width, height })
}

/// Read the IHDR width/height of a PNG byte stream.
fn png_dimensions(png: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if png.len() < 24 || png[..8] != SIGNATURE {
        return None;
    }
    // PNG layout: 8-byte signature, 4-byte chunk length, 4-byte "IHDR",
    // then big-endian width and height.
    let width = u32::from_be_bytes(png[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(png[20..24].try_into().ok()?);
    Some((width, height))
}

fn fenced_math(trimmed: &str) -> Option<MathSource> {
    let mut lines = trimmed.lines();
    let opening = lines.next()?;
    let language = opening.strip_prefix("```")?.trim();
    if !language.eq_ignore_ascii_case("math") {
        return None;
    }
    let body: Vec<&str> = lines.collect();
    let last = body.last()?;
    if last.trim() != "```" {
        return None;
    }
    let inner = body[..body.len() - 1].join("\n");
    Some(MathSource {
        latex: inner,
        display: true,
    })
}

fn inline_math(text: &str) -> Option<MathSource> {
    let mut start = None;
    let mut escaped = false;
    for (index, ch) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch != '$' {
            continue;
        }
        match start {
            None => start = Some(index),
            Some(open) => {
                // `$$` pairs are display math and are handled above only when
                // they span the whole block; skip empty inline pairs.
                if index > open + 1 {
                    return Some(MathSource {
                        latex: text[open + 1..index].to_string(),
                        display: false,
                    });
                }
                start = None;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests;
