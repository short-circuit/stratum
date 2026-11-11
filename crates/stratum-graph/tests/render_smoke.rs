//! Render smoke tests.
//!
//! The GPU test is skipped when no adapter is available. When an adapter exists
//! it checks that the frame contains visible geometry, that picking a projected
//! node centre returns that node, and that identical input renders byte-for-byte
//! identically. The software fallback runs unconditionally.

use stratum_graph::{fixture, Camera, RenderConfig, Renderer, Scene, SceneOptions, Vec3};

fn smoke_scene() -> Scene {
    let data = fixture(200, 1);
    let mut positions = Vec::with_capacity(data.nodes.len());
    for index in 0..data.nodes.len() {
        if index == 0 {
            positions.push(Vec3::ZERO);
        } else {
            let column = (index % 20) as f32;
            let row = (index / 20) as f32;
            positions.push(Vec3::new(-120.0 + column * 12.0, 50.0 - row * 10.0, 0.0));
        }
    }
    Scene::build(&data, &positions, &SceneOptions::default()).expect("scene")
}

fn non_background_pixels(frame: &stratum_graph::Frame, background: [u8; 4]) -> usize {
    frame
        .rgba()
        .chunks_exact(4)
        .filter(|pixel| {
            pixel[0] != background[0] || pixel[1] != background[1] || pixel[2] != background[2]
        })
        .count()
}

fn run_smoke(mut renderer: Renderer, config: RenderConfig, label: &str) {
    let scene = smoke_scene();
    let camera = Camera::default();
    let first = renderer.render(&scene, &camera).expect("first render");
    let total = (config.width * config.height) as usize;
    let visible = non_background_pixels(&first, scene.background);
    assert!(
        visible * 100 > total,
        "{label} render has only {visible} non-background pixels of {total}"
    );
    let (x, y) = scene
        .project_node(&camera, config.width, config.height, 0)
        .expect("node 0 is visible");
    assert_eq!(
        renderer.pick(&scene, &camera, x, y).expect("pick"),
        Some(0),
        "{label} pick at projected node 0 centre"
    );
    let second = renderer.render(&scene, &camera).expect("second render");
    assert_eq!(
        first.rgba(),
        second.rgba(),
        "{label} renders must be byte-identical"
    );
}

#[test]
fn gpu_render_smoke() {
    let config = RenderConfig {
        width: 640,
        height: 360,
        ..RenderConfig::default()
    };
    let renderer = match Renderer::gpu(config) {
        Ok(renderer) => renderer,
        Err(error) => {
            eprintln!("skipping GPU render smoke test: {error}");
            return;
        }
    };
    run_smoke(renderer, config, "gpu");
}

#[test]
fn software_render_smoke() {
    let config = RenderConfig {
        width: 320,
        height: 180,
        ..RenderConfig::default()
    };
    let renderer = Renderer::software(config).expect("software renderer");
    run_smoke(renderer, config, "software");
}
