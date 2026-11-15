//! 10 000-block editor performance capture.
//!
//! Opens the native editor on the deterministic 10 000-block fixture, sweeps
//! the virtual list with the scroll handle for `STRATUM_PERF_FRAMES` frames
//! (default 600), records frame-to-frame timings and the editor's row-build
//! durations, writes JSON to `STRATUM_PERF_OUT` and quits.
//!
//! The JSON keys are the contract consumed by
//! `scripts/capture-gpui-editor-perf.sh`; the script owns the 55 fps
//! (`render_p95_ms <= 18.18`) gate.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use gpui_kit::*;
use stratum_ui::panels::editor::EditorView;
use stratum_ui::provider::{FixtureProvider, ShellData};
use stratum_ui::{init, StratumAssets, ThemeConfig};

/// Frame and row-build timings collected by the driver.
#[derive(Default)]
struct PerfState {
    frames: Vec<f64>,
    last: Option<Instant>,
}

/// Record one frame, scroll the editor, and schedule the next frame.
fn step(
    window: &mut Window,
    cx: &mut App,
    editor: Entity<EditorView>,
    state: Arc<Mutex<PerfState>>,
    frame: usize,
    total: usize,
) {
    let now = Instant::now();
    if let Ok(mut state) = state.lock() {
        if let Some(last) = state.last {
            state.frames.push(last.elapsed().as_secs_f64() * 1000.0);
        }
        state.last = Some(now);
    }

    if frame >= total {
        finish(cx, &editor, &state);
        return;
    }

    let total_height = editor.read(cx).total_height();
    let max_scroll = (total_height - 720.0).max(0.0);
    let progress = if total > 1 {
        frame as f32 / (total - 1) as f32
    } else {
        0.0
    };
    let offset = max_scroll * progress;
    editor.update(cx, |view, cx| {
        view.set_scroll_offset(offset);
        cx.notify();
    });
    window.refresh();

    window.on_next_frame(move |window, cx| {
        step(window, cx, editor, state, frame + 1, total);
    });
}

/// Write the JSON report and quit the application.
fn finish(cx: &mut App, editor: &Entity<EditorView>, state: &Arc<Mutex<PerfState>>) {
    let frames = state
        .lock()
        .map(|state| state.frames.clone())
        .unwrap_or_default();
    let (rows, render, assets) = {
        let view = editor.read(cx);
        (
            view.row_count(),
            view.metrics().render_ms().to_vec(),
            view.metrics().assets_ms().to_vec(),
        )
    };
    let frame_report = percentiles(&frames);
    let render_report = percentiles(&render);
    let asset_report = percentiles(&assets);

    let out = std::env::var("STRATUM_PERF_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
            root.join(".hermes/perf/editor-10k-frames.json")
        });
    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let report = serde_json::json!({
        "frames": frames.len(),
        "p50_ms": frame_report.p50,
        "p95_ms": frame_report.p95,
        "p99_ms": frame_report.p99,
        "max_ms": frame_report.max,
        "fps_p50": frame_report.fps_p50,
        "fps_p95": frame_report.fps_p95,
        "render_p95_ms": render_report.p95,
        "render_p50_ms": render_report.p50,
        "asset_p95_ms": asset_report.p95,
        "blocks": rows,
        "rows_rendered": editor.read(cx).metrics().rows_rendered(),
    });
    let _ = std::fs::write(
        &out,
        serde_json::to_string_pretty(&report).unwrap_or_default(),
    );

    println!(
        "editor-perf: {} frames, blocks={}, rows_rendered={}",
        report["frames"], report["blocks"], report["rows_rendered"]
    );
    println!(
        "editor-perf: p50={:.2}ms ({:.1} fps), p95={:.2}ms ({:.1} fps), p99={:.2}ms, max={:.2}ms",
        frame_report.p50,
        frame_report.fps_p50,
        frame_report.p95,
        frame_report.fps_p95,
        frame_report.p99,
        frame_report.max
    );
    println!(
        "editor-perf: render_p95={:.2}ms (budget 18.18ms / 55 fps), asset_p95={:.2}ms (not gated)",
        render_report.p95, asset_report.p95
    );
    println!("editor-perf: wrote {}", out.display());
    cx.quit();
}

/// A small percentile summary.
#[derive(Default)]
struct Percentiles {
    p50: f64,
    p95: f64,
    p99: f64,
    max: f64,
    fps_p50: f64,
    fps_p95: f64,
}

fn percentiles(values: &[f64]) -> Percentiles {
    if values.is_empty() {
        return Percentiles::default();
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let at = |p: f64| {
        let index = ((sorted.len() - 1) as f64 * p).round() as usize;
        sorted[index.min(sorted.len() - 1)]
    };
    let p50 = at(0.50);
    let p95 = at(0.95);
    Percentiles {
        p50,
        p95,
        p99: at(0.99),
        max: *sorted.last().unwrap_or(&0.0),
        fps_p50: if p50 > 0.0 { 1000.0 / p50 } else { 0.0 },
        fps_p95: if p95 > 0.0 { 1000.0 / p95 } else { 0.0 },
    }
}

fn main() {
    let frames = std::env::var("STRATUM_PERF_FRAMES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(600);

    gpui_kit::application()
        .with_assets(StratumAssets::new())
        .run(move |cx| {
            gpui_kit::init(cx);
            init(cx, &ThemeConfig::default());
            let provider: Arc<dyn ShellData> =
                Arc::new(FixtureProvider::with_welcome_blocks(10_000));
            let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(1280.0), px(720.0)));
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Stratum editor perf".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            let state = Arc::new(Mutex::new(PerfState::default()));
            match gpui_kit::open_window(options, cx, move |window, cx| {
                let editor = cx.new(|cx| EditorView::new("pages/Welcome.md", provider, false, cx));
                let scheduled = editor.clone();
                let perf_state = Arc::clone(&state);
                window.on_next_frame(move |window, cx| {
                    step(window, cx, scheduled, perf_state, 0, frames);
                });
                editor
            }) {
                Ok(_) => log::info!("editor perf window opened"),
                Err(error) => {
                    eprintln!("editor-perf: failed to open window: {error}");
                    cx.quit();
                }
            }
            cx.activate(true);
        });
}
