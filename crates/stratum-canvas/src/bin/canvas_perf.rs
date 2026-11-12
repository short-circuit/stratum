//! Canvas performance harness.
//!
//! Runs the two acceptance scenarios from the whiteboard plan:
//!
//! * a 5 000-element board panned/zoomed for 600 frames, gated on the p95
//!   frame time (55 fps work budget);
//! * a 500-element text board, reporting scene/layout build time and a short
//!   pan loop.
//!
//! Usage:
//!
//! ```text
//! canvas_perf [--elements N] [--frames N] [--text N] [--json PATH]
//! ```
//!
//! The report is printed as one JSON object per line and, when `--json` is
//! given, written to that path. The process exits non-zero when a gated
//! scenario fails.

use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use stratum_canvas::geometry::{Rect, Viewport};
use stratum_canvas::render::perf::{
    pan_centres, stress_board, FrameTimer, PerfReport, DEFAULT_ELEMENTS, DEFAULT_FRAMES,
    DEFAULT_TEXT,
};
use stratum_canvas::render::{Scene, SceneCache, SceneOptions};

/// Parsed command line.
struct Args {
    elements: usize,
    frames: usize,
    text: usize,
    json: Option<String>,
}

/// Screen size used for the pan loop (1920x1080 desktop canvas).
const SCREEN_W: f64 = 1920.0;
const SCREEN_H: f64 = 1080.0;

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("canvas_perf: {error}");
            return ExitCode::FAILURE;
        }
    };
    let reports = vec![run_shapes(&args), run_text(&args)];
    let mut failed = false;
    for report in &reports {
        let line = report.to_json();
        println!("{line}");
        if !report.passes() {
            failed = true;
        }
    }
    if let Some(path) = &args.json {
        let body = reports
            .iter()
            .map(PerfReport::to_json)
            .collect::<Vec<_>>()
            .join("\n");
        if let Err(error) = fs::write(path, format!("{body}\n")) {
            eprintln!("canvas_perf: write {path}: {error}");
            return ExitCode::FAILURE;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Parse `--flag value` pairs.
fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut parsed = Args {
        elements: DEFAULT_ELEMENTS,
        frames: DEFAULT_FRAMES,
        text: DEFAULT_TEXT,
        json: None,
    };
    while let Some(flag) = args.next() {
        let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--elements" => parsed.elements = parse_count(&flag, &value)?,
            "--frames" => parsed.frames = parse_count(&flag, &value)?,
            "--text" => parsed.text = parse_count(&flag, &value)?,
            "--json" => parsed.json = Some(value),
            other => return Err(format!("unknown flag {other}")),
        }
    }
    Ok(parsed)
}

/// Parse a non-negative count.
fn parse_count(flag: &str, value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("{flag}: {error}"))
}

/// Run the 5k shape scenario.
fn run_shapes(args: &Args) -> PerfReport {
    let doc = stress_board(args.elements, 0);
    let options = SceneOptions::default();
    let build_start = Instant::now();
    let scene = match Scene::build(&doc, &options) {
        Ok(scene) => scene,
        Err(error) => {
            eprintln!("canvas_perf: scene build failed: {error}");
            return PerfReport::from_timer(
                "shapes",
                args.elements,
                0,
                0.0,
                &FrameTimer::new(1),
                0,
                0,
            );
        }
    };
    let build_ms = build_start.elapsed().as_secs_f64() * 1000.0;
    let bounds = scene
        .bounds
        .unwrap_or_else(|| Rect::new(0.0, 0.0, 100.0, 100.0));
    let mut cache = SceneCache::new();
    if let Err(error) = cache.scene(&doc, &options) {
        eprintln!("canvas_perf: cache build failed: {error}");
        return PerfReport::from_timer(
            "shapes",
            args.elements,
            0,
            build_ms,
            &FrameTimer::new(1),
            0,
            0,
        );
    }
    let mut timer = FrameTimer::new(args.frames.max(1));
    let centres = pan_centres(args.frames, bounds.w, bounds.h);
    let mut visible_paths = 0;
    for (frame, (cx, cy)) in centres.iter().enumerate() {
        let zoom = 1.0 + (frame % 4) as f64 * 0.25;
        let viewport = Viewport {
            x: SCREEN_W / 2.0 - cx * zoom,
            y: SCREEN_H / 2.0 - cy * zoom,
            zoom,
        };
        let start = Instant::now();
        match cache.visible(&viewport, Rect::new(0.0, 0.0, SCREEN_W, SCREEN_H)) {
            Ok(set) => visible_paths = set.paths.len(),
            Err(error) => {
                eprintln!("canvas_perf: visible failed: {error}");
                break;
            }
        }
        timer.record(start.elapsed().as_secs_f64() * 1000.0);
    }
    PerfReport::from_timer(
        "shapes",
        args.elements,
        0,
        build_ms,
        &timer,
        visible_paths,
        0,
    )
}

/// Run the 500-element text scenario.
fn run_text(args: &Args) -> PerfReport {
    let doc = stress_board(0, args.text);
    let options = SceneOptions::default();
    let build_start = Instant::now();
    let scene = match Scene::build(&doc, &options) {
        Ok(scene) => scene,
        Err(error) => {
            eprintln!("canvas_perf: text scene build failed: {error}");
            return PerfReport::from_timer("text", 0, args.text, 0.0, &FrameTimer::new(1), 0, 0);
        }
    };
    let build_ms = build_start.elapsed().as_secs_f64() * 1000.0;
    let bounds = scene
        .bounds
        .unwrap_or_else(|| Rect::new(0.0, 0.0, 100.0, 100.0));
    let mut cache = SceneCache::new();
    if let Err(error) = cache.scene(&doc, &options) {
        eprintln!("canvas_perf: text cache build failed: {error}");
        return PerfReport::from_timer("text", 0, args.text, build_ms, &FrameTimer::new(1), 0, 0);
    }
    let frames = args.frames.min(120);
    let mut timer = FrameTimer::new(frames.max(1));
    let centres = pan_centres(frames, bounds.w, bounds.h);
    let mut visible_texts = 0;
    for (cx, cy) in &centres {
        let viewport = Viewport {
            x: SCREEN_W / 2.0 - cx,
            y: SCREEN_H / 2.0 - cy,
            zoom: 1.0,
        };
        let start = Instant::now();
        match cache.visible(&viewport, Rect::new(0.0, 0.0, SCREEN_W, SCREEN_H)) {
            Ok(set) => visible_texts = set.texts.len(),
            Err(error) => {
                eprintln!("canvas_perf: text visible failed: {error}");
                break;
            }
        }
        timer.record(start.elapsed().as_secs_f64() * 1000.0);
    }
    PerfReport::from_timer("text", 0, args.text, build_ms, &timer, 0, visible_texts)
}
