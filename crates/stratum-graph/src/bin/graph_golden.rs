//! Golden runner for the headless graph renderer.
//!
//! The runner generates deterministic fixture graphs, advances the ported
//! simulation, places the shipped `3d-force-graph` camera with
//! [`Camera::shipped_for_nodes`], renders a fixed number of frames through the
//! requested backend, writes PNG captures plus JSON/markdown reports, and
//! optionally enforces the frame-rate and memory gates. It is intentionally
//! dependency-free (no clap/serde) so the crate stays small and reproducible
//! offline.

use pkm_core::{PkmError, PkmResult};
use std::path::{Path, PathBuf};
use std::time::Instant;
use stratum_graph::{
    fixture, peak_rss_mb, Camera, FrameBucket, FrameTimer, RenderConfig, Renderer, Scene,
    Simulation, MIN_GOLDEN_FPS, WORK_BUDGET_MS,
};

/// The card memory gate for the 20k fixture.
const MEMORY_GATE_MB: f64 = 800.0;
/// Fixture size the memory gate applies to.
const MEMORY_GATE_FIXTURE: usize = 20_000;
/// Round-1.5 acceptance p95 budget for the 10k fixture (60 fps).
const BUDGET_10K_MS: f64 = 16.6;
/// Round-1.5 acceptance p95 budget for the 20k fixture (45 fps).
const BUDGET_20K_MS: f64 = 22.2;

/// Parsed command-line options.
struct Options {
    fixtures: Vec<usize>,
    seed: u64,
    ticks: usize,
    frames: usize,
    out: PathBuf,
    assert_fps: bool,
    software: bool,
    width: u32,
    height: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            fixtures: vec![200, 2000, 5000, 10_000, 20_000],
            seed: 1,
            ticks: 120,
            frames: 120,
            out: PathBuf::from(".hermes/e6"),
            assert_fps: false,
            software: false,
            width: 1280,
            height: 720,
        }
    }
}

/// A single fixture report.
struct FixtureReport {
    nodes: usize,
    edges: usize,
    tick_ms: f64,
    frame_count: usize,
    mean_ms: f64,
    p50_ms: f64,
    p95_ms: f64,
    max_ms: f64,
    fps: f64,
    peak_rss_mb: f64,
    hash: String,
    budget_ms: f64,
    passed: bool,
    histogram: Vec<FrameBucket>,
    png: String,
    pick: Option<usize>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("graph_golden failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> PkmResult<()> {
    let options = parse_args(std::env::args().skip(1))?;
    std::fs::create_dir_all(&options.out)
        .map_err(|error| PkmError::Render(format!("create output directory failed: {error}")))?;
    let config = RenderConfig {
        width: options.width,
        height: options.height,
        ..RenderConfig::default()
    };
    let mut renderer = if options.software {
        Renderer::software(config)?
    } else {
        Renderer::gpu(config)?
    };
    println!(
        "graph_golden backend={} adapter={} out={} fixtures={:?} ticks={} frames={}",
        renderer.backend(),
        renderer.adapter_name().unwrap_or("software"),
        options.out.display(),
        options.fixtures,
        options.ticks,
        options.frames
    );
    let mut reports = Vec::new();
    for count in &options.fixtures {
        let report = run_fixture(&mut renderer, &options, *count)?;
        println!(
            "fixture nodes={} edges={} tick_ms={:.2} frames={} mean_ms={:.2} p50_ms={:.2} p95_ms={:.2} max_ms={:.2} fps={:.2} peak_rss_mb={:.1} hash={} passed={} pick={:?} png={}",
            report.nodes,
            report.edges,
            report.tick_ms,
            report.frame_count,
            report.mean_ms,
            report.p50_ms,
            report.p95_ms,
            report.max_ms,
            report.fps,
            report.peak_rss_mb,
            report.hash,
            report.passed,
            report.pick,
            report.png
        );
        if options.assert_fps && report.fps < MIN_GOLDEN_FPS {
            return Err(PkmError::Render(format!(
                "fixture {} rendered at {:.2} fps, below the {:.2} fps gate",
                report.nodes, report.fps, MIN_GOLDEN_FPS
            )));
        }
        if options.assert_fps
            && report.nodes >= MEMORY_GATE_FIXTURE
            && report.peak_rss_mb > MEMORY_GATE_MB
        {
            return Err(PkmError::Render(format!(
                "fixture {} peak RSS {:.1} MB exceeds the {:.1} MB gate",
                report.nodes, report.peak_rss_mb, MEMORY_GATE_MB
            )));
        }
        reports.push(report);
    }
    let report_path = options.out.join("report.json");
    write_report(
        &report_path,
        renderer.backend(),
        renderer.adapter_name(),
        &options,
        &reports,
    )?;
    let markdown_path = options.out.join("report.md");
    write_markdown_report(
        &markdown_path,
        renderer.backend(),
        renderer.adapter_name(),
        &options,
        &reports,
    )?;
    println!(
        "graph_golden report written to {} and {}",
        report_path.display(),
        markdown_path.display()
    );
    Ok(())
}

/// Acceptance budget for a fixture size.
fn round_budget_ms(nodes: usize) -> f64 {
    if nodes >= MEMORY_GATE_FIXTURE {
        BUDGET_20K_MS
    } else if nodes >= 10_000 {
        BUDGET_10K_MS
    } else {
        WORK_BUDGET_MS
    }
}

fn run_fixture(
    renderer: &mut Renderer,
    options: &Options,
    count: usize,
) -> PkmResult<FixtureReport> {
    let data = fixture(count, options.seed);
    let mut simulation = Simulation::new(&data, options.seed)?;
    simulation.set_parallel(true);
    let tick_start = Instant::now();
    simulation.advance(options.ticks);
    let tick_ms = tick_start.elapsed().as_secs_f64() * 1000.0;
    let positions = simulation.positions();
    let config = RenderConfig {
        width: options.width,
        height: options.height,
        ..RenderConfig::default()
    };
    let scene = Scene::build(&data, &positions, &config.scene_options())?;
    let camera = Camera::shipped_for_nodes(data.nodes.len());
    let mut timer = FrameTimer::new();
    let mut last_frame = None;
    for _ in 0..options.frames.max(1) {
        let start = Instant::now();
        let frame = renderer.render(&scene, &camera)?;
        timer.record(start.elapsed());
        last_frame = Some(frame);
    }
    let budget_ms = round_budget_ms(count);
    let report = timer.report(budget_ms);
    let png_path = options.out.join(format!("graph-{count}.png"));
    let frame = last_frame
        .ok_or_else(|| PkmError::Render(format!("fixture {count} produced no rendered frame")))?;
    frame.save_png(&png_path)?;
    let hash = frame.content_hash();
    let peak_rss_mb = peak_rss_mb();
    let pick = match scene.project_node(&camera, options.width, options.height, 0) {
        Some((x, y)) => renderer.pick(&scene, &camera, x, y)?,
        None => None,
    };
    Ok(FixtureReport {
        nodes: data.nodes.len(),
        edges: data.edges.len(),
        tick_ms,
        frame_count: report.frame_count,
        mean_ms: report.mean_ms,
        p50_ms: report.p50_ms,
        p95_ms: report.p95_ms,
        max_ms: report.max_ms,
        fps: report.fps,
        peak_rss_mb,
        hash,
        budget_ms,
        passed: report.passed,
        histogram: report.histogram,
        png: png_path.display().to_string(),
        pick,
    })
}

fn parse_args<I>(arguments: I) -> PkmResult<Options>
where
    I: IntoIterator<Item = String>,
{
    let mut options = Options::default();
    let mut iterator = arguments.into_iter();
    while let Some(argument) = iterator.next() {
        match argument.as_str() {
            "--fixtures" => {
                let value = next_value(&mut iterator, "--fixtures")?;
                options.fixtures = parse_list(&value)?;
            }
            "--seed" => {
                options.seed = parse_value(&next_value(&mut iterator, "--seed")?, "--seed")?;
            }
            "--ticks" => {
                options.ticks = parse_value(&next_value(&mut iterator, "--ticks")?, "--ticks")?;
            }
            "--frames" => {
                options.frames = parse_value(&next_value(&mut iterator, "--frames")?, "--frames")?;
            }
            "--out" => {
                options.out = PathBuf::from(next_value(&mut iterator, "--out")?);
            }
            "--width" => {
                options.width = parse_value(&next_value(&mut iterator, "--width")?, "--width")?;
            }
            "--height" => {
                options.height = parse_value(&next_value(&mut iterator, "--height")?, "--height")?;
            }
            "--assert-fps" => options.assert_fps = true,
            "--software" => options.software = true,
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                return Err(PkmError::Internal(format!(
                    "unknown graph_golden argument: {other}"
                )))
            }
        }
    }
    if options.fixtures.is_empty() {
        return Err(PkmError::Internal(
            "graph_golden needs at least one fixture size".to_string(),
        ));
    }
    Ok(options)
}

fn next_value<I>(iterator: &mut I, flag: &str) -> PkmResult<String>
where
    I: Iterator<Item = String>,
{
    iterator
        .next()
        .ok_or_else(|| PkmError::Internal(format!("missing value for {flag}")))
}

fn parse_value<T>(value: &str, flag: &str) -> PkmResult<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    value
        .parse()
        .map_err(|error| PkmError::Internal(format!("invalid {flag} value {value}: {error}")))
}

fn parse_list(value: &str) -> PkmResult<Vec<usize>> {
    let mut sizes = Vec::new();
    for item in value.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        sizes.push(parse_value(item, "--fixtures")?);
    }
    Ok(sizes)
}

fn print_help() {
    println!(
        "graph_golden [--fixtures 200,2000,5000] [--seed 1] [--ticks 120] [--frames 120]\n\
         [--out .hermes/e6] [--width 1280] [--height 720] [--assert-fps] [--software]"
    );
}

/// Whether every fixture met its round budget and the 20k memory gate.
fn verdict(reports: &[FixtureReport]) -> bool {
    reports.iter().all(|report| report.passed)
        && reports
            .iter()
            .filter(|report| report.nodes >= MEMORY_GATE_FIXTURE)
            .all(|report| report.peak_rss_mb <= MEMORY_GATE_MB)
}

fn write_report(
    path: &Path,
    backend: &str,
    adapter: Option<&str>,
    options: &Options,
    reports: &[FixtureReport],
) -> PkmResult<()> {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"backend\": \"{}\",\n", json_escape(backend)));
    json.push_str(&format!(
        "  \"adapter\": \"{}\",\n",
        json_escape(adapter.unwrap_or("software"))
    ));
    json.push_str(&format!(
        "  \"seed\": {},\n  \"ticks\": {},\n  \"frames\": {},\n  \"width\": {},\n  \"height\": {},\n",
        options.seed, options.ticks, options.frames, options.width, options.height
    ));
    json.push_str(&format!(
        "  \"work_budget_ms\": {:.2},\n  \"min_fps\": {:.2},\n",
        WORK_BUDGET_MS, MIN_GOLDEN_FPS
    ));
    json.push_str(&format!(
        "  \"memory_gate_mb\": {:.1},\n  \"verdict\": \"{}\",\n",
        MEMORY_GATE_MB,
        if verdict(reports) { "pass" } else { "fail" }
    ));
    json.push_str("  \"fixtures\": [\n");
    for (index, report) in reports.iter().enumerate() {
        json.push_str("    {\n");
        json.push_str(&format!("      \"nodes\": {},\n", report.nodes));
        json.push_str(&format!("      \"edges\": {},\n", report.edges));
        json.push_str(&format!("      \"tick_ms\": {:.3},\n", report.tick_ms));
        json.push_str(&format!("      \"frame_count\": {},\n", report.frame_count));
        json.push_str(&format!("      \"mean_ms\": {:.3},\n", report.mean_ms));
        json.push_str(&format!("      \"p50_ms\": {:.3},\n", report.p50_ms));
        json.push_str(&format!("      \"p95_ms\": {:.3},\n", report.p95_ms));
        json.push_str(&format!("      \"max_ms\": {:.3},\n", report.max_ms));
        json.push_str(&format!("      \"fps\": {:.3},\n", report.fps));
        json.push_str(&format!("      \"budget_ms\": {:.2},\n", report.budget_ms));
        json.push_str(&format!(
            "      \"peak_rss_mb\": {:.1},\n",
            report.peak_rss_mb
        ));
        json.push_str(&format!("      \"hash\": \"{}\",\n", report.hash));
        json.push_str("      \"histogram\": [");
        for (bucket_index, bucket) in report.histogram.iter().enumerate() {
            if bucket_index > 0 {
                json.push(',');
            }
            let upper = if bucket.upper_ms.is_finite() {
                format!("{:.2}", bucket.upper_ms)
            } else {
                "null".to_string()
            };
            json.push_str(&format!(
                "{{\"upper_ms\": {upper}, \"count\": {}}}",
                bucket.count
            ));
        }
        json.push_str("],\n");
        json.push_str(&format!("      \"passed\": {},\n", report.passed));
        json.push_str(&format!("      \"pick\": {},\n", option_json(report.pick)));
        json.push_str(&format!(
            "      \"png\": \"{}\"\n",
            json_escape(&report.png)
        ));
        if index + 1 == reports.len() {
            json.push_str("    }\n");
        } else {
            json.push_str("    },\n");
        }
    }
    json.push_str("  ]\n}\n");
    std::fs::write(path, json)
        .map_err(|error| PkmError::Render(format!("write golden report failed: {error}")))
}

fn write_markdown_report(
    path: &Path,
    backend: &str,
    adapter: Option<&str>,
    options: &Options,
    reports: &[FixtureReport],
) -> PkmResult<()> {
    let passed = verdict(reports);
    let mut markdown = String::new();
    markdown.push_str("# stratum-graph golden report\n\n");
    markdown.push_str(&format!("- backend: `{}`\n", backend));
    markdown.push_str(&format!("- adapter: `{}`\n", adapter.unwrap_or("software")));
    markdown.push_str(&format!(
        "- seed: `{}` / ticks: `{}` / frames: `{}` / viewport: `{}x{}`\n",
        options.seed, options.ticks, options.frames, options.width, options.height
    ));
    markdown.push_str(&format!(
        "- memory gate: `{:.1} MB` at `{}` nodes\n",
        MEMORY_GATE_MB, MEMORY_GATE_FIXTURE
    ));
    markdown.push_str(&format!(
        "- verdict: **{}**\n\n",
        if passed { "PASS" } else { "FAIL" }
    ));
    markdown.push_str(
        "| nodes | edges | tick ms | p50 frame ms | p95 frame ms | max frame ms | fps | peak RSS MB | golden frame hash | PNG |\n",
    );
    markdown.push_str("|---:|---:|---:|---:|---:|---:|---:|---:|---|---|\n");
    for report in reports {
        markdown.push_str(&format!(
            "| {} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.1} | `{}` | `{}` |\n",
            report.nodes,
            report.edges,
            report.tick_ms,
            report.p50_ms,
            report.p95_ms,
            report.max_ms,
            report.fps,
            report.peak_rss_mb,
            report.hash,
            report.png
        ));
    }
    for report in reports {
        markdown.push_str(&format!("\n### {} nodes frame histogram\n\n", report.nodes));
        markdown.push_str("| <= X ms | count |\n|---:|---:|\n");
        for bucket in &report.histogram {
            let upper = if bucket.upper_ms.is_finite() {
                format!("{:.2}", bucket.upper_ms)
            } else {
                "inf".to_string()
            };
            markdown.push_str(&format!("| <= {} ms | {} |\n", upper, bucket.count));
        }
    }
    std::fs::write(path, markdown)
        .map_err(|error| PkmError::Render(format!("write golden markdown failed: {error}")))
}

fn option_json(value: Option<usize>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_string(),
    }
}

fn json_escape(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            other => output.push(other),
        }
    }
    output
}
