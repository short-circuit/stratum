//! Performance harness tests: timer math, the perf gate, stress-board scene
//! builds and the 5k/500-element acceptance scenarios.
//!
//! The 5 000-element pan loop runs a reduced frame count so the suite stays
//! fast; `canvas_perf` runs the full 600-frame scenario and writes the report.

use std::time::Instant;

use stratum_canvas::geometry::{Rect, Viewport};
use stratum_canvas::render::perf::{
    pan_centres, stress_board, FrameTimer, PerfReport, WORK_BUDGET_MS,
};
use stratum_canvas::render::{Scene, SceneCache, SceneOptions};

#[test]
fn timer_percentiles_and_bounds() {
    let mut timer = FrameTimer::new(1000);
    for value in 1..=1000 {
        timer.record(f64::from(value));
    }
    assert_eq!(timer.len(), 1000);
    assert_eq!(timer.percentile(50.0), Some(500.0));
    assert_eq!(timer.percentile(95.0), Some(950.0));
    assert_eq!(timer.max(), Some(1000.0));
    assert_eq!(timer.mean(), Some(500.5));
    timer.reset();
    assert!(timer.is_empty());
    assert_eq!(timer.percentile(95.0), None);
}

#[test]
fn perf_gate_uses_the_work_budget() {
    let mut fast = FrameTimer::new(10);
    fast.record(WORK_BUDGET_MS - 1.0);
    let report = PerfReport::from_timer("shapes", 5000, 0, 100.0, &fast, 10, 0);
    assert!(report.passes());
    let mut slow = FrameTimer::new(10);
    slow.record(WORK_BUDGET_MS + 1.0);
    let report = PerfReport::from_timer("shapes", 5000, 0, 100.0, &slow, 10, 0);
    assert!(!report.passes());
    assert!(report.to_json().contains("\"budget_ms\":18.18"));
}

#[test]
fn five_thousand_elements_build_and_pan_within_budget() {
    let doc = stress_board(5000, 0);
    assert_eq!(doc.elements().len(), 5000);
    let options = SceneOptions::default();
    let build_start = Instant::now();
    let scene = Scene::build(&doc, &options).unwrap();
    let build_ms = build_start.elapsed().as_secs_f64() * 1000.0;
    assert!(!scene.paths.is_empty());
    let bounds = scene.bounds.unwrap();
    let mut cache = SceneCache::new();
    cache.scene(&doc, &options).unwrap();
    let mut timer = FrameTimer::new(120);
    let centres = pan_centres(120, bounds.w, bounds.h);
    let mut last_visible = 0;
    for (frame, (cx, cy)) in centres.iter().enumerate() {
        let zoom = 1.0 + (frame % 4) as f64 * 0.25;
        let viewport = Viewport {
            x: 960.0 - cx * zoom,
            y: 540.0 - cy * zoom,
            zoom,
        };
        let start = Instant::now();
        let set = cache
            .visible(&viewport, Rect::new(0.0, 0.0, 1920.0, 1080.0))
            .unwrap();
        timer.record(start.elapsed().as_secs_f64() * 1000.0);
        last_visible = set.paths.len();
    }
    let report = PerfReport::from_timer("shapes", 5000, 0, build_ms, &timer, last_visible, 0);
    assert!(last_visible > 0);
    assert!(
        report.passes(),
        "p95 frame time {} ms exceeded {} ms (build {:.1} ms)",
        report.p95_ms,
        WORK_BUDGET_MS,
        report.build_ms
    );
}

#[test]
fn five_hundred_text_elements_build_without_layout_stalls() {
    let doc = stress_board(0, 500);
    assert_eq!(doc.elements().len(), 500);
    let options = SceneOptions::default();
    let start = Instant::now();
    let scene = Scene::build(&doc, &options).unwrap();
    let build_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(scene.texts.len(), 500);
    // Generous ceiling: catches accidental O(n^2) layout, not machine speed.
    assert!(build_ms < 2000.0, "text build took {build_ms:.1} ms");
    let mut cache = SceneCache::new();
    cache.scene(&doc, &options).unwrap();
    let set = cache
        .visible(&Viewport::default(), Rect::new(0.0, 0.0, 400.0, 300.0))
        .unwrap();
    assert!(!set.texts.is_empty());
}

#[test]
fn pan_centres_are_deterministic() {
    assert_eq!(
        pan_centres(3, 100.0, 200.0),
        vec![(0.0, 0.0), (50.0, 100.0), (100.0, 200.0)]
    );
    assert!(pan_centres(0, 10.0, 10.0).is_empty());
}
