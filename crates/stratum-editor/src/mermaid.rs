//! Mermaid source extraction and SVG caching.
//!
//! The shipped editor renders Mermaid diagrams in the browser. The native shell
//! renders the same fenced source with `mermaid-rs-renderer` (pure Rust, SVG
//! only) and caches the SVG. Invalid diagrams are reported as
//! [`pkm_core::PkmError::Render`] and never panic.
//!
//! # Cache
//!
//! [`MermaidCache`] is a bounded LRU (32 entries by default) keyed by a hash of
//! the diagram source plus the theme name, because the theme changes the SVG.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use pkm_core::{PkmError, PkmResult};

/// Extract a Mermaid diagram body from fenced block content.
///
/// Returns `None` when the content is not a ```mermaid fence.
pub fn mermaid_source(block_content: &str) -> Option<&str> {
    let trimmed = block_content.trim();
    let mut lines = trimmed.lines();
    let opening = lines.next()?;
    let language = opening.strip_prefix("```")?.trim();
    if !language.eq_ignore_ascii_case("mermaid") {
        return None;
    }
    let body: Vec<&str> = lines.collect();
    let last = body.last()?;
    if last.trim() != "```" {
        return None;
    }
    let inner_start = opening.len() + 1;
    let inner_end = trimmed.len() - last.len() - 1;
    if inner_start > inner_end {
        return Some("");
    }
    Some(trimmed[inner_start..inner_end].trim_matches('\n'))
}

/// A bounded LRU cache of rendered Mermaid SVGs.
#[derive(Debug)]
pub struct MermaidCache {
    entries: HashMap<String, Arc<String>>,
    order: Vec<String>,
    capacity: usize,
    hits: u64,
    misses: u64,
}

impl Default for MermaidCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MermaidCache {
    /// Create a cache with the default bound of 32 diagrams.
    pub fn new() -> Self {
        Self::with_capacity(32)
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

    /// Number of cached diagrams.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Render `source` for `theme`, returning a cached SVG when the same source
    /// and theme were rendered before.
    ///
    /// # Errors
    ///
    /// Invalid Mermaid syntax is reported as [`PkmError::Render`].
    pub fn render(&mut self, source: &str, theme: &str) -> PkmResult<Arc<String>> {
        let key = cache_key(source, theme);
        if let Some(svg) = self.entries.get(&key).cloned() {
            self.hits += 1;
            self.touch(&key);
            return Ok(svg);
        }
        self.misses += 1;
        let svg = Arc::new(render_svg(source, theme)?);
        self.insert(key, Arc::clone(&svg));
        Ok(svg)
    }

    fn touch(&mut self, key: &str) {
        if let Some(position) = self.order.iter().position(|entry| entry == key) {
            let entry = self.order.remove(position);
            self.order.push(entry);
        }
    }

    fn insert(&mut self, key: String, svg: Arc<String>) {
        while self.entries.len() >= self.capacity {
            let Some(oldest) = self.order.first().cloned() else {
                break;
            };
            self.order.remove(0);
            self.entries.remove(&oldest);
        }
        self.order.push(key.clone());
        self.entries.insert(key, svg);
    }
}

fn cache_key(source: &str, theme: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    format!("{:016x}|{}", hasher.finish(), theme)
}

fn render_svg(source: &str, theme: &str) -> PkmResult<String> {
    let options = mermaid_rs_renderer::RenderOptions {
        theme: match theme {
            "dark" => mermaid_rs_renderer::Theme::dark(),
            _ => mermaid_rs_renderer::Theme::modern(),
        },
        ..Default::default()
    };
    mermaid_rs_renderer::render_strict(source, options)
        .map_err(|error| PkmError::Render(format!("invalid Mermaid: {error}")))
}

#[cfg(test)]
mod tests;
