//! Frame timing collection and golden-run performance gates.
//!
//! The golden runner records render timings and can assert that the mean frame
//! rate stays above [`MIN_GOLDEN_FPS`]. The p95 work budget mirrors the editor
//! perf harness so CI can reject a regression that makes a frame exceed a 55 fps
//! work budget.

use pkm_core::{PkmError, PkmResult};
use std::time::Duration;

/// The 55 fps work budget used by the editor perf gate.
pub const WORK_BUDGET_MS: f64 = 18.18;
/// Minimum mean frame rate accepted by the golden runner's `--assert-fps` gate.
pub const MIN_GOLDEN_FPS: f64 = 30.0;

/// A collection of frame timings in milliseconds.
#[derive(Debug, Clone, Default)]
pub struct FrameTimer {
    samples: Vec<f64>,
}

impl FrameTimer {
    /// Create an empty timer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a frame duration.
    pub fn record(&mut self, duration: Duration) {
        self.samples.push(duration.as_secs_f64() * 1000.0);
    }

    /// Record a frame duration in milliseconds.
    pub fn record_ms(&mut self, milliseconds: f64) {
        if milliseconds.is_finite() {
            self.samples.push(milliseconds.max(0.0));
        }
    }

    /// Number of recorded frames.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether no frames have been recorded.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Build a deterministic report from the recorded samples.
    pub fn report(&self, budget_ms: f64) -> PerfReport {
        let mut sorted = self.samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mean = if sorted.is_empty() {
            0.0
        } else {
            sorted.iter().sum::<f64>() / sorted.len() as f64
        };
        let p95 = if sorted.is_empty() {
            0.0
        } else {
            let index = ((sorted.len() as f64 - 1.0) * 0.95).round() as usize;
            sorted[index.min(sorted.len() - 1)]
        };
        PerfReport {
            frame_count: sorted.len(),
            mean_ms: mean,
            p95_ms: p95,
            fps: if mean > 0.0 { 1000.0 / mean } else { 0.0 },
            budget_ms,
            passed: p95 <= budget_ms,
        }
    }
}

/// A summarized frame-timing report.
#[derive(Debug, Clone, PartialEq)]
pub struct PerfReport {
    /// Number of frames measured.
    pub frame_count: usize,
    /// Mean frame time in milliseconds.
    pub mean_ms: f64,
    /// 95th percentile frame time in milliseconds.
    pub p95_ms: f64,
    /// Mean frames per second.
    pub fps: f64,
    /// Work budget used for the p95 gate.
    pub budget_ms: f64,
    /// Whether the p95 gate passed.
    pub passed: bool,
}

impl PerfReport {
    /// Assert that the mean frame rate meets `minimum_fps`.
    pub fn assert_fps(&self, minimum_fps: f64) -> PkmResult<()> {
        if self.fps >= minimum_fps {
            Ok(())
        } else {
            Err(PkmError::Render(format!(
                "graph render frame rate {:.2} fps is below the {:.2} fps gate",
                self.fps, minimum_fps
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_timer_reports_zero_and_fails_fps_gate() {
        let timer = FrameTimer::new();
        let report = timer.report(WORK_BUDGET_MS);
        assert_eq!(report.frame_count, 0);
        assert_eq!(report.fps, 0.0);
        assert!(report.assert_fps(MIN_GOLDEN_FPS).is_err());
    }

    #[test]
    fn p95_is_deterministic() {
        let mut timer = FrameTimer::new();
        for ms in [1.0, 2.0, 3.0, 4.0, 100.0] {
            timer.record_ms(ms);
        }
        let report = timer.report(WORK_BUDGET_MS);
        assert_eq!(report.frame_count, 5);
        assert_eq!(report.p95_ms, 100.0);
        assert!(!report.passed);
        assert!(report.assert_fps(1.0).is_ok());
    }

    #[test]
    fn fast_frames_pass_the_gates() {
        let mut timer = FrameTimer::new();
        for _ in 0..10 {
            timer.record_ms(10.0);
        }
        let report = timer.report(WORK_BUDGET_MS);
        assert!(report.passed);
        assert_eq!(report.fps, 100.0);
        assert!(report.assert_fps(MIN_GOLDEN_FPS).is_ok());
    }
}
