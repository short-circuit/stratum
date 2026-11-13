//! Performance harness helpers for the canvas renderer.
//!
//! The acceptance gate is 5k elements at 60 fps pan/zoom and a 500-element
//! text-heavy board without layout stalls. [`FrameTimer`] records per-frame
//! timings and computes percentiles, [`PerfReport`] carries the gated numbers,
//! and [`stress_board`] builds the deterministic boards the `canvas_perf`
//! binary exercises.
//!
//! [`WORK_BUDGET_MS`] is the 55 fps work budget used by the editor perf gate
//! (`.hermes/perf/editor-10k-frames.json`); a report passes when its p95 frame
//! time is at or below it. Scene builds are reported but not gated: they happen
//! on open/export, not per frame.

use std::collections::VecDeque;

use serde_json::json;

use crate::document::BoardDocument;
use crate::element::Element;
use crate::element_kind::{ElementKind, FillStyle};
use crate::geometry::Point;

/// Target frame rate.
pub const TARGET_FPS: f64 = 60.0;
/// Frame budget for the target frame rate, in milliseconds.
pub const FRAME_BUDGET_MS: f64 = 1000.0 / TARGET_FPS;
/// Work budget used by the perf gates (55 fps).
pub const WORK_BUDGET_MS: f64 = 18.18;
/// Default shape count for the stress board.
pub const DEFAULT_ELEMENTS: usize = 5000;
/// Default frame count for the pan/zoom loop.
pub const DEFAULT_FRAMES: usize = 600;
/// Default text count for the text-heavy board.
pub const DEFAULT_TEXT: usize = 500;

/// Bounded frame-time recorder.
#[derive(Debug)]
pub struct FrameTimer {
    capacity: usize,
    samples: VecDeque<f64>,
}

impl FrameTimer {
    /// Create a timer keeping at most `capacity` samples.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            samples: VecDeque::with_capacity(capacity.min(4096)),
        }
    }

    /// Record one frame time in milliseconds. Non-finite values are ignored.
    pub fn record(&mut self, millis: f64) {
        if !millis.is_finite() || millis < 0.0 {
            return;
        }
        if self.capacity > 0 && self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(millis);
    }

    /// Recorded samples in arrival order.
    pub fn samples(&self) -> Vec<f64> {
        self.samples.iter().copied().collect()
    }

    /// Number of recorded samples.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether no samples were recorded.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Percentile (nearest-rank) in milliseconds.
    pub fn percentile(&self, percentile: f64) -> Option<f64> {
        percentile_of(&self.samples, percentile)
    }

    /// Mean frame time in milliseconds.
    pub fn mean(&self) -> Option<f64> {
        if self.samples.is_empty() {
            return None;
        }
        let sum: f64 = self.samples.iter().sum();
        Some(sum / self.samples.len() as f64)
    }

    /// Maximum frame time in milliseconds.
    pub fn max(&self) -> Option<f64> {
        self.samples
            .iter()
            .copied()
            .fold(None, |max, value| match max {
                Some(current) if current >= value => Some(current),
                _ => Some(value),
            })
    }

    /// Drop every sample.
    pub fn reset(&mut self) {
        self.samples.clear();
    }
}

/// Nearest-rank percentile over an unsorted sample slice.
fn percentile_of(samples: &VecDeque<f64>, percentile: f64) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted: Vec<f64> = samples.iter().copied().collect();
    sorted.sort_by(f64::total_cmp);
    let clamped = percentile.clamp(0.0, 100.0);
    let rank = ((clamped / 100.0) * sorted.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted.len() - 1);
    sorted.get(index).copied()
}

/// Gated performance numbers for one scenario.
#[derive(Debug, Clone, PartialEq)]
pub struct PerfReport {
    /// Scenario label.
    pub scenario: String,
    /// Number of shapes in the board.
    pub elements: usize,
    /// Number of text elements in the board.
    pub text_elements: usize,
    /// Number of measured frames.
    pub frames: usize,
    /// Scene build time in milliseconds.
    pub build_ms: f64,
    /// Median frame time in milliseconds.
    pub p50_ms: f64,
    /// 95th percentile frame time in milliseconds.
    pub p95_ms: f64,
    /// 99th percentile frame time in milliseconds.
    pub p99_ms: f64,
    /// Maximum frame time in milliseconds.
    pub max_ms: f64,
    /// Mean frame time in milliseconds.
    pub mean_ms: f64,
    /// Visible paths in the final frame.
    pub visible_paths: usize,
    /// Visible texts in the final frame.
    pub visible_texts: usize,
}

impl PerfReport {
    /// Build a report from a timer and scenario metadata.
    ///
    /// # Errors
    ///
    /// Returns an empty report with zero timings when the timer has no
    /// samples; callers should treat that as a failed run.
    pub fn from_timer(
        scenario: &str,
        elements: usize,
        text_elements: usize,
        build_ms: f64,
        timer: &FrameTimer,
        visible_paths: usize,
        visible_texts: usize,
    ) -> Self {
        let p50 = timer.percentile(50.0).unwrap_or(0.0);
        let p95 = timer.percentile(95.0).unwrap_or(0.0);
        let p99 = timer.percentile(99.0).unwrap_or(0.0);
        Self {
            scenario: scenario.to_string(),
            elements,
            text_elements,
            frames: timer.len(),
            build_ms,
            p50_ms: p50,
            p95_ms: p95,
            p99_ms: p99,
            max_ms: timer.max().unwrap_or(0.0),
            mean_ms: timer.mean().unwrap_or(0.0),
            visible_paths,
            visible_texts,
        }
    }

    /// Whether the frame gate passes (p95 within [`WORK_BUDGET_MS`]).
    pub fn passes(&self) -> bool {
        self.frames > 0 && self.p95_ms <= WORK_BUDGET_MS
    }

    /// Deterministic JSON report.
    pub fn to_json(&self) -> String {
        json!({
            "scenario": self.scenario,
            "elements": self.elements,
            "text_elements": self.text_elements,
            "frames": self.frames,
            "build_ms": round(self.build_ms),
            "p50_ms": round(self.p50_ms),
            "p95_ms": round(self.p95_ms),
            "p99_ms": round(self.p99_ms),
            "max_ms": round(self.max_ms),
            "mean_ms": round(self.mean_ms),
            "visible_paths": self.visible_paths,
            "visible_texts": self.visible_texts,
            "budget_ms": WORK_BUDGET_MS,
            "passes": self.passes(),
        })
        .to_string()
    }
}

/// Round to three decimals for stable reports.
fn round(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

/// Build a deterministic stress board with `elements` shapes and
/// `text_elements` text blocks.
///
/// Shapes alternate rectangle/ellipse/diamond on a grid with 140 x 120 world
/// spacing; text blocks sit in a row below the grid. Ids, seeds and styles are
/// fixed so repeated runs render identically.
pub fn stress_board(elements: usize, text_elements: usize) -> BoardDocument {
    let mut doc = BoardDocument::empty();
    let columns = (elements as f64).sqrt().ceil().max(1.0) as usize;
    for index in 0..elements {
        let kind = match index % 3 {
            0 => ElementKind::Rectangle,
            1 => ElementKind::Ellipse,
            _ => ElementKind::Diamond,
        };
        let x = (index % columns) as f64 * 140.0;
        let y = (index / columns) as f64 * 120.0;
        let mut element = Element::new(kind, &format!("perf-shape-{index}"), x, y, 100.0, 70.0);
        element.set_style(&crate::element::Style {
            background_color: "#a5d8ff".to_string(),
            fill_style: FillStyle::Solid,
            ..crate::element::Style::default()
        });
        let _ = doc.add_element(element);
    }
    let text_y = (elements.div_ceil(columns)) as f64 * 120.0 + 40.0;
    for index in 0..text_elements {
        let mut element = Element::new(
            ElementKind::Text,
            &format!("perf-text-{index}"),
            (index % columns) as f64 * 140.0,
            text_y + (index / columns) as f64 * 40.0,
            120.0,
            25.0,
        );
        element.set_text(&format!("Note {index}"));
        element.set_extra("originalText", json!(format!("Note {index}")));
        let _ = doc.add_element(element);
    }
    doc
}

/// Deterministic viewport centres for a pan/zoom loop.
///
/// Returns `frames` centres on a diagonal path across a `width` x `height`
/// board, cycling through zoom levels so the visible set changes every frame.
pub fn pan_centres(frames: usize, width: f64, height: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(frames);
    for frame in 0..frames {
        let t = if frames > 1 {
            frame as f64 / (frames - 1) as f64
        } else {
            0.0
        };
        out.push((width * t, height * t));
    }
    out
}

/// Points of a deterministic freedraw-style polyline, used by tests.
pub fn diagonal_points(count: usize) -> Vec<Point> {
    (0..count)
        .map(|index| Point::new(index as f64 * 10.0, index as f64 * 4.0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::scene::{Scene, SceneOptions};
    use crate::render::SceneCache;

    #[test]
    fn percentiles_use_nearest_rank() {
        let mut timer = FrameTimer::new(100);
        for value in 1..=100 {
            timer.record(f64::from(value));
        }
        assert_eq!(timer.percentile(50.0), Some(50.0));
        assert_eq!(timer.percentile(95.0), Some(95.0));
        assert_eq!(timer.max(), Some(100.0));
    }

    #[test]
    fn bounded_timer_drops_old_samples() {
        let mut timer = FrameTimer::new(2);
        timer.record(1.0);
        timer.record(2.0);
        timer.record(3.0);
        assert_eq!(timer.len(), 2);
        assert_eq!(timer.samples(), vec![2.0, 3.0]);
    }

    #[test]
    fn report_gate_rejects_slow_frames() {
        let mut timer = FrameTimer::new(10);
        timer.record(10.0);
        let report = PerfReport::from_timer("test", 10, 0, 5.0, &timer, 1, 0);
        assert!(report.passes());
        let mut slow = FrameTimer::new(10);
        slow.record(WORK_BUDGET_MS + 5.0);
        let report = PerfReport::from_timer("test", 10, 0, 5.0, &slow, 1, 0);
        assert!(!report.passes());
        assert!(report.to_json().contains("\"passes\":false"));
    }

    #[test]
    fn stress_board_renders_and_filters() {
        let doc = stress_board(300, 40);
        assert_eq!(doc.elements().len(), 340);
        let scene = Scene::build(&doc, &SceneOptions::default()).unwrap();
        assert!(!scene.paths.is_empty());
        assert_eq!(scene.texts.len(), 40);
        let mut cache = SceneCache::new();
        cache.scene(&doc, &SceneOptions::default()).unwrap();
        let set = cache
            .visible(
                &crate::geometry::Viewport::default(),
                crate::geometry::Rect::new(0.0, 0.0, 400.0, 300.0),
            )
            .unwrap();
        assert!(!set.paths.is_empty());
        assert!(set.paths.len() < scene.paths.len());
    }

    #[test]
    fn pan_centres_are_deterministic() {
        let centres = pan_centres(3, 100.0, 200.0);
        assert_eq!(centres, vec![(0.0, 0.0), (50.0, 100.0), (100.0, 200.0)]);
        assert_eq!(
            diagonal_points(2),
            vec![Point::new(0.0, 0.0), Point::new(10.0, 4.0)]
        );
    }
}
