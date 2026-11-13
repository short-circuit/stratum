//! Per-frame render measurements for the native block editor.
//!
//! [`EditorMetrics`] collects the row-materialization and asset-rendering
//! durations the 10k perf capture consumes. The struct is UI-free so the perf
//! binary and the window-free tests can read it without a gpui context.
//!
//! Row materialization and asset rendering are deliberately separate figures:
//! math, Mermaid and attachment thumbnails are budgeted to one new asset per
//! frame and cached afterwards, so a single decode may exceed the per-frame
//! work budget without stalling scrolling.

/// Per-frame render measurements used by the 10k perf capture.
#[derive(Debug, Default)]
pub struct EditorMetrics {
    render_ms: Vec<f64>,
    assets_ms: Vec<f64>,
    rows_rendered: usize,
    frames: usize,
}

impl EditorMetrics {
    /// Record one row-materialization pass.
    pub fn record(&mut self, elapsed_ms: f64, rows: usize) {
        self.frames += 1;
        self.rows_rendered = rows;
        self.render_ms.push(elapsed_ms);
    }

    /// Record the math/Mermaid/thumbnail asset-rendering time of one pass.
    ///
    /// Asset rendering is intentionally not part of the row-materialization
    /// figure: it is budgeted to one new asset per frame and cached afterwards,
    /// so a single diagram or thumbnail render may exceed the per-frame work
    /// budget without stalling scrolling.
    pub fn record_assets(&mut self, elapsed_ms: f64) {
        self.assets_ms.push(elapsed_ms);
    }

    /// All recorded row-materialization durations in milliseconds.
    pub fn render_ms(&self) -> &[f64] {
        &self.render_ms
    }

    /// All recorded asset-rendering durations in milliseconds.
    pub fn assets_ms(&self) -> &[f64] {
        &self.assets_ms
    }

    /// Rows materialized in the last pass.
    pub fn rows_rendered(&self) -> usize {
        self.rows_rendered
    }

    /// Number of recorded passes.
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// The last recorded duration.
    pub fn last_render_ms(&self) -> f64 {
        self.render_ms.last().copied().unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    #[test]
    fn metrics_record_separate_figures() {
        let mut metrics = EditorMetrics::default();
        metrics.record(1.5, 4);
        metrics.record_assets(0.25);
        metrics.record(2.0, 2);
        assert_eq!(metrics.frames(), 2);
        assert_eq!(metrics.rows_rendered(), 2);
        assert_eq!(metrics.render_ms(), &[1.5, 2.0]);
        assert_eq!(metrics.assets_ms(), &[0.25]);
        assert_eq!(metrics.last_render_ms(), 2.0);
    }
}
